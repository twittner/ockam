use crate::VaultError;
use ockam_core::compat::{collections::BTreeMap, string::String, sync::RwLock};
use ockam_core::vault::{Secret, SecretAttributes, SecretKey};
use ockam_core::Result;
use tracing::info;

/// Vault implementation that stores secrets in memory and uses software crypto.
///
/// # Examples
///
/// ```
/// use ockam_vault::SoftwareVault;
/// use ockam_core::Result;
/// use ockam_core::vault::{SecretAttributes, SecretType, SecretPersistence, CURVE25519_SECRET_LENGTH, SecretVault, Signer, Verifier};
///
/// async fn example() -> Result<()> {
///     let mut vault = SoftwareVault::default();
///
///     let mut attributes = SecretAttributes::new(
///         SecretType::X25519,
///         SecretPersistence::Ephemeral,
///         CURVE25519_SECRET_LENGTH,
///     );
///
///     let secret = vault.secret_generate(attributes).await?;
///     let public = vault.secret_public_key_get(&secret).await?;
///
///     let data = "Very important stuff".as_bytes();
///
///     let signature = vault.sign(&secret, data).await?;
///     assert!(vault.verify(&signature, &public, data).await?);
///
///     Ok(())
/// }
/// ```
///
/// # Synchronous and Asynchronous Methods
///
/// In addition to the asynchronous methods defined by the various [vault
/// traits](ockam_core::vault), `SoftwareVault` provides synchronous access to
/// most of its functionality.
///
/// These require concrete access to a `SoftwareVault` instance — 
/// 1. 
///
/// However, these are provided as direct inherent methods, and *not* as a
/// trait or set of traits. This is intentional -- the software vault may be the
/// only Vault implementation that is able to always provide synchronous results
/// without blocking.
///
/// he asynchronous methods are the more fundamental abstraction, and the
/// software vault may be the only implementation
///
/// the best possible way to implement a Vault on some platforms
///
/// which provide their results asynchronously, Wasm 
///
///
///
/// The cryptographic routines provided by Sof
pub struct SoftwareVault {
    // Ideally, this would probably be lockfree (using `sharded-slab`, for
    // example). 
    pub(crate) inner: RwLock<VaultStorage>,
}

pub(crate) struct VaultStorage {
    pub(crate) entries: BTreeMap<usize, VaultEntry>,
    next_id: usize,
}

impl SoftwareVault {
    /// Create a new SoftwareVault
    pub fn new() -> Self {
        info!("Creating vault");
        Self {
            inner: RwLock::new(VaultStorage {
                entries: BTreeMap::new(),
                next_id: 0,
            }),
        }
    }

    pub(crate) fn insert(&self, entry: VaultEntry) -> Secret {
        let mut storage = self.inner.write();
        let next_id = storage.next_id + 1;
        storage.next_id = next_id;
        storage.entries.insert(next_id, entry);
        Secret::new(next_id)
    }

    pub(crate) fn remove(&self, entry: Secret) -> Option<VaultEntry> {
        let mut storage = self.inner.write();
        storage.entries.remove(&entry.index())
    }

    // TODO: we only need this because we don't have mapped guards on `std`.
    pub(crate) fn with_entry<Ret, F: FnOnce(&VaultEntry) -> Ret>(
        &self,
        secret: &Secret,
        reader: F,
    ) -> Result<Ret, VaultError> {
        let storage = self.inner.read();
        let entry = storage.get_entry(&secret)?;
        Ok(reader(entry))
    }
}

impl VaultStorage {
    pub(crate) fn get_entry<'a>(&'a self, secret: &Secret) -> Result<&'a VaultEntry, VaultError> {
        self.entries
            .get(&secret.index())
            .ok_or(VaultError::EntryNotFound)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct VaultEntry {
    key_id: Option<String>,
    key_attributes: SecretAttributes,
    key: SecretKey,
}

impl VaultEntry {
    pub fn key_id(&self) -> &Option<String> {
        &self.key_id
    }
    pub fn key_attributes(&self) -> SecretAttributes {
        self.key_attributes
    }
    pub fn key(&self) -> &SecretKey {
        &self.key
    }
}

impl VaultEntry {
    pub fn new(key_id: Option<String>, key_attributes: SecretAttributes, key: SecretKey) -> Self {
        VaultEntry {
            key_id,
            key_attributes,
            key,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;

    #[test]
    fn new_vault() {
        let vault = SoftwareVault::new();
        assert_eq!(vault.next_id, 0);
        assert_eq!(vault.entries.len(), 0);
    }
}
