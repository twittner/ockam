use crate::{TcpRecvProcessor, TcpRouterHandle};
use core::time::Duration;
use ockam_core::{async_trait, route, Any, Decodable, LocalMessage};
use ockam_core::{Address, Encodable, Message, Result, Routed, TransportMessage, Worker};
use ockam_node::{Context, DelayedEvent};
use ockam_transport_core::TransportError;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tracing::{debug, trace, warn};

/// Transmit and receive peers of a TCP connection
#[derive(Debug)]
pub(crate) struct WorkerPair {
    hostnames: Vec<String>,
    peer: SocketAddr,
    tx_addr: Address,
}

impl WorkerPair {
    pub fn hostnames(&self) -> &[String] {
        &self.hostnames
    }
    pub fn peer(&self) -> SocketAddr {
        self.peer
    }
    pub fn tx_addr(&self) -> Address {
        self.tx_addr.clone()
    }
}

#[derive(Serialize, Deserialize, Message, Clone)]
pub(crate) enum TcpSendWorkerMsg {
    Heartbeat,
    ConnectionClosed,
}

/// A TCP sending message worker
///
/// Create this worker type by calling
/// [`start_tcp_worker`](crate::start_tcp_worker)!
///
/// This half of the worker is created when spawning a new connection
/// worker pair, and listens for messages from the node message system
/// to dispatch to a remote peer.
pub(crate) struct TcpSendWorker {
    router_handle: TcpRouterHandle,
    rx: Option<OwnedReadHalf>,
    tx: Option<OwnedWriteHalf>,
    peer: SocketAddr,
    internal_addr: Address,
    rx_addr: Option<Address>,
    heartbeat: DelayedEvent<TcpSendWorkerMsg>,
    heartbeat_interval: Option<Duration>,
}

impl TcpSendWorker {
    fn new(
        router_handle: TcpRouterHandle,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        internal_addr: Address,
        heartbeat: DelayedEvent<TcpSendWorkerMsg>,
    ) -> Self {
        let (rx, tx) = match stream {
            Some(s) => {
                let (rx, tx) = s.into_split();
                (Some(rx), Some(tx))
            }
            None => (None, None),
        };

        Self {
            router_handle,
            rx,
            tx,
            peer,
            internal_addr,
            rx_addr: None,
            heartbeat,
            heartbeat_interval: Some(Duration::from_secs(5 * 60)),
        }
    }

    pub(crate) async fn start_pair(
        ctx: &Context,
        router_handle: TcpRouterHandle,
        stream: Option<TcpStream>,
        peer: SocketAddr,
        hostnames: Vec<String>,
    ) -> Result<WorkerPair> {
        trace!("Creating new TCP worker pair");

        let tx_addr = Address::random_local();
        let internal_addr = Address::random_local();
        let sender = TcpSendWorker::new(
            router_handle,
            stream,
            peer,
            internal_addr.clone(),
            DelayedEvent::create(ctx, internal_addr.clone(), TcpSendWorkerMsg::Heartbeat).await?,
        );

        ctx.start_worker(vec![tx_addr.clone(), internal_addr], sender)
            .await?;

        // Return a handle to the worker pair
        Ok(WorkerPair {
            hostnames,
            peer,
            tx_addr,
        })
    }

    async fn schedule_heartbeat(&mut self) -> Result<()> {
        let heartbeat_interval;
        if let Some(hi) = &self.heartbeat_interval {
            heartbeat_interval = *hi;
        } else {
            return Ok(());
        }

        self.heartbeat.schedule(heartbeat_interval).await
    }

    async fn stop_and_unregister(&self, ctx: &Context) -> Result<()> {
        self.router_handle.unregister(ctx.address()).await?;

        ctx.stop_worker(ctx.address()).await?;

        Ok(())
    }
}

fn prepare_message(msg: TransportMessage) -> Result<Vec<u8>> {
    let mut msg_buf = msg.encode().map_err(|_| TransportError::SendBadMessage)?;

    // Create a buffer that includes the message length in big endian
    let mut len = (msg_buf.len() as u16).to_be_bytes().to_vec();

    // Fun fact: reversing a vector in place, appending the length,
    // and then reversing it again is faster for large message sizes
    // than adding the large chunk of data.
    //
    // https://play.rust-lang.org/?version=stable&mode=release&edition=2018&gist=8669a640004ac85c7be38b19e3e73dcb
    msg_buf.reverse();
    len.reverse();
    msg_buf.append(&mut len);
    msg_buf.reverse();

    Ok(msg_buf)
}

#[async_trait]
impl Worker for TcpSendWorker {
    type Context = Context;
    type Message = Any;

    async fn initialize(&mut self, ctx: &mut Self::Context) -> Result<()> {
        ctx.set_cluster(crate::CLUSTER_NAME).await?;

        if self.tx.is_none() {
            let connection = match TcpStream::connect(self.peer).await {
                Ok(c) => c,
                Err(e) => {
                    self.stop_and_unregister(ctx).await?;

                    return Err(TransportError::from(e).into());
                }
            };
            let (rx, tx) = connection.into_split();
            self.tx = Some(tx);
            self.rx = Some(rx);
        }

        let rx = self.rx.take().ok_or(TransportError::GenericIo)?;

        let rx_addr = Address::random_local();
        let receiver = TcpRecvProcessor::new(
            rx,
            Address::new(crate::TCP, self.peer.to_string()),
            self.internal_addr.clone(),
        );
        ctx.start_processor(rx_addr.clone(), receiver).await?;

        self.rx_addr = Some(rx_addr);

        self.schedule_heartbeat().await?;

        Ok(())
    }

    async fn shutdown(&mut self, ctx: &mut Self::Context) -> Result<()> {
        if let Some(rx_addr) = self.rx_addr.take() {
            let _ = ctx.stop_processor(rx_addr).await;
        }

        Ok(())
    }

    // TcpSendWorker will receive messages from the TcpRouter to send
    // across the TcpStream to our friend
    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        self.heartbeat.cancel();

        let tx;
        if let Some(t) = &mut self.tx {
            tx = t;
        } else {
            return Err(TransportError::PeerNotFound.into());
        }

        let recipient = msg.msg_addr();
        if recipient == self.internal_addr {
            let msg = TcpSendWorkerMsg::decode(msg.payload())?;

            match msg {
                TcpSendWorkerMsg::Heartbeat => {
                    let msg = TransportMessage::v1(route![], route![], vec![]);
                    let msg = prepare_message(msg)?;
                    // Sending empty heartbeat
                    if tx.write_all(&msg).await.is_err() {
                        warn!("Failed to send heartbeat to peer {}", self.peer);
                        self.stop_and_unregister(ctx).await?;

                        return Ok(());
                    }

                    debug!("Sent heartbeat to peer {}", self.peer);
                }
                TcpSendWorkerMsg::ConnectionClosed => {
                    warn!("Stopping sender due to closed connection {}", self.peer);
                    // No need to stop Receiver as it notified us about connection drop and will
                    // stop itself
                    self.rx_addr = None;
                    self.stop_and_unregister(ctx).await?;

                    return Ok(());
                }
            }
        } else {
            let mut msg = LocalMessage::decode(msg.payload())?.into_transport_message();
            // Remove our own address from the route so the other end
            // knows what to do with the incoming message
            msg.onward_route.step()?;
            // Create a message buffer with pre-pended length
            let msg = prepare_message(msg)?;

            if tx.write_all(msg.as_slice()).await.is_err() {
                warn!("Failed to send message to peer {}", self.peer);
                self.stop_and_unregister(ctx).await?;

                return Ok(());
            }
        }

        self.schedule_heartbeat().await?;

        Ok(())
    }
}
