use crate::compat::vec::Vec;
use crate::{Address, LocalMessage, Message};
use minicbor::{Encode, Decode};

/// A command message for router implementations
///
/// If a router is implemented as a worker, it should accept this
/// message type.
#[derive(Encode, Decode, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Message)]
pub enum RouterMessage {
    /// Route the provided message towards its destination
    #[n(0)] Route(#[n(0)] LocalMessage),
    /// Register a new client to this routing scope
    #[n(1)] Register {
        /// Specify an accept scope for this client
        #[n(0)] accepts: Vec<Address>,
        /// The clients own worker bus address
        #[n(1)] self_addr: Address,
    },
}
