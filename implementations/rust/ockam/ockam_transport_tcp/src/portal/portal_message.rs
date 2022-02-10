use minicbor::{Encode, Decode};
use ockam_core::Message;

#[derive(Encode, Decode, Message, Debug)]
pub enum PortalMessage {
    /// First message that Inlet sends to the Outlet
    #[n(0)] Ping,
    /// First message that Outlet sends to the Inlet
    #[n(1)] Pong,
    /// Message to indicate that connection from Outlet to the target,
    /// or from the target to the Inlet was dropped
    #[n(2)] Disconnect,
    /// Message with binary payload
    #[n(3)] Payload(#[cbor(n(0), with = "minicbor::bytes")] Vec<u8>),
}

#[derive(Encode, Decode, Message, Debug)]
pub enum PortalInternalMessage {
    /// Connection was dropped
    #[n(0)] Disconnect,
    /// Message with binary payload
    #[n(1)] Payload(#[cbor(n(0), with = "minicbor::bytes")] Vec<u8>),
}
