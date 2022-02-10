use crate::change_history::IdentityChangeHistory;
use crate::{
    ChangeBlock, EventIdentifier, IdentityChange, IdentityChangeEvent, IdentityChangeType,
    IdentityError, IdentityEventAttributes, IdentityState, IdentityStateConst, IdentityVault,
    KeyAttributes, MetaKeyAttributes, Signature, SignatureType,
};
use ockam_core::vault::PublicKey;
use ockam_core::vault::Signature as OckamVaultSignature;
use ockam_core::{Encodable, Result};
use minicbor::{Encode, Decode};

/// RotateKeyChangeData
#[derive(Encode, Decode, Debug, Clone)]
pub struct RotateKeyChangeData {
    #[n(0)] key_attributes: KeyAttributes,
    #[n(1)] public_key: PublicKey,
}

impl RotateKeyChangeData {
    /// Return key attributes
    pub fn key_attributes(&self) -> &KeyAttributes {
        &self.key_attributes
    }
    /// Return public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}

impl RotateKeyChangeData {
    /// Create RotateKeyChangeData
    pub fn new(key_attributes: KeyAttributes, public_key: PublicKey) -> Self {
        RotateKeyChangeData {
            key_attributes,
            public_key,
        }
    }
}

/// RotateKeyChange
#[derive(Encode, Decode, Debug, Clone)]
pub struct RotateKeyChange {
    #[n(0)] data: RotateKeyChangeData,
    #[n(1)] self_signature: OckamVaultSignature,
    #[n(2)] prev_signature: OckamVaultSignature,
}

impl RotateKeyChange {
    /// Return the data
    pub fn data(&self) -> &RotateKeyChangeData {
        &self.data
    }
    /// Return the self signature
    pub fn self_signature(&self) -> &OckamVaultSignature {
        &self.self_signature
    }
    /// Return the previous signature
    pub fn prev_signature(&self) -> &OckamVaultSignature {
        &self.prev_signature
    }
}

impl RotateKeyChange {
    /// Create a new RotateKeyChange
    pub fn new(
        data: RotateKeyChangeData,
        self_signature: OckamVaultSignature,
        prev_signature: OckamVaultSignature,
    ) -> Self {
        RotateKeyChange {
            data,
            self_signature,
            prev_signature,
        }
    }
}

impl<V: IdentityVault> IdentityState<V> {
    /// Rotate key event
    pub(crate) async fn make_rotate_key_event(
        &mut self,
        key_attributes: KeyAttributes,
        attributes: IdentityEventAttributes,
    ) -> Result<IdentityChangeEvent> {
        let prev_event_id = self.change_history().get_last_event_id()?;

        let last_event_in_chain = IdentityChangeHistory::find_last_key_event(
            self.change_history().as_ref(),
            key_attributes.label(),
        )?
        .clone();

        let last_key_in_chain =
            Self::get_secret_key_from_event(&last_event_in_chain, &mut self.vault).await?;

        let secret_attributes = match key_attributes.meta() {
            MetaKeyAttributes::SecretAttributes(secret_attributes) => *secret_attributes,
        };

        let secret_key = self.vault.secret_generate(secret_attributes).await?;
        let public_key = self.vault.secret_public_key_get(&secret_key).await?;

        let data = RotateKeyChangeData::new(key_attributes, public_key);
        let data_binary = Encodable::encode(&data).map_err(|_| IdentityError::BareError)?;
        let data_hash = self.vault.sha256(data_binary.as_slice()).await?;
        let self_signature = self.vault.sign(&secret_key, &data_hash).await?;
        let prev_signature = self.vault.sign(&last_key_in_chain, &data_hash).await?;
        let change = RotateKeyChange::new(data, self_signature, prev_signature);

        let identity_change = IdentityChange::new(
            IdentityStateConst::CURRENT_CHANGE_VERSION,
            attributes,
            IdentityChangeType::RotateKey(change),
        );
        let change_block = ChangeBlock::new(prev_event_id, identity_change);
        let change_block_binary = Encodable::encode(&change_block)
            .map_err(|_| IdentityError::BareError)?;

        let event_id = self.vault.sha256(&change_block_binary).await?;
        let event_id = EventIdentifier::from_hash(event_id);

        let self_signature = self.vault.sign(&secret_key, event_id.as_ref()).await?;
        let self_signature = Signature::new(SignatureType::SelfSign, self_signature);

        let root_key = self.get_root_secret_key().await?;

        let root_signature = self.vault.sign(&root_key, event_id.as_ref()).await?;
        let root_signature = Signature::new(SignatureType::RootSign, root_signature);

        let signed_change_event =
            IdentityChangeEvent::new(event_id, change_block, vec![self_signature, root_signature]);

        Ok(signed_change_event)
    }
}
