use crate::SecureChannelError;
use ockam_core::compat::string::String;
use ockam_core::{Decodable, Encodable, LocalInfo, LocalMessage, Result};
use minicbor::{Encode, Decode};

/// SecureChannel LocalInfo unique Identifier
pub const SECURE_CHANNEL_IDENTIFIER: &str = "SECURE_CHANNEL_IDENTIFIER";

/// Identity SecureChannel LocalInfo used for LocalMessage
#[derive(Encode, Decode)]
pub struct SecureChannelLocalInfo {
    #[n(0)] key_exchange: String,
}

impl SecureChannelLocalInfo {
    /// Create SecureChannel LocalInfo object using Ockam Routing LocalInfo
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != SECURE_CHANNEL_IDENTIFIER {
            return Err(SecureChannelError::InvalidLocalInfoType.into());
        }

        if let Ok(info) = Decodable::decode(value.data()) {
            return Ok(info);
        }

        Err(SecureChannelError::InvalidLocalInfoType.into())
    }

    /// Create Ockam Routing LocalInfo object using SecureChannel LocalInfo
    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            SECURE_CHANNEL_IDENTIFIER.into(),
            Encodable::encode(self)?,
        ))
    }

    /// Find SecureChannel LocalInfo in a LocalMessage
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        if let Some(local_info) = local_msg
            .local_info()
            .iter()
            .find(|x| x.type_identifier() == SECURE_CHANNEL_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(SecureChannelError::InvalidLocalInfoType.into())
        }
    }
}

impl SecureChannelLocalInfo {
    /// Key exchange name
    pub fn key_exchange(&self) -> &str {
        &self.key_exchange
    }
}

impl SecureChannelLocalInfo {
    /// Constructor
    pub fn new(key_exchange: String) -> Self {
        Self { key_exchange }
    }
}
