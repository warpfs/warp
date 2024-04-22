use self::store::{DefaultStore, Keystore};
use crate::config::AppConfig;
use crate::home::Home;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::sync::Arc;
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
                assert!(keys.insert(k.id().clone(), k).is_none());
            }

            assert!(stores.insert(s.id(), s).is_none());
        }

        Ok(Self { stores, keys })
    }

    pub fn stores(&self) -> impl Iterator<Item = &dyn Keystore> + FusedIterator {
        self.stores.values().map(|s| s.as_ref())
    }

    pub fn keys(&self) -> impl Iterator<Item = &Key> + ExactSizeIterator + FusedIterator {
        self.keys.values()
    }
}

/// Unique identifier of a [`Key`].
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct KeyId([u8; 16]);

/// Key to encrypt/decrypt files in a repository.
pub struct Key {
    id: KeyId,
}

impl Key {
    pub fn id(&self) -> &KeyId {
        &self.id
    }
}

/// Represents an error when [`KeyMgr`] fails to initialize.
#[derive(Debug, Error)]
pub enum KeyMgrError {}
