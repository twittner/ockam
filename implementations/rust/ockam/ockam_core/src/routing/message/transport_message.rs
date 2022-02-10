use crate::{compat::vec::Vec, Message, Route};
use core::fmt::{self, Display, Formatter};
use minicbor::{Encode, Decode};

/// A generic transport message
///
/// While this type is exposed in ockam_core (and the root `ockam`
/// crate) in order to provide a mechanism for third-party developers
/// to create custom transport channel routers.  Casual users of ockam
/// should never have to interact with this type directly.
#[derive(Encode, Decode, Debug, Clone, Hash, PartialOrd, Ord, Eq, PartialEq, Message)]
pub struct TransportMessage {
    /// The transport protocol version
    #[n(0)] pub version: u8,
    /// Onward message route
    #[n(1)] pub onward_route: Route,
    /// Return message route
    ///
    /// This field must be populated by routers handling this message
    /// along the way.
    #[n(2)] pub return_route: Route,
    /// The message payload
    #[cbor(n(3), with = "minicbor::bytes")] payload: Vec<u8>
}

impl TransportMessage {
    /// Create a new v1 transport message with empty return route
    pub fn v1(
        onward_route: impl Into<Route>,
        return_route: impl Into<Route>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            version: 1,
            onward_route: onward_route.into(),
            return_route: return_route.into(),
            payload
        }
    }

    /// Get access to the payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Deconstruct this type into the payload part.
    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }
}

impl Display for TransportMessage {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Message (onward route: {}, return route: {})",
            self.onward_route, self.return_route
        )
    }
}
