#![deny(missing_docs)]

use ockam_core::compat::{string::String, vec::Vec};
use minicbor::{Encode, Decode};

/// A lease for managing secrets
#[derive(Debug, Encode, Decode)]
pub struct Lease<T> {
    /// Unique identifier
    #[cbor(n(0), with = "minicbor::bytes")] pub id: [u8; 16],
    /// Unix timestamp in seconds when issued
    #[n(1)] pub issued: u64,
    /// Can the lease be renewed or not
    #[n(2)] pub renewable: bool,
    /// Any tags that the issuer applied to this lease
    #[n(3)] pub tags: Vec<String>,
    /// The value thats leased
    #[n(4)] pub value: T,
}

#[test]
fn test_serialization() {
    use minicbor::bytes::ByteArray;
    use ockam_core::{Decodable, Encodable, Result};

    let secret = ByteArray::from([0xFFu8; 32]);
    let lease = Lease {
        id: [0x33; 16],
        issued: 1613519081,
        renewable: true,
        tags: [String::from("can-write"), String::from("can-read")].to_vec(),
        value: secret,
    };

    let res = Encodable::encode(&lease);
    assert!(res.is_ok());
    let bare = res.unwrap();
    let res: Result<Lease<ByteArray<32>>> = Decodable::decode(&bare);
    assert!(res.is_ok());
    let lease2 = res.unwrap();

    assert_eq!(lease.id, lease2.id);
    assert_eq!(lease.issued, lease2.issued);
    assert_eq!(lease.tags, lease2.tags);
    assert_eq!(lease.value, lease2.value);
}
