use ockam_core::compat::string::String;
use ockam_core::vault::{SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH};
use ockam_vault::SecretAttributes;
use minicbor::{Encode, Decode};

/// Meta-Attributes about a key
#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
pub enum MetaKeyAttributes {
    #[n(0)] SecretAttributes(#[n(0)] SecretAttributes),
}

/// Attributes that are used to identify key
#[derive(Encode, Decode, Debug, Clone, Eq, PartialEq)]
pub struct KeyAttributes {
    #[n(0)] label: String,
    #[n(1)] meta: MetaKeyAttributes,
}

impl KeyAttributes {
    /// Human-readable key name
    pub fn label(&self) -> &str {
        &self.label
    }
    pub fn meta(&self) -> &MetaKeyAttributes {
        &self.meta
    }
}

impl KeyAttributes {
    pub fn default_with_label(label: impl Into<String>) -> Self {
        Self::new(
            label.into(),
            MetaKeyAttributes::SecretAttributes(SecretAttributes::new(
                SecretType::Ed25519,
                SecretPersistence::Persistent,
                CURVE25519_SECRET_LENGTH,
            )),
        )
    }

    pub fn new(label: String, meta: MetaKeyAttributes) -> Self {
        Self { label, meta }
    }
}
