#[cfg(target_os = "macos")]
use self::macos::KeyList;
use super::Keystore;
use crate::home::Home;
use crate::key::{Key, KeyId, KeyMgr};
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use getrandom::getrandom;
use serde::{Deserialize, Serialize};
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::Shake128;
use std::error::Error;
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use zeroize::Zeroizing;

#[cfg(target_os = "macos")]
mod macos;

/// Implementation of [`Keystore`] using native key store of the OS.
pub struct DefaultStore {
    #[allow(dead_code)]
    home: Arc<Home>,
}

impl DefaultStore {
    pub fn new(home: &Arc<Home>) -> Self {
        Self { home: home.clone() }
    }

    #[cfg(target_os = "linux")]
    fn store(&self, _: &KeyId, _: &[u8], _: &KeyData) -> Result<(), GenerateError> {
        todo!()
    }

    #[cfg(target_os = "macos")]
    fn store(&self, id: &KeyId, key: &[u8], data: &KeyData) -> Result<(), GenerateError> {
        use self::macos::{
            kSecAttrGeneric, kSecAttrSynchronizable, kSecUseDataProtectionKeychain,
            SecAccessControl, KEYCHAIN_SERVICE,
        };
        use core_foundation::base::{TCFType, ToVoid};
        use core_foundation::data::CFData;
        use core_foundation::dictionary::CFMutableDictionary;
        use core_foundation::number::kCFBooleanTrue;
        use core_foundation::string::CFString;
        use security_framework_sys::access_control::{
            kSecAccessControlUserPresence, kSecAttrAccessibleWhenUnlocked,
            SecAccessControlCreateWithFlags,
        };
        use security_framework_sys::item::{
            kSecAttrAccessControl, kSecAttrAccount, kSecAttrDescription, kSecAttrLabel,
            kSecAttrService, kSecClass, kSecClassGenericPassword, kSecValueData,
        };
        use security_framework_sys::keychain_item::SecItemAdd;
        use std::ptr::null_mut;

        // Setup attributes.
        let mut attrs = CFMutableDictionary::new();
        let key = CFData::from_buffer(key);
        let data = CFData::from_buffer(&postcard::to_stdvec(data).unwrap());
        let label = CFString::from_static_string("Warp File Key");
        let desc = CFString::from_static_string("Key to encrypt Warp files");
        let service = CFString::from_static_string(KEYCHAIN_SERVICE);
        let id = CFString::new(&id.to_string());
        let access = unsafe {
            SecAccessControl::wrap_under_create_rule(SecAccessControlCreateWithFlags(
                null_mut(),
                kSecAttrAccessibleWhenUnlocked.to_void(),
                kSecAccessControlUserPresence,
                null_mut(),
            ))
        };

        unsafe { attrs.set(kSecClass.to_void(), kSecClassGenericPassword.to_void()) };
        unsafe { attrs.set(kSecValueData.to_void(), key.to_void()) };
        unsafe { attrs.set(kSecAttrGeneric.to_void(), data.to_void()) };
        unsafe { attrs.set(kSecAttrService.to_void(), service.to_void()) };
        unsafe { attrs.set(kSecAttrAccount.to_void(), id.to_void()) };
        unsafe { attrs.set(kSecAttrLabel.to_void(), label.to_void()) };
        unsafe { attrs.set(kSecAttrDescription.to_void(), desc.to_void()) };
        unsafe { attrs.set(kSecAttrSynchronizable.to_void(), kCFBooleanTrue.to_void()) };
        unsafe { attrs.set(kSecAttrAccessControl.to_void(), access.to_void()) };

        unsafe {
            attrs.set(
                kSecUseDataProtectionKeychain.to_void(),
                kCFBooleanTrue.to_void(),
            )
        };

        // Add to keychain.
        let status = unsafe { SecItemAdd(attrs.as_concrete_TypeRef(), null_mut()) };

        if status != 0 {
            Err(GenerateError::StoreKeyFailed(status))
        } else {
            Ok(())
        }
    }

    #[cfg(target_os = "windows")]
    fn store(&self, id: &KeyId, key: &[u8], data: &KeyData) -> Result<(), GenerateError> {
        use std::fs::{create_dir_all, remove_file, File};
        use std::io::{Error, Write};
        use std::mem::zeroed;
        use std::ptr::null;
        use windows_sys::Win32::Foundation::{LocalFree, FALSE};
        use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};

        // Setup data to encrypt.
        let data = CRYPT_INTEGER_BLOB {
            cbData: data.len().try_into().unwrap(),
            pbData: data.as_ptr().cast_mut(),
        };

        // Encrypt the data.
        let mut buf = unsafe { zeroed() };

        if unsafe { CryptProtectData(&data, null(), null(), null(), null(), 0, &mut buf) == FALSE }
        {
            return Err(GenerateError::EncryptKeyFailed(Error::last_os_error()));
        }

        // Copy the encrypted data then free the buffer.
        let len = buf.cbData.try_into().unwrap();
        let data = unsafe { std::slice::from_raw_parts(buf.pbData, len).to_owned() };

        assert!(unsafe { LocalFree(buf.pbData.cast()).is_null() });

        // Ensure the directory to store the key are exists.
        let mut path = self.home.keys();

        if let Err(e) = create_dir_all(&path) {
            return Err(GenerateError::CreateDirectoryFailed(path, e));
        }

        path.push(id.to_string());

        // Write the encrypted key.
        let mut file = match File::create_new(&path) {
            Ok(v) => v,
            Err(e) => return Err(GenerateError::CreateFileFailed(path, e)),
        };

        if let Err(e) = file.write_all(&data) {
            remove_file(&path).unwrap();
            return Err(GenerateError::WriteFileFailed(path, e));
        }

        Ok(())
    }

    fn get_id(key: &[u8; 16]) -> KeyId {
        // Get a key check value.
        let mut kcv = [0u8; 16];

        Aes128::new(key.into()).encrypt_block((&mut kcv).into());

        // Get key ID.
        let mut hasher = Shake128::default();
        let mut id = [0u8; 16];

        hasher.update(&kcv);
        hasher.finalize_xof().read(&mut id);

        KeyId(id)
    }
}

impl Keystore for DefaultStore {
    fn id(&self) -> &'static str {
        KeyMgr::DEFAULT_STORE
    }

    fn list(self: &Arc<Self>) -> impl Iterator<Item = Result<Key, Box<dyn Error>>>
    where
        Self: Sized,
    {
        KeyList::default()
    }

    fn generate(self: Arc<Self>) -> Result<Key, Box<dyn Error>> {
        // Generate a new key.
        let mut key = Zeroizing::new([0u8; 16]);

        if let Err(e) = getrandom(key.deref_mut()) {
            return Err(Box::new(GenerateError::GenerateKeyFailed(e)));
        }

        // Store the key.
        let id = Self::get_id(&key);
        let data = KeyData {
            kdf: KeyDerivation::HkdfSha3256,
            enc: Encryption::AesCtr128,
            mac: Some(Mac::HmacSha3256),
            created: SystemTime::now(),
        };

        self.store(&id, key.as_ref(), &data)?;

        Ok(Key {
            id,
            created: data.created,
        })
    }
}

/// Per-key data stored unencrypted with the key.
#[derive(Serialize, Deserialize)]
struct KeyData {
    kdf: KeyDerivation,
    enc: Encryption,
    mac: Option<Mac>,
    created: SystemTime,
}

/// Key derivation algorithm of the key.
#[derive(Serialize, Deserialize)]
enum KeyDerivation {
    HkdfSha3256,
}

/// Encryption algorithm of the key.
#[derive(Serialize, Deserialize)]
enum Encryption {
    AesCtr128,
}

/// Message authentication code of the key.
#[derive(Serialize, Deserialize)]
enum Mac {
    HmacSha3256,
}

/// Represents an error when [`DefaultStore::new()`] fails.
#[derive(Debug, Error)]
enum GenerateError {
    #[error("couldn't generate a new key")]
    GenerateKeyFailed(#[source] getrandom::Error),

    #[cfg(target_os = "macos")]
    #[error("couldn't store the generated key to a keychain (code: {0})")]
    StoreKeyFailed(core_foundation::base::OSStatus),

    #[cfg(target_os = "windows")]
    #[error("couldn't encrypt the generated key")]
    EncryptKeyFailed(#[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't create {0}")]
    CreateDirectoryFailed(std::path::PathBuf, #[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't create {0}")]
    CreateFileFailed(std::path::PathBuf, #[source] std::io::Error),

    #[cfg(target_os = "windows")]
    #[error("couldn't write {0}")]
    WriteFileFailed(std::path::PathBuf, #[source] std::io::Error),
}
