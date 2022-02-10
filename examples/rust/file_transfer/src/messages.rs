use ockam::Message;
use minicbor::{Encode, Decode};

#[derive(Encode, Decode, Debug)]
pub struct FileDescription {
    #[n(0)] pub name: String,
    #[n(1)] pub size: usize,
}
impl Message for FileDescription {}

#[derive(Encode, Decode, Debug)]
pub enum FileData {
    #[n(0)] Description(#[n(0)] FileDescription),
    #[n(1)] Data(#[cbor(n(0), with = "minicbor::bytes")] Vec<u8>),
    #[n(2)] Quit,
}

impl Message for FileData {}
