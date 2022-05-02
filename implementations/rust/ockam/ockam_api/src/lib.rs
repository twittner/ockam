//! Basic request-response type definitions shared by all API implementations.

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/ockam.api.rs"));
}

use bytes::{Buf, BufMut};
use core::fmt;
use ockam_core::compat::rand;
use prost::Message;
use tinyvec::ArrayVec;

pub use proto::{Method, Status};

/// An API request.
#[derive(Debug, Clone)]
pub struct Request(proto::Request);

impl Request {
    pub fn decode(b: impl Buf) -> Result<Self, Error> {
        let r = proto::Request::decode(b)?;
        Ok(Request(r))
    }

    pub fn new<S: Into<String>>(m: Method, path: S) -> Self {
        Request(proto::Request {
            id: rand::random(),
            path: path.into(),
            method: m as i32,
            body: Vec::new(),
        })
    }

    pub fn get<S: Into<String>>(path: S) -> Self {
        Request::new(Method::Get, path)
    }

    pub fn post<S: Into<String>>(path: S) -> Self {
        Request::new(Method::Post, path)
    }

    pub fn id(&self) -> Id {
        Id(self.0.id)
    }

    pub fn method(&self) -> Option<Method> {
        Method::from_i32(self.0.method)
    }

    pub fn with_method(mut self, m: Method) -> Self {
        self.0.method = m as i32;
        self
    }

    pub fn path(&self) -> &str {
        &self.0.path
    }

    pub fn path_segments<const N: usize>(&self) -> Segments<N> {
        Segments::parse(self.path())
    }

    pub fn with_path<S: Into<String>>(mut self, path: S) -> Self {
        self.0.path = path.into();
        self
    }

    pub fn body(&self) -> &[u8] {
        &self.0.body
    }

    pub fn decode_body<T: Message + Default>(&self) -> Result<T, Error> {
        let t = T::decode(self.body())?;
        Ok(t)
    }

    pub fn with_body<M: Message>(mut self, body: &M) -> Self {
        self.0.body = body.encode_to_vec();
        self
    }

    pub fn encode(&self, mut buf: impl BufMut) -> Result<(), Error> {
        self.0.encode(&mut buf)?;
        Ok(())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.encode_to_vec()
    }
}

/// An API response.
#[derive(Debug, Clone)]
pub struct Response(proto::Response);

impl Response {
    pub fn decode(b: impl Buf) -> Result<Self, Error> {
        let r = proto::Response::decode(b)?;
        Ok(Response(r))
    }

    pub fn new(re: Id, s: Status) -> Self {
        Response(proto::Response {
            id: rand::random(),
            re: re.into(),
            status: s as i32,
            body: Vec::new(),
        })
    }

    pub fn id(&self) -> Id {
        Id(self.0.id)
    }

    pub fn re(&self) -> Id {
        Id(self.0.re)
    }

    pub fn status(&self) -> Option<Status> {
        Status::from_i32(self.0.status)
    }

    pub fn body(&self) -> &[u8] {
        &self.0.body
    }

    pub fn decode_body<T: Message + Default>(&self) -> Result<T, Error> {
        let t = T::decode(self.body())?;
        Ok(t)
    }

    pub fn with_body<M: Message>(mut self, body: &M) -> Self {
        self.0.body = body.encode_to_vec();
        self
    }

    pub fn encode(&self, mut buf: impl BufMut) -> Result<(), Error> {
        self.0.encode(&mut buf)?;
        Ok(())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        self.0.encode_to_vec()
    }
}

/// A Request/Response ID.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u64);

impl From<Id> for u64 {
    fn from(x: Id) -> Self {
        x.0
    }
}

/// An error response body.
#[derive(Debug, Clone)]
pub struct ErrorBody(proto::Error);

impl ErrorBody {
    pub fn new<S: Into<String>>(path: S) -> Self {
        ErrorBody(proto::Error {
            path: path.into(),
            message: String::new(),
        })
    }

    pub fn path(&self) -> &str {
        &self.0.path
    }

    pub fn message(&self) -> &str {
        &self.0.message
    }

    pub fn with_message<S: Into<String>>(mut self, m: S) -> Self {
        self.0.message = m.into();
        self
    }

    pub fn finish(self) -> impl Message {
        self.0
    }
}

/// Path segements, i.e. '/'-separated string slices.
pub struct Segments<'a, const N: usize>(ArrayVec<[&'a str; N]>);

impl<'a, const N: usize> Segments<'a, N> {
    /// Split a path into its segments.
    pub fn parse(s: &'a str) -> Self {
        if s.starts_with('/') {
            Self(s.trim_start_matches('/').splitn(N, '/').collect())
        } else {
            Self(s.splitn(N, '/').collect())
        }
    }

    pub fn as_slice(&self) -> &[&'a str] {
        &self.0[..]
    }
}

/// An API error.
#[derive(Debug)]
pub struct Error(ErrorImpl);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorImpl::Encode(e) => e.fmt(f),
            ErrorImpl::Decode(e) => e.fmt(f),
        }
    }
}

impl ockam_core::compat::error::Error for Error {
    #[cfg(feature = "std")]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0 {
            ErrorImpl::Decode(e) => Some(e),
            ErrorImpl::Encode(e) => Some(e),
        }
    }
}

#[derive(Debug)]
enum ErrorImpl {
    Decode(prost::DecodeError),
    Encode(prost::EncodeError),
}

impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Self {
        Error(ErrorImpl::Decode(e))
    }
}

impl From<prost::EncodeError> for Error {
    fn from(e: prost::EncodeError) -> Self {
        Error(ErrorImpl::Encode(e))
    }
}
