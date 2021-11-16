//! Ockam pipe module

mod internal;

mod listener;
pub use listener::PipeListener;

mod receiver;
pub use receiver::PipeReceiver;

mod sender;
pub use sender::PipeSender;

use ockam_core::{Address, Result, Route};
use ockam_node::Context;

const CLUSTER_NAME: &str = "_internal.pipe";

/// Connect to the receiving end of a pipe
///
/// Returns the PipeSender's public address.
pub async fn connect_static<R: Into<Route>>(ctx: &mut Context, recv: R) -> Result<Address> {
    let addr = Address::random(0);
    PipeSender::create(ctx, recv.into(), addr.clone(), Address::random(0))
        .await
        .map(|_| addr)
}

/// Connect to the pipe receive listener and then to a pipe receiver
pub async fn connect_dynamic(_listener: Route) -> PipeSender {
    todo!()
}

/// Create a receiver with a static address
pub async fn receiver<I: Into<Address>>(ctx: &mut Context, addr: I) -> Result<()> {
    PipeReceiver::create(ctx, addr.into()).await
}

/// Create a pipe receive listener
///
/// This special worker will create pipe receivers for any incoming
/// connection.  The worker can simply be stopped via its address.
pub async fn listen_for_connections(ctx: &mut Context) -> Result<Address> {
    let addr = Address::random(0);
    PipeListener::create(ctx, addr.clone()).await.map(|_| addr)
}

