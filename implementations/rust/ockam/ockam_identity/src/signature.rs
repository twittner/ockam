use ockam_core::vault::Signature as OckamVaultSignature;
use minicbor::{Encode, Decode};

/// Types of proof signatures.
#[derive(Encode, Decode, Debug, Clone, Copy, Eq, PartialEq)]
#[cbor(index_only)]
pub enum SignatureType {
    /// Root signature
    #[n(0)] RootSign,
    /// Self signature
    #[n(1)] SelfSign,
    /// Signature using previous key
    #[n(2)] PrevSign,
}

/// Signature, its type and data
#[derive(Encode, Decode, Debug, Clone)]
pub struct Signature {
    #[n(0)] stype: SignatureType,
    #[n(1)] data: OckamVaultSignature,
}

impl Signature {
    /// Return the signature type
    pub fn stype(&self) -> &SignatureType {
        &self.stype
    }
    /// Return signature data
    pub fn data(&self) -> &OckamVaultSignature {
        &self.data
    }
}

impl Signature {
    /// Create a new signature
    pub fn new(stype: SignatureType, data: OckamVaultSignature) -> Self {
        Signature { stype, data }
    }
}
