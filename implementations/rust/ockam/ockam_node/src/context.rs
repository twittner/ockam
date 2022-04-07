use crate::relay::RelayPayload;
use crate::tokio::{
    self,
    runtime::Runtime,
    sync::mpsc::{channel, Receiver, Sender},
    time::timeout,
};
use crate::{
    error::Error,
    parser,
    relay::{CtrlSignal, ProcessorRelay, RelayMessage, WorkerRelay},
    router::SenderPair,
    Cancel, NodeMessage, ShutdownType,
};
use crate::{NodeError, Reason};
use core::time::Duration;
use ockam_core::compat::{boxed::Box, string::String, sync::Arc, vec::Vec};
use ockam_core::{
    AccessControl, Address, AddressSet, AllowAll, AsyncTryClone, LocalMessage, Message, Processor,
    Result, Route, TransportMessage, TransportType, Worker,
};

/// A default timeout in seconds
pub const DEFAULT_TIMEOUT: u64 = 30;

enum AddressType {
    Worker,
    Processor,
}

impl AddressType {
    fn str(&self) -> &'static str {
        match self {
            AddressType::Worker => "worker",
            AddressType::Processor => "processor",
        }
    }
}

/// Context contains Node state and references to the runtime.
pub struct Context {
    address: AddressSet,
    sender: Sender<NodeMessage>,
    rt: Arc<Runtime>,
    mailbox: Receiver<RelayMessage>,
    access_control: Box<dyn AccessControl>,
}

#[ockam_core::async_trait]
impl AsyncTryClone for Context {
    async fn async_try_clone(&self) -> Result<Self> {
        self.new_context(Address::random_local()).await
    }
}

impl Context {
    /// Return runtime clone
    pub fn runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }
    /// Wait for the next message from the mailbox
    pub(crate) async fn mailbox_next(&mut self) -> Result<Option<RelayMessage>> {
        loop {
            let relay_msg;
            if let Some(msg) = self.mailbox.recv().await.map(|msg| {
                trace!("{:?}: received new message!", self.address());
                msg
            }) {
                relay_msg = msg;
            } else {
                return Ok(None);
            }

            if let RelayPayload::Direct(local_msg) = &relay_msg.data {
                if !self.access_control.is_authorized(local_msg).await? {
                    warn!(
                        "Message for {:?} did not pass access control",
                        relay_msg.addr
                    );
                    continue;
                }
            }

            return Ok(Some(relay_msg));
        }
    }
}

impl Context {
    /// Create a new context
    ///
    /// This function returns a new instance of Context, the relay
    /// sender pair, and relay control signal receiver.
    pub(crate) fn new(
        rt: Arc<Runtime>,
        sender: Sender<NodeMessage>,
        address: AddressSet,
        access_control: impl AccessControl,
    ) -> (Self, SenderPair, Receiver<CtrlSignal>) {
        let (mailbox_tx, mailbox) = channel(32);
        let (ctrl_tx, ctrl_rx) = channel(1);
        (
            Self {
                rt,
                sender,
                address,
                mailbox,
                access_control: Box::new(access_control),
            },
            SenderPair {
                msgs: mailbox_tx,
                ctrl: ctrl_tx,
            },
            ctrl_rx,
        )
    }

    /// Return the primary address of the current worker
    pub fn address(&self) -> Address {
        self.address.first()
    }

    /// Return all addresses of the current worker
    pub fn aliases(&self) -> AddressSet {
        self.address.clone().into_iter().skip(1).collect()
    }

    /// Utility function to sleep tasks from other crates
    #[doc(hidden)]
    pub async fn sleep(&self, dur: Duration) {
        tokio::time::sleep(dur).await;
    }

    /// Create a new context without spawning a full worker
    ///
    /// Note: this function is very low-level.  For most users
    /// [`start_worker()`](Self::start_worker) is the recommended to
    /// way to create a new worker context.
    pub async fn new_context<S>(&self, addr: S) -> Result<Context>
    where
        S: TryInto<Address>,
        S::Error: Into<ockam_core::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.new_context_impl(a).await
    }

    async fn new_context_impl(&self, addr: Address) -> Result<Context> {
        // Create a new context and get access to the mailbox senders
        let (ctx, sender, _) = Self::new(
            Arc::clone(&self.rt),
            self.sender.clone(),
            addr.clone().into(),
            AllowAll,
        );

        // Create a "bare relay" and register it with the router
        let (msg, mut rx) = NodeMessage::start_worker(addr.into(), sender, true);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::FailedStartWorker)?;

        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ctx)?)
    }

    /// Start a new worker instance at the given address set
    ///
    /// A worker is an asynchronous piece of code that can send and
    /// receive messages of a specific type.  This type is encoded via
    /// the [`Worker`](ockam_core::Worker) trait.  If your code relies
    /// on a manual run-loop you may want to use
    /// [`start_processor()`](Self::start_processor) instead!
    ///
    /// Each address in the set must be unique and unused on the
    /// current node.  Workers must implement the Worker trait and be
    /// thread-safe.  Workers run asynchronously and will be scheduled
    /// independently of each other.  To wait for the initialisation
    /// of your worker to complete you can use
    /// [`wait_for()`](Self::wait_for).
    ///
    /// ```rust
    /// use ockam_core::{Result, Worker, worker};
    /// use ockam_node::Context;
    ///
    /// struct MyWorker;
    ///
    /// #[worker]
    /// impl Worker for MyWorker {
    ///     type Context = Context;
    ///     type Message = String;
    /// }
    ///
    /// async fn start_my_worker(ctx: &mut Context) -> Result<()> {
    ///     ctx.start_worker("my-worker-address", MyWorker).await
    /// }
    /// ```
    pub async fn start_worker<NM, NW, S>(&self, address: S, worker: NW) -> Result<()>
    where
        S: TryInto<AddressSet>,
        S::Error: Into<ockam_core::Error>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
    {
        let set = address.try_into().map_err(|e| e.into())?;
        self.start_worker_impl(set, worker, AllowAll).await
    }

    /// Start a new worker instance with explicit access controls
    // TODO: Worker builder?
    // TODO: how is this meant to be used?
    pub async fn start_worker_with_access_control<NM, NW, NA, S>(
        &self,
        address: S,
        worker: NW,
        access_control: NA,
    ) -> Result<()>
    where
        S: TryInto<AddressSet>,
        S::Error: Into<ockam_core::Error>,
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
        NA: AccessControl,
    {
        let set = address.try_into().map_err(|e| e.into())?;
        self.start_worker_impl(set, worker, access_control).await
    }

    async fn start_worker_impl<NM, NW, NA>(
        &self,
        address: AddressSet,
        worker: NW,
        access_control: NA,
    ) -> Result<()>
    where
        NM: Message + Send + 'static,
        NW: Worker<Context = Context, Message = NM>,
        NA: AccessControl,
    {
        // Pass it to the context
        let (ctx, sender, ctrl_rx) = Context::new(
            self.rt.clone(),
            self.sender.clone(),
            address.clone(),
            access_control,
        );

        // Then initialise the worker message relay
        WorkerRelay::<NW, NM>::init(self.rt.as_ref(), worker, ctx, ctrl_rx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_worker(address, sender, false);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::FailedStartWorker)?;

        // Wait for the actual return code
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Start a new processor instance at the given address set
    ///
    /// A processor is an asynchronous piece of code that runs a
    /// custom run loop, with access to a worker context to send and
    /// receive messages.  If your code is built around responding to
    /// message events, consider using
    /// [`start_worker()`](Self::start_processor) instead!
    pub async fn start_processor<P, A>(&self, address: A, processor: P) -> Result<()>
    where
        A: TryInto<Address>,
        A::Error: Into<ockam_core::Error>,
        P: Processor<Context = Context>,
    {
        let a = address.try_into().map_err(|e| e.into())?;
        self.start_processor_impl(a, processor).await
    }

    async fn start_processor_impl<P>(&self, address: Address, processor: P) -> Result<()>
    where
        P: Processor<Context = Context>,
    {
        let addr = address.clone();

        let (ctx, senders, ctrl_rx) =
            Context::new(self.rt.clone(), self.sender.clone(), addr.into(), AllowAll);

        // Initialise the processor relay with the ctrl receiver
        ProcessorRelay::<P>::init(self.rt.as_ref(), processor, ctx, ctrl_rx);

        // Send start request to router
        let (msg, mut rx) = NodeMessage::start_processor(address, senders);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::FailedStartProcessor)?;

        // Wait for the actual return code
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Shut down a local worker by its primary address
    pub async fn stop_worker<A>(&self, addr: A) -> Result<()>
    where
        A: TryInto<Address>,
        A::Error: Into<ockam_core::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.stop_address(a, AddressType::Worker).await
    }

    /// Shut down a local processor by its address
    pub async fn stop_processor<A>(&self, addr: A) -> Result<()>
    where
        A: TryInto<Address>,
        A::Error: Into<ockam_core::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.stop_address(a, AddressType::Processor).await
    }

    async fn stop_address(&self, addr: Address, t: AddressType) -> Result<()> {
        debug!("Shutting down {} {:?}", t.str(), addr);

        // Send the stop request
        let (req, mut rx) = match t {
            AddressType::Worker => NodeMessage::stop_worker(addr),
            AddressType::Processor => NodeMessage::stop_processor(addr),
        };
        self.sender.send(req).await.map_err(Error::from)?;

        // Then check that address was properly shut down
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Signal to the local runtime to shut down immediately
    ///
    /// **WARNING**: calling this function may result in data loss.
    /// It is recommended to use the much safer
    /// [`Context::stop`](Context::stop) function instead!
    pub async fn stop_now(&mut self) -> Result<()> {
        let tx = self.sender.clone();
        info!("Immediately shutting down all workers");
        let (msg, _) = NodeMessage::stop_node(ShutdownType::Immediate);

        match tx.send(msg).await {
            Ok(()) => Ok(()),
            Err(_e) => Err(Error::FailedStopNode.into()),
        }
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed.
    /// The default timeout for a safe shutdown is 1 second.  You can
    /// change this behaviour by calling
    /// [`Context::stop_timeout`](Context::stop_timeout) directly.
    pub async fn stop(&mut self) -> Result<()> {
        self.stop_timeout(1).await
    }

    /// Signal to the local runtime to shut down
    ///
    /// This call will hang until a safe shutdown has been completed
    /// or the desired timeout has been reached.
    pub async fn stop_timeout(&mut self, seconds: u8) -> Result<()> {
        let (req, mut rx) = NodeMessage::stop_node(ShutdownType::Graceful(seconds));
        self.sender.send(req).await.map_err(Error::from)?;

        // Wait until we get the all-clear
        Ok(rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)?
            .map(|_| ())?)
    }

    /// Send a message to another address associated with this worker
    ///
    /// This function is a simple wrapper around `Self::send()` which
    /// validates the address given to it and will reject invalid
    /// addresses.
    pub async fn send_to_self<A, M>(&self, from: A, addr: A, msg: M) -> Result<()>
    where
        A: TryInto<Address>,
        A::Error: Into<ockam_core::Error>,
        M: Message + Send + 'static,
    {
        let addr = addr.try_into().map_err(|e| e.into())?;
        if self.address.contains(&addr) {
            let from = from.try_into().map_err(|e| e.into())?;
            self.send_from_address(addr, msg, from).await
        } else {
            Err(NodeError::Rejected(Reason::InvalidAddress).into())
        }
    }

    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// ```rust
    /// # use {ockam_node::Context, ockam_core::Result};
    /// # async fn test(ctx: &mut Context) -> Result<()> {
    /// use ockam_core::Message;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Message, Serialize, Deserialize)]
    /// struct MyMessage(String);
    ///
    /// impl MyMessage {
    ///     fn new(s: &str) -> Self {
    ///         Self(s.into())
    ///     }
    /// }
    ///
    /// ctx.send("my-test-worker", MyMessage::new("Hello you there :)")).await?;
    /// Ok(())
    /// # }
    /// ```
    pub async fn send<R, M>(&self, route: R, msg: M) -> Result<()>
    where
        R: TryInto<Route>,
        R::Error: Into<ockam_core::Error>,
        M: Message + Send + 'static,
    {
        let r = route.try_into().map_err(|e| e.into())?;
        self.send_from_address(r, msg, self.address()).await
    }

    /// Send a message to an address or via a fully-qualified route
    ///
    /// Routes can be constructed from a set of [`Address`]es, or via
    /// the [`RouteBuilder`] type.  Routes can contain middleware
    /// router addresses, which will re-address messages that need to
    /// be handled by specific domain workers.
    ///
    /// [`Address`]: ockam_core::Address
    /// [`RouteBuilder`]: ockam_core::RouteBuilder
    ///
    /// This function additionally takes the sending address
    /// parameter, to specify which of a worker's (or processor's)
    /// addresses should be used.
    pub async fn send_from_address<R, M>(
        &self,
        route: R,
        msg: M,
        sending_address: Address,
    ) -> Result<()>
    where
        R: TryInto<Route>,
        R::Error: Into<ockam_core::Error>,
        M: Message + Send + 'static,
    {
        let r = route.try_into().map_err(|e| e.into())?;
        self.send_from_address_impl(r, msg, sending_address).await
    }

    async fn send_from_address_impl<M>(
        &self,
        route: Route,
        msg: M,
        sending_address: Address,
    ) -> Result<()>
    where
        M: Message + Send + 'static,
    {
        if !self.address.as_ref().contains(&sending_address) {
            return Err(Error::SenderAddressDoesNotExist.into());
        }

        let (reply_tx, mut reply_rx) = channel(1);
        let next = route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(Error::from)?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_sender()?;

        // Pack the payload into a TransportMessage
        let payload = msg.encode().unwrap();
        let mut transport_msg = TransportMessage::v1(route.clone(), Route::new(), payload);
        transport_msg.return_route.modify().append(sending_address);
        let local_msg = LocalMessage::new(transport_msg, Vec::new());

        // Pack transport message into relay message wrapper
        let msg = if needs_wrapping {
            RelayMessage::pre_router(addr, local_msg, route)
        } else {
            RelayMessage::direct(addr, local_msg, route)
        };

        // Send the packed user message with associated route
        sender.send(msg).await.map_err(Error::from)?;

        Ok(())
    }

    /// Forward a transport message to its next routing destination
    ///
    /// Similar to [`Context::send`], but taking a
    /// [`TransportMessage`], which contains the full destination
    /// route, and calculated return route for this hop.
    ///
    /// **Note:** you most likely want to use
    /// [`Context::send`] instead, unless you are writing an
    /// external router implementation for ockam node.
    ///
    /// [`Context::send`]: crate::Context::send
    /// [`TransportMessage`]: ockam_core::TransportMessage
    pub async fn forward(&self, local_msg: LocalMessage) -> Result<()> {
        // Resolve the sender for the next hop in the messages route
        let (reply_tx, mut reply_rx) = channel(1);
        let next = local_msg.transport().onward_route.next().unwrap(); // TODO: communicate bad routes
        let req = NodeMessage::SenderReq(next.clone(), reply_tx);

        // First resolve the next hop in the route
        self.sender.send(req).await.map_err(Error::from)?;
        let (addr, sender, needs_wrapping) = reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_sender()?;

        // Pack the transport message into a relay message
        let onward = local_msg.transport().onward_route.clone();
        // let msg = RelayMessage::direct(addr, data, onward);
        let msg = if needs_wrapping {
            RelayMessage::pre_router(addr, local_msg, onward)
        } else {
            RelayMessage::direct(addr, local_msg, onward)
        };
        sender.send(msg).await.map_err(Error::from)?;

        Ok(())
    }

    /// Block the current worker to wait for a typed message
    ///
    /// **Warning** this function will wait until its running ockam
    /// node is shut down.  A safer variant of this function is
    /// [`receive`](Self::receive) and
    /// [`receive_timeout`](Self::receive_timeout).
    pub async fn receive_block<M: Message>(&mut self) -> Result<Cancel<'_, M>> {
        let (msg, data, addr) = self.next_from_mailbox().await?;
        Ok(Cancel::new(msg, data, addr, self))
    }

    /// Block the current worker to wait for a typed message
    ///
    /// This function may return a `Err(FailedLoadData)` if the
    /// underlying worker was shut down, or `Err(Timeout)` if the call
    /// was waiting for longer than the `default timeout`.  Use
    /// [`receive_timeout`](Context::receive_timeout) to adjust the
    /// timeout period.
    ///
    /// Will return `None` if the corresponding worker has been
    /// stopped, or the underlying Node has shut down.
    pub async fn receive<M: Message>(&mut self) -> Result<Cancel<'_, M>> {
        self.receive_timeout(DEFAULT_TIMEOUT).await
    }

    /// Wait to receive a message up to a specified timeout
    ///
    /// See [`receive`](Self::receive) for more details.
    pub async fn receive_timeout<M: Message>(
        &mut self,
        timeout_secs: u64,
    ) -> Result<Cancel<'_, M>> {
        let (msg, data, addr) = timeout(Duration::from_secs(timeout_secs), async {
            self.next_from_mailbox().await
        })
        .await
        .map_err(Error::from)??;
        Ok(Cancel::new(msg, data, addr, self))
    }

    /// Block the current worker to wait for a message satisfying a conditional
    ///
    /// Will return `Err` if the corresponding worker has been
    /// stopped, or the underlying node has shut down.  This operation
    /// has a [default timeout](DEFAULT_TIMEOUT).
    ///
    /// Internally this function uses [`receive`](Self::receive), so
    /// is subject to the same timeout.
    pub async fn receive_match<M, F>(&mut self, check: F) -> Result<Cancel<'_, M>>
    where
        M: Message,
        F: Fn(&M) -> bool,
    {
        let (m, data, addr) = timeout(Duration::from_secs(DEFAULT_TIMEOUT), async {
            loop {
                match self.next_from_mailbox().await {
                    Ok((m, data, addr)) if check(&m) => break Ok((m, data, addr)),
                    Ok((_, data, _)) => {
                        // Requeue
                        self.forward(data).await?;
                    }
                    e => break e,
                }
            }
        })
        .await
        .map_err(Error::from)??;

        Ok(Cancel::new(m, data, addr, self))
    }

    /// Assign the current worker to a cluster
    ///
    /// A cluster is a set of workers that should be stopped together
    /// when the node is stopped or parts of the system are reloaded.
    /// **This is not to be confused with supervisors!**
    ///
    /// By adding your worker to a cluster you signal to the runtime
    /// that your worker may be depended on by other workers that
    /// should be stopped first.
    ///
    /// **Your cluster name MUST NOT start with `_internals.` or
    /// `ockam.`!**
    ///
    /// Clusters are de-allocated in reverse order of their
    /// initialisation when the node is stopped.
    pub async fn set_cluster<S: Into<String>>(&self, label: S) -> Result<()> {
        let (msg, mut rx) = NodeMessage::set_cluster(self.address(), label.into());
        self.sender.send(msg).await.map_err(Error::from)?;
        Ok(rx.recv().await.ok_or(Error::InternalIOFailure)??.is_ok()?)
    }

    /// Return a list of all available worker addresses on a node
    pub async fn list_workers(&self) -> Result<Vec<Address>> {
        let (msg, mut reply_rx) = NodeMessage::list_workers();

        self.sender.send(msg).await.map_err(Error::from)?;

        Ok(reply_rx
            .recv()
            .await
            .ok_or(Error::InternalIOFailure)??
            .take_workers()?)
    }

    /// Register a router for a specific address type
    pub async fn register<A>(&self, type_: TransportType, addr: A) -> Result<()>
    where
        A: TryInto<Address>,
        A::Error: Into<ockam_core::Error>,
    {
        let a = addr.try_into().map_err(|e| e.into())?;
        self.register_impl(type_, a).await
    }

    /// Send a shutdown acknowledgement to the router
    pub(crate) async fn send_stop_ack(&self) -> Result<()> {
        self.sender
            .send(NodeMessage::StopAck(self.address()))
            .await
            .map_err(Error::from)?;
        Ok(())
    }

    async fn register_impl(&self, type_: TransportType, addr: Address) -> Result<()> {
        let (tx, mut rx) = channel(1);
        self.sender
            .send(NodeMessage::Router(type_, addr, tx))
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        Ok(rx.recv().await.ok_or(Error::InternalIOFailure)??.is_ok()?)
    }

    /// A convenience function to get a data 3-tuple from the mailbox
    ///
    /// The reason this function doesn't construct a `Cancel<_, M>` is
    /// to avoid the lifetime collision between the mutation on `self`
    /// and the ref to `Context` passed to `Cancel::new(..)`
    ///
    /// This function will block and re-queue messages into the
    /// mailbox until it can receive the correct message payload.
    ///
    /// WARNING: this will temporarily create a busyloop, this
    /// mechanism should be replaced with a waker system that lets the
    /// mailbox work not yield another message until the relay worker
    /// has woken it.
    async fn next_from_mailbox<M: Message>(&mut self) -> Result<(M, LocalMessage, Address)> {
        loop {
            let msg = self.mailbox_next().await?.ok_or(Error::FailedLoadData)?;
            let (addr, data) = msg.local_msg();

            // FIXME: make message parsing idempotent to avoid cloning
            match parser::message(&data.transport().payload).ok() {
                Some(msg) => break Ok((msg, data, addr)),
                None => {
                    // Requeue
                    self.forward(data).await?;
                }
            }
        }
    }

    /// Set access control for current context
    pub async fn set_access_control(&mut self) -> Result<()> {
        unimplemented!()
    }

    /// This function is called by Relay to indicate a worker is initialised
    pub(crate) async fn set_ready(&mut self) -> Result<()> {
        self.sender
            .send(NodeMessage::set_ready(self.address()))
            .await
            .map_err(|_| Error::InternalIOFailure)?;
        Ok(())
    }

    /// Wait for a particular address to become "ready"
    pub async fn wait_for<A>(&mut self, addr: A) -> Result<()>
    where
        A: TryInto<Address>,
        A::Error: Into<ockam_core::Error>,
    {
        let addr = addr.try_into().map_err(|e| e.into())?;
        let (msg, mut reply) = NodeMessage::get_ready(addr);
        self.sender
            .send(msg)
            .await
            .map_err(|_| Error::InternalIOFailure)?;

        // This call blocks until the address has become ready or is
        // dropped
        reply.recv().await.ok_or(Error::InternalIOFailure)??;
        Ok(())
    }
}
