use crate::{IdentityError, IdentityIdentifier};
use ockam_core::{Decodable, Encodable, LocalInfo, LocalMessage, Result};
use minicbor::{Encode, Decode};

/// Identity SecureChannel LocalInfo unique Identifier
pub const IDENTITY_SECURE_CHANNEL_IDENTIFIER: &str = "IDENTITY_SECURE_CHANNEL_IDENTIFIER";

/// Identity SecureChannel LocalInfo used for LocalMessage
#[derive(Encode, Decode)]
pub struct IdentitySecureChannelLocalInfo {
    #[n(0)] their_identity_id: IdentityIdentifier,
}

impl IdentitySecureChannelLocalInfo {
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != IDENTITY_SECURE_CHANNEL_IDENTIFIER {
            return Err(IdentityError::InvalidLocalInfoType.into());
        }

        if let Ok(info) = Decodable::decode(value.data()) {
            return Ok(info);
        }

        Err(IdentityError::InvalidLocalInfoType.into())
    }

    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            IDENTITY_SECURE_CHANNEL_IDENTIFIER.into(),
            Encodable::encode(self)?,
        ))
    }

    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        if let Some(local_info) = local_msg
            .local_info()
            .iter()
            .find(|x| x.type_identifier() == IDENTITY_SECURE_CHANNEL_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(IdentityError::InvalidLocalInfoType.into())
        }
    }
}

impl IdentitySecureChannelLocalInfo {
    /// Key exchange name
    pub fn their_identity_id(&self) -> &IdentityIdentifier {
        &self.their_identity_id
    }
}

impl IdentitySecureChannelLocalInfo {
    /// Constructor
    pub fn new(their_identity_id: IdentityIdentifier) -> Self {
        Self { their_identity_id }
    }
}
