//! Ockam pipe protocol structures

pub mod internal;

use crate::Message;
use ockam_core::compat::vec::Vec;
use ockam_core::{Decodable, Encodable, Result, TransportMessage};
use minicbor::{Encode, Decode};

/// An indexed message for pipes
#[derive(Encode, Decode, Clone, Message, Debug)]
pub struct PipeMessage {
    /// Pipe message index
    #[n(0)] pub index: u64,
    /// Pipe message raw data
    #[cbor(n(1), with = "minicbor::bytes")]
    pub data: Vec<u8>,
}

impl PipeMessage {
    pub(crate) fn from_transport(index: u64, msg: TransportMessage) -> Result<Self> {
        let data = Encodable::encode(&msg)?;
        Ok(Self {
            index: index.into(),
            data,
        })
    }

    pub(crate) fn to_transport(&self) -> Result<TransportMessage> {
        Decodable::decode(&self.data)
    }
}
