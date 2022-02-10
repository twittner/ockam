//! Stream protocol response payloads and parser

use crate::{
    protocols::{ProtocolParser, ProtocolPayload},
    Message, OckamError, Result,
};
use ockam_core::compat::{collections::BTreeSet, string::String, vec::Vec};
use ockam_core::Decodable;
use minicbor::{Encode, Decode};

/// Response to a `CreateStreamRequest`
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct Init {
    #[n(0)] pub stream_name: String,
}

impl Init {
    //noinspection RsExternalLinter
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<S: Into<String>>(s: S) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_create",
            Self {
                stream_name: s.into(),
            },
        )
    }
}

/// Confirm push operation on the mailbox
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct PushConfirm {
    #[n(0)] pub request_id: u64,
    #[n(1)] pub status: Status,
    #[n(2)] pub index: u64,
}

impl PushConfirm {
    //noinspection RsExternalLinter
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<S: Into<Status>>(request_id: u64, status: S, index: u64) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_push",
            Self {
                request_id: request_id.into(),
                index: index.into(),
                status: status.into(),
            },
        )
    }
}

/// A simple status code
#[derive(Debug, PartialEq, Encode, Decode)]
#[cbor(index_only)]
pub enum Status {
    #[n(0)] Ok,
    #[n(1)] Error,
}

impl From<bool> for Status {
    fn from(b: bool) -> Self {
        if b {
            Self::Ok
        } else {
            Self::Error
        }
    }
}

impl From<Option<()>> for Status {
    fn from(b: Option<()>) -> Self {
        b.map(|_| Self::Ok).unwrap_or(Self::Error)
    }
}

/// Response to a `PullRequest`
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct PullResponse {
    #[n(0)] pub request_id: u64,
    #[n(1)] pub messages: Vec<StreamMessage>,
}

impl PullResponse {
    //noinspection RsExternalLinter
    #[allow(dead_code, clippy::new_ret_no_self)]
    pub fn new<T: Into<Vec<StreamMessage>>>(request_id: u64, messages: T) -> ProtocolPayload {
        ProtocolPayload::new(
            "stream_pull",
            Self {
                request_id: request_id.into(),
                messages: messages.into(),
            },
        )
    }
}

/// A stream message with a reference index
#[derive(Debug, PartialEq, Encode, Decode, Message)]
pub struct StreamMessage {
    /// Index of the message in the stream
    #[n(0)] pub index: u64,
    /// Encoded data of the message
    #[cbor(n(1), with = "minicbor::bytes")] pub data: Vec<u8>,
}

/// The index return payload
#[derive(Debug, PartialEq, Encode, Decode)]
pub struct Index {
    #[n(0)] pub client_id: String,
    #[n(1)] pub stream_name: String,
    #[n(2)] pub index: Option<u64>,
}

/// A convenience enum to wrap all possible response types
///
/// In your worker you will want to match this enum, given to you via
/// the `ProtocolParser` abstraction.
#[allow(clippy::enum_variant_names)]
#[derive(Encode, Decode, Message)]
pub enum Response {
    #[n(0)] Init(#[n(0)] Init),
    #[n(1)] PushConfirm(#[n(0)] PushConfirm),
    #[n(2)] PullResponse(#[n(0)] PullResponse),
    #[n(3)] Index(#[n(0)] Index),
}

impl ProtocolParser for Response {
    fn check_id(id: &str) -> bool {
        vec![
            "stream_create",
            "stream_push",
            "stream_pull",
            "stream_index",
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
        .contains(id)
    }

    fn parse(ProtocolPayload { protocol, data }: ProtocolPayload) -> Result<Self> {
        Ok(match protocol.as_str() {
            "stream_create" => Response::Init(Decodable::decode(&data)?),
            "stream_push" => Response::PushConfirm(Decodable::decode(&data)?),
            "stream_pull" => Response::PullResponse(Decodable::decode(&data)?),
            "stream_index" => Response::Index(Decodable::decode(&data)?),
            _ => return Err(OckamError::NoSuchProtocol.into()),
        })
    }
}
