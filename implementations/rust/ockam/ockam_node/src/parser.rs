use ockam_core::{Message, Result};

pub(crate) fn message<M: Message>(vec: &[u8]) -> Result<M> {
    M::decode(vec)
}
