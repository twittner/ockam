use crate::Contact;
use ockam_core::compat::vec::Vec;
use ockam_core::Message;
use minicbor::{Encode, Decode};

#[derive(Encode, Decode, Message, Debug)]
pub(crate) enum IdentityChannelMessage {
    #[n(0)] Request {
        #[n(0)] contact: Contact,
        #[cbor(n(1), with = "minicbor::bytes")] proof: Vec<u8>
    },
    #[n(1)] Response {
        #[n(0)] contact: Contact,
        #[cbor(n(1), with = "minicbor::bytes")] proof: Vec<u8>
    },
    #[n(2)] Confirm,
}
