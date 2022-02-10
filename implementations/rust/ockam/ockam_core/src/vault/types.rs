use cfg_if::cfg_if;
use minicbor::{Encode, Decode};
use zeroize::Zeroize;

/// Curve25519 private key length
pub const CURVE25519_SECRET_LENGTH: usize = 32;
/// Curve25519 public key length
pub const CURVE25519_PUBLIC_LENGTH: usize = 32;
/// AES256 private key length
pub const AES256_SECRET_LENGTH: usize = 32;
/// AES128 private key length
pub const AES128_SECRET_LENGTH: usize = 16;

cfg_if! {
    if #[cfg(not(feature = "alloc"))] {
        /// Secret Key Vector
        pub type SecretKeyVec = heapless::Vec<u8, 32>;
        /// Public Key Vector
        pub type PublicKeyVec = heapless::Vec<u8, 65>;
        /// Bufer for small vectors (e.g. array of attributes). Max size - 4
        pub type SmallBuffer<T> = heapless::Vec<T, 4>;
        /// Buffer for large binaries (e.g. encrypted data). Max size - 512
        pub type Buffer<T> = heapless::Vec<T, 512>;
        pub type KeyId = heapless::String<64>;
        /// Signature Vector. Max size - 112
        pub type SignatureVec = heapless::Vec<u8, 112>;

        impl From<&str> for KeyId {
            fn from(s: &str) -> Self {
                heapless::String::from(s)
            }
        }
    }
    else {
        use alloc::vec::Vec;
        use alloc::string::String;
        /// Secret Key Vector
        pub type SecretKeyVec = Vec<u8>;
        /// Public Key Vector
        pub type PublicKeyVec = Vec<u8>;
        /// Buffer for small vectors (e.g. array of attributes)
        pub type SmallBuffer<T> = Vec<T>;
        /// Buffer for large binaries (e.g. encrypted data)
        pub type Buffer<T> = Vec<T>;
        /// ID of a Key
        pub type KeyId = String;
        ///Signature Vector
        pub type SignatureVec = Vec<u8>;
    }
}

/// Binary representation of a Secret.
#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq, Zeroize)]
#[zeroize(drop)]
pub struct SecretKey(#[cbor(n(0), with = "minicbor::bytes")] SecretKeyVec);

impl SecretKey {
    /// Create a new secret key
    pub fn new(data: SecretKeyVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// A public key
#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq, Zeroize)]
#[zeroize(drop)]
pub struct PublicKey {
    #[cbor(n(0), with = "minicbor::bytes")] data: PublicKeyVec,
    #[n(1)] stype: SecretType,
}

impl PublicKey {
    /// Public Key data
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    /// Corresponding secret key type
    pub fn stype(&self) -> SecretType {
        self.stype
    }
}

impl PublicKey {
    /// Create a new public key
    pub fn new(data: PublicKeyVec, stype: SecretType) -> Self {
        PublicKey { data, stype }
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

///Binary representation of Signature
#[derive(Encode, Decode, Clone, Debug, Eq, PartialEq, Zeroize)]
#[zeroize(drop)]
pub struct Signature(#[cbor(n(0), with = "minicbor::bytes")] SignatureVec);

impl Signature {
    /// Create a new signature
    pub fn new(data: SignatureVec) -> Self {
        Self(data)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// All possible [`SecretType`]s
#[derive(Encode, Decode, Copy, Clone, Debug, Eq, PartialEq, Zeroize)]
#[cbor(index_only)]
pub enum SecretType {
    /// Secret buffer
    #[n(0)] Buffer,
    /// AES key
    #[n(1)] Aes,
    /// Curve 22519 key
    #[n(2)] X25519,
    /// Curve 22519 key
    #[n(3)] Ed25519,
    /// BLS key
    #[cfg(feature = "bls")]
    #[n(4)] Bls,
}

/// Possible [`SecretKey`]'s persistence
#[derive(Encode, Decode, Copy, Clone, Debug, Eq, PartialEq)]
#[cbor(index_only)]
pub enum SecretPersistence {
    /// An ephemeral/temporary secret
    #[n(0)] Ephemeral,
    /// A persistent secret
    #[n(1)] Persistent,
}

/// Attributes for a specific vault [`SecretKey`]
#[derive(Encode, Decode, Copy, Clone, Debug, Eq, PartialEq)]
pub struct SecretAttributes {
    #[n(0)] stype: SecretType,
    #[n(1)] persistence: SecretPersistence,
    #[n(2)] length: usize,
}

impl SecretAttributes {
    /// Return the type of secret
    pub fn stype(&self) -> SecretType {
        self.stype
    }
    /// Return the persistence of the secret
    pub fn persistence(&self) -> SecretPersistence {
        self.persistence
    }
    /// Return the length of the secret
    pub fn length(&self) -> usize {
        self.length
    }
}

impl SecretAttributes {
    /// Create a new secret attribute
    pub fn new(stype: SecretType, persistence: SecretPersistence, length: usize) -> Self {
        SecretAttributes {
            stype,
            persistence,
            length,
        }
    }
}
