use self::store::{DefaultStore, Keystore};
use crate::home::Home;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::iter::FusedIterator;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use thiserror::Error;

mod store;

/// Manage file encryption keys.
pub struct KeyMgr {
    stores: HashMap<&'static str, Arc<dyn Keystore>>,
    keys: RwLock<HashMap<KeyId, Arc<Key>>>,
}

impl KeyMgr {
    pub const DEFAULT_STORE: &'static str = "default";

    pub fn new(home: &Arc<Home>) -> Result<Self, KeyMgrError> {
        let mut stores = HashMap::<&'static str, Arc<dyn Keystore>>::new();
        let mut keys = HashMap::new();

        // Initialize default store.
        let store = Arc::new(DefaultStore::new(home));

        for e in store.list() {
            let k = e.map_err(|e| KeyMgrError::ListKeyFailed(store.id(), e))?;

            assert!(keys.insert(k.id().clone(), Arc::new(k)).is_none());
        }

        assert!(stores.insert(store.id(), store).is_none());

        Ok(Self {
            stores,
            keys: RwLock::new(keys),
        })
    }

    pub fn has_keys(&self) -> bool {
        !self.keys.read().unwrap().is_empty()
    }

    pub fn stores(&self) -> impl FusedIterator<Item = &dyn Keystore> {
        self.stores.values().map(|s| s.as_ref())
    }

    pub fn generate(&self, store: &str) -> Result<Option<Arc<Key>>, Box<dyn Error>> {
        // Get target store.
        let store = match self.stores.get(store) {
            Some(v) => v.clone(),
            None => return Ok(None),
        };

        // Generate.
        let key = Arc::new(store.generate()?);

        assert!(self
            .keys
            .write()
            .unwrap()
            .insert(key.id().clone(), key.clone())
            .is_none());

        Ok(Some(key))
    }

    pub fn for_each_key(&self, mut f: impl FnMut(&Arc<Key>)) {
        for k in self.keys.read().unwrap().values() {
            f(k);
        }
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
