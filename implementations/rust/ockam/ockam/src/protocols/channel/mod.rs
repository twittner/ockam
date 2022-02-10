use crate::Message;
use ockam_core::Address;
use minicbor::{Encode, Decode};

/// A simple message type to create a bi-directional channel
#[derive(Debug, Encode, Decode, Message)]
pub struct ChannelCreationHandshake {
    #[n(0)] pub channel_addr: Address,
    #[n(1)] pub tx_addr: Address,
    #[n(2)] pub tx_int_addr: Address,
    #[n(3)] pub rx_addr: Address,
    #[n(4)] pub rx_int_addr: Address,
}
