//! Stream protocol request payloads

use crate::protocols::ProtocolPayload;
use crate::Message;
use ockam_core::compat::{string::String, vec::Vec};
use minicbor::{Encode, Decode};

/// Request a new mailbox to be created
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct CreateStreamRequest {
    #[n(0)] pub stream_name: Option<String>,
}

impl CreateStreamRequest {
    //noinspection ALL
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<S: Into<Option<String>>>(s: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_create",
            Self {
                stream_name: s.into(),
            },
        )
    }
}

/// Push a message into the mailbox
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct PushRequest {
    #[n(0)] pub request_id: u64,
    #[cbor(n(1), with = "minicbor::bytes")] pub data: Vec<u8>,
}

impl PushRequest {
    //noinspection ALL
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<T: Into<Vec<u8>>>(request_id: u64, data: T) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_push",
            Self {
                request_id,
                data: data.into(),
            },
        )
    }
}

/// Pull messages from the mailbox
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct PullRequest {
    #[n(0)] pub request_id: u64,
    #[n(1)] pub index: u64,
    #[n(2)] pub limit: u64,
}

impl PullRequest {
    //noinspection ALL
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new(request_id: u64, index: u64, limit: u64) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_pull",
            Self {
                request_id: request_id.into(),
                index: index.into(),
                limit: limit.into(),
            },
        )
    }
}

/// Index request protocols to get and save indices
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub enum Index {
    #[n(0)] Get {
        #[n(0)] client_id: String,
        #[n(1)] stream_name: String,
    },
    #[n(1)] Save {
        #[n(0)] client_id: String,
        #[n(1)] stream_name: String,
        #[n(2)] index: u64,
    },
}

impl Index {
    //noinspection ALL
    #[allow(dead_code)]
    pub fn get<S: Into<String>>(stream_name: S, client_id: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_index",
            Self::Get {
                client_id: client_id.into(),
                stream_name: stream_name.into(),
            },
        )
    }

    //noinspection ALL
    #[allow(dead_code)]
    pub fn save<S: Into<String>>(stream_name: S, client_id: S, index: u64) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_index",
            Self::Save {
                client_id: client_id.into(),
                stream_name: stream_name.into(),
                index: index.into(),
            },
        )
    }
}
