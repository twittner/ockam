pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/ockam.api.nodes.rs"));
}

use bytes::BufMut;
use core::fmt;
use ockam_api::{ErrorBody, Method, Request, Response, Status};
use ockam_core::compat::collections::HashMap;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{self, Routed, Worker};
use ockam_node::Context;

pub use proto::{CreateNode, NodeInfo, NodeInfoList};

#[derive(Debug, Default)]
pub struct Nodes(HashMap<String, proto::NodeInfo>);

#[ockam_core::worker]
impl Worker for Nodes {
    type Context = Context;
    type Message = Vec<u8>;

    async fn handle_message(
        &mut self,
        ctx: &mut Context,
        msg: Routed<Self::Message>,
    ) -> ockam_core::Result<()> {
        let mut buf = Vec::new();
        self.on_request(msg.as_body(), &mut buf)
            .await
            .map_err(|e| ockam_core::Error::new(Origin::Application, Kind::Invalid, e))?;
        ctx.send(msg.return_route(), buf).await
    }
}

impl Nodes {
    pub fn new() -> Self {
        Nodes::default()
    }

    async fn on_request<B>(&mut self, data: &[u8], mut response: B) -> Result<(), Error>
    where
        B: BufMut,
    {
        let req = Request::decode(data)?;

        match req.method() {
            Some(Method::Get) => match req.path_segments::<2>().as_slice() {
                // Get all nodes:
                [""] => Response::new(req.id(), Status::Ok)
                    .with_body(&proto::NodeInfoList {
                        nodes: self.0.values().cloned().collect::<Vec<_>>(),
                    })
                    .encode(&mut response)?,
                // Get a single node:
                [id] => {
                    if let Some(n) = self.0.get(*id) {
                        Response::new(req.id(), Status::Ok)
                            .with_body(n)
                            .encode(&mut response)?
                    } else {
                        Response::new(req.id(), Status::NotFound).encode(&mut response)?
                    }
                }
                _ => Response::new(req.id(), Status::BadRequest)
                    .with_body(
                        &ErrorBody::new(req.path())
                            .with_message("unknown path")
                            .finish(),
                    )
                    .encode(&mut response)?,
            },
            Some(Method::Post) => {
                let c: proto::CreateNode = req.decode_body()?;
                let n = proto::NodeInfo {
                    // TODO
                    id: "dsfsdfsdf".to_string(),
                    name: c.name,
                    status: "status".to_string(),
                    addr: b"/ip4/127.0.0.1/tcp/1234".to_vec(),
                };
                Response::new(req.id(), Status::Ok)
                    .with_body(&n)
                    .encode(&mut response)?;
                self.0.insert(n.id.clone(), n);
            }
            Some(_) => Response::new(req.id(), Status::MethodNotAllowed)
                .with_body(&ErrorBody::new(req.path()).finish())
                .encode(&mut response)?,
            None => Response::new(req.id(), Status::NotImplemented)
                .with_body(
                    &ErrorBody::new(req.path())
                        .with_message("method not implemented")
                        .finish(),
                )
                .encode(&mut response)?,
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Error(ErrorImpl);

#[derive(Debug)]
enum ErrorImpl {
    Api(ockam_api::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::Api(e) => e.fmt(f),
        }
    }
}

impl From<ockam_api::Error> for Error {
    fn from(e: ockam_api::Error) -> Self {
        Error(ErrorImpl::Api(e))
    }
}

impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Self {
        Error(ErrorImpl::Api(e.into()))
    }
}

impl ockam_core::compat::error::Error for Error {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::Api(e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateNode, NodeInfo, NodeInfoList, Nodes};
    use ockam_api::{Request, Response, Status};
    use prost::Message;

    #[test]
    fn smoke() {
        let (mut ctx, mut executor) = ockam_node::start_node();
        executor
            .execute(async move {
                ctx.start_worker("nodes", Nodes::default()).await?;

                // create the first node
                let req = Request::post("/").with_body(&CreateNode {
                    name: "first".into(),
                });
                ctx.send("nodes", req.to_vec()).await?;

                // read the response, giving us the node identifier
                let vec = ctx.receive::<Vec<u8>>().await?.take();
                let res = Response::decode(&**vec)?;
                let node_id = match res.status() {
                    Some(Status::Ok) => {
                        let ni = NodeInfo::decode(res.body())?;
                        println!("{ni:?}");
                        ni.id
                    }
                    other => return Err(format!("unexpected status code {other:?}").into()),
                };

                // get the node info for the identifier received
                let req = Request::get(format!("/{node_id}"));
                ctx.send("nodes", req.to_vec()).await?;
                let vec = ctx.receive::<Vec<u8>>().await?;
                let res = Response::decode(&**vec)?;
                match res.status() {
                    Some(Status::Ok) => {
                        let ni = NodeInfo::decode(res.body())?;
                        println!("{ni:?}");
                    }
                    other => return Err(format!("unexpected status code {other:?}").into()),
                }

                // get all nodes
                let req = Request::get("/");
                ctx.send("nodes", req.to_vec()).await?;

                let vec = ctx.receive::<Vec<u8>>().await?.take();
                let res = Response::decode(&**vec)?;
                match res.status() {
                    Some(Status::Ok) => {
                        let list = NodeInfoList::decode(res.body())?;
                        println!("{:?}", list.nodes);
                    }
                    other => return Err(format!("unexpected status code {other:?}").into()),
                }

                ctx.stop().await?;
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
            })
            .unwrap()
            .unwrap();
    }
}
