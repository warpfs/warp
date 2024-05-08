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
pub struct DefaultStore {
    #[allow(dead_code)]
    home: Arc<Home>,
}

impl DefaultStore {
    const KEY_TYPE1: &'static CStr = c"HKDF:SHA3:256:AES:CTR:128:HMAC:SHA3:256";

    pub fn new(home: &Arc<Home>) -> Self {
        Self { home: home.clone() }
    }

    #[cfg(target_os = "linux")]
    fn store(&self, _: &KeyId, _: Zeroizing<[u8; 16]>) -> Result<(), GenerateError> {
        todo!()
    }

    #[cfg(target_os = "macos")]
    fn store(&self, id: &KeyId, key: Zeroizing<[u8; 16]>) -> Result<(), GenerateError> {
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
    fn store(&self, id: &KeyId, mut key: Zeroizing<[u8; 16]>) -> Result<(), GenerateError> {
        use std::fs::File;
        use std::io::{Error, Write};
        use std::mem::zeroed;
        use std::ptr::null;
        use windows_sys::Win32::Foundation::{LocalFree, FALSE};
        use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

        // Setup data to encrypt.
        let data = CRYPT_INTEGER_BLOB {
            cbData: 16,
            pbData: key.as_mut_ptr(),
        };

        // Encrypt the key.
        let mut key = unsafe { zeroed() };

        if unsafe { CryptProtectData(&data, null(), null(), null(), null(), 0, &mut key) == FALSE }
        {
            return Err(GenerateError::EncryptKeyFailed(Error::last_os_error()));
        }

        // Copy the encrypted key then free the buffer.
        let mut data = Self::KEY_TYPE1.to_bytes_with_nul().to_owned();
        let len = key.cbData.try_into().unwrap();

        data.extend_from_slice(unsafe { std::slice::from_raw_parts(key.pbData, len) });

        assert!(unsafe { LocalFree(key.pbData.cast()).is_null() });

        // Get file path to store the key.
        let mut path = self.home.keys();

        path.push(id.to_string());

        // Ensure the directory to store the key are exists.
        if let Err(e) = std::fs::create_dir_all(&path) {
            return Err(GenerateError::CreateDirectoryFailed(path, e));
        }

        // Write the encrypted key.
        let mut file = match File::create_new(&path) {
            Ok(v) => v,
            Err(e) => return Err(GenerateError::CreateFileFailed(path, e)),
        };

        file.write_all(&data).unwrap(); // Let's panic when fails instead of leaving an empty file.
        Ok(())
    }
}

impl Keystore for DefaultStore {
    fn id(&self) -> &'static str {
        "default"
    }

    #[cfg(target_os = "linux")]
    fn list(self: &Arc<Self>) -> impl Iterator<Item = Result<Key, Box<dyn Error>>>
    where
        Self: Sized,
    {
        KeyList {}
    }

    #[cfg(target_os = "macos")]
    fn list(self: &Arc<Self>) -> impl Iterator<Item = Result<Key, Box<dyn Error>>>
    where
        Self: Sized,
    {
        KeyList {}
    }

    #[cfg(target_os = "windows")]
    fn list(self: &Arc<Self>) -> impl Iterator<Item = Result<Key, Box<dyn Error>>>
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

        self.store(&id, key)?;

        Ok(Key { id })
    }
}

/// Iterator to list all keys in the [`DefaultStore`].
struct KeyList {}

impl Iterator for KeyList {
    type Item = Result<Key, Box<dyn Error>>;

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

    #[cfg(target_os = "windows")]
    #[error("couldn't encrypt the generated key")]
    EncryptKeyFailed(#[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't create {0}")]
    CreateDirectoryFailed(std::path::PathBuf, #[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't create {0}")]
    CreateFileFailed(std::path::PathBuf, #[source] std::io::Error),
}

#[cfg(target_os = "macos")]
extern "C" {
    fn default_store_store_key(
        id: *const u8,
        key: *const u8,
        tag: *const std::ffi::c_char,
    ) -> std::ffi::c_int;
}
