use super::Keystore;
use crate::home::Home;
use crate::key::Key;
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use getrandom::getrandom;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::Shake128;
use std::error::Error;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use thiserror::Error;
use zeroize::Zeroizing;

/// Implementation of [`Keystore`] using native key store of the OS.
pub struct DefaultStore {}

impl DefaultStore {
    pub fn new(_: &Home) -> Self {
        Self {}
    }
}

impl Keystore for DefaultStore {
    fn id(&self) -> &'static str {
        "default"
    }

    fn list(self: &Arc<Self>) -> impl Iterator<Item = Key>
    where
        Self: Sized,
    {
        KeyList {}
    }

    fn new(self: Arc<Self>) -> Result<Key, Box<dyn Error>> {
        // Generate a new key.
        let mut key = Zeroizing::new([0u8; 16]);

        if let Err(e) = getrandom(key.deref_mut()) {
            return Err(Box::new(NewError::GenerateKeyFailed(e)));
        }

        // Get a key check value.
        let mut kcv = [0u8; 16];

        Aes128::new(key.deref().into()).encrypt_block((&mut kcv).into());

        // Get key ID.
        let mut hasher = Shake128::default();
        let mut id = [0u8; 16];

        hasher.update(&kcv);
        hasher.finalize_xof().read(&mut id);

        todo!()
    }
}

/// Iterator to list all keys in the [`DefaultStore`].
struct KeyList {}

impl Iterator for KeyList {
    type Item = Key;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Represents an error when [`DefaultStore::new()`] fails.
#[derive(Debug, Error)]
enum NewError {
    #[error("couldn't generate a new key")]
    GenerateKeyFailed(#[source] getrandom::Error),
}
