//! Internal messaging structures

use crate::Message;
use ockam_core::{Decodable, Result, Route, TransportMessage};
use minicbor::{Encode, Decode};

/// Make the sender re-send a payload
#[derive(Debug, Encode, Decode, Message)]
pub struct Resend {
    #[n(0)] pub idx: u64,
}

/// Acknowlege successful delivery
#[derive(Debug, Encode, Decode, Message)]
pub struct Ack {
    #[n(0)] pub idx: u64,
}

/// Payload sent from handshake listener to newly spawned receiver
#[derive(Debug, Encode, Decode, Message)]
pub struct Handshake {
    #[n(0)] pub route_to_sender: Route,
}

/// An enum containing all internal commands
#[derive(Debug, Encode, Decode, Message)]
pub enum InternalCmd {
    /// Issue the pipe sender to re-send
    #[n(0)] Resend(#[n(0)] Resend),
    /// Acknowlege receival of pipe message,
    #[n(1)] Ack(#[n(0)] Ack),
    /// Message received by pipe spawn listener
    #[n(2)] InitHandshake,
    /// Message sent from listener to receiver
    #[n(3)] Handshake(#[n(0)] Handshake),
    /// Initialise a pipe sender with a route
    #[n(4)] InitSender,
}

impl InternalCmd {
    pub fn from_transport(msg: &TransportMessage) -> Result<Self> {
        Decodable::decode(msg.payload())
    }
}
