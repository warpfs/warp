use self::store::{DefaultStore, Keystore};
use crate::config::AppConfig;
use crate::home::Home;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter::FusedIterator;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;

mod store;

/// Manage file encryption keys.
pub struct KeyMgr {
    stores: HashMap<&'static str, Arc<dyn Keystore>>,
    keys: HashMap<KeyId, Key>,
}

impl KeyMgr {
    pub fn new(home: &Arc<Home>, config: &Arc<AppConfig>) -> Result<Self, KeyMgrError> {
        let mut stores = HashMap::<&'static str, Arc<dyn Keystore>>::new();
        let mut keys = HashMap::new();

        // Initialize default store.
        if config.key.default_storage {
            let s = Arc::new(DefaultStore::new(home));

            for k in s.list() {
                let k = k.map_err(|e| KeyMgrError::ListKeyFailed(s.id(), e))?;

                assert!(keys.insert(k.id().clone(), k).is_none());
            }

            assert!(stores.insert(s.id(), s).is_none());
        }

        Ok(Self { stores, keys })
    }

    pub fn stores(&self) -> impl FusedIterator<Item = &dyn Keystore> {
        self.stores.values().map(|s| s.as_ref())
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = &Key> + FusedIterator {
        self.keys.values()
    }
}

/// Unique identifier of a [`Key`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KeyId([u8; 16]);

impl AsRef<[u8; 16]> for KeyId {
    fn as_ref(&self) -> &[u8; 16] {
        &self.0
    }
}

impl Display for KeyId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for b in self.0 {
            write!(f, "{b:x}")?;
        }

        Ok(())
    }
}

/// Key to encrypt/decrypt files in a repository.
pub struct Key {
    id: KeyId,
    created: SystemTime,
}

impl Key {
    pub fn id(&self) -> &KeyId {
        &self.id
    }

    pub fn created(&self) -> SystemTime {
        self.created
    }
}

/// Represents an error when [`KeyMgr`] fails to initialize.
#[derive(Debug, Error)]
pub enum KeyMgrError {
    #[error("couldn't list keys from '{0}' store")]
    ListKeyFailed(&'static str, #[source] Box<dyn Error>),
}
