use super::Keystore;
use crate::home::Home;
use crate::key::{Key, KeyId};
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use getrandom::getrandom;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::Shake128;
use std::error::Error;
use std::ffi::CStr;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use thiserror::Error;
use zeroize::Zeroizing;

/// Implementation of [`Keystore`] using native key store of the OS.
pub struct DefaultStore {}

impl DefaultStore {
    const KEY_TYPE1: &'static CStr = c"HKDF:SHA3:256:AES:CTR:128:HMAC:SHA3:256";

    pub fn new(_: &Home) -> Self {
        Self {}
    }

    #[cfg(target_os = "linux")]
    fn store(&self, _: &KeyId, _: &[u8; 16]) -> Result<(), GenerateError> {
        todo!()
    }

    #[cfg(target_os = "macos")]
    fn store(&self, id: &KeyId, key: &[u8; 16]) -> Result<(), GenerateError> {
        let id = id.as_ref().as_ptr();
        let key = key.as_ptr();
        let tag = Self::KEY_TYPE1.as_ptr();
        let status = unsafe { default_store_store_key(id, key, tag) };

        if status == 0 {
            Ok(())
        } else {
            Err(GenerateError::StoreKeyFailed(status))
        }
    }

    #[cfg(target_os = "windows")]
    fn store(&self, _: &KeyId, _: &[u8; 16]) -> Result<(), GenerateError> {
        todo!()
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

    fn generate(self: Arc<Self>) -> Result<Key, Box<dyn Error>> {
        // Generate a new key.
        let mut key = Zeroizing::new([0u8; 16]);

        if let Err(e) = getrandom(key.deref_mut()) {
            return Err(Box::new(GenerateError::GenerateKeyFailed(e)));
        }

        // Get a key check value.
        let mut kcv = [0u8; 16];

        Aes128::new(key.deref().into()).encrypt_block((&mut kcv).into());

        // Get key ID.
        let mut hasher = Shake128::default();
        let mut id = [0u8; 16];

        hasher.update(&kcv);
        hasher.finalize_xof().read(&mut id);

        // Store the key.
        let id = KeyId(id);

        self.store(&id, &key)?;

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
enum GenerateError {
    #[error("couldn't generate a new key")]
    GenerateKeyFailed(#[source] getrandom::Error),

    #[cfg(target_os = "macos")]
    #[error("couldn't store the generated key to a keychain (code: {0})")]
    StoreKeyFailed(std::ffi::c_int),
}

#[cfg(target_os = "macos")]
extern "C" {
    fn default_store_store_key(
        id: *const u8,
        key: *const u8,
        tag: *const std::ffi::c_char,
    ) -> std::ffi::c_int;
}
