use super::Keystore;
use crate::home::Home;
use crate::key::{Key, KeyId, KeyMgr};
use aes::cipher::{BlockEncrypt, KeyInit};
use aes::Aes128;
use getrandom::getrandom;
use sha3::digest::{ExtendableOutput, Update, XofReader};
use sha3::Shake128;
use std::error::Error;
use std::ffi::CStr;
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
    const KEY_TYPE1: &'static CStr = c"HKDF:SHA3:256:AES:CTR:128:HMAC:SHA3:256";

    pub fn new(home: &Arc<Home>) -> Self {
        Self { home: home.clone() }
    }

    #[cfg(target_os = "linux")]
    fn store(&self, _: &KeyId, _: Zeroizing<[u8; 16]>) -> Result<SystemTime, GenerateError> {
        todo!()
    }

    #[cfg(target_os = "macos")]
    fn store(&self, id: &KeyId, key: Zeroizing<[u8; 16]>) -> Result<SystemTime, GenerateError> {
        use self::macos::{
            kSecAttrApplicationTag, kSecAttrCreationDate, kSecAttrIsExtractable,
            kSecAttrSynchronizable, kSecUseDataProtectionKeychain, SecAccessControl,
        };
        use core_foundation::base::{CFType, TCFType, ToVoid};
        use core_foundation::data::CFData;
        use core_foundation::date::{kCFAbsoluteTimeIntervalSince1970, CFDate};
        use core_foundation::dictionary::{CFDictionary, CFMutableDictionary};
        use core_foundation::number::kCFBooleanTrue;
        use core_foundation::string::CFString;
        use security_framework_sys::access_control::{
            kSecAccessControlUserPresence, kSecAttrAccessibleWhenUnlocked,
            SecAccessControlCreateWithFlags,
        };
        use security_framework_sys::item::{
            kSecAttrAccessControl, kSecAttrApplicationLabel, kSecAttrDescription,
            kSecAttrIsPermanent, kSecAttrKeyClass, kSecAttrKeyClassSymmetric, kSecAttrLabel,
            kSecClass, kSecClassKey, kSecReturnAttributes, kSecValueData,
        };
        use security_framework_sys::keychain_item::SecItemAdd;
        use std::ptr::{null, null_mut};
        use std::time::{Duration, UNIX_EPOCH};

        // Setup attributes.
        let mut attrs = CFMutableDictionary::new();
        let id = CFData::from_buffer(id.as_ref());
        let key = CFData::from_buffer(key.as_slice());
        let label = CFString::from_static_string("Warp File Key");
        let desc = CFString::from_static_string("Key to encrypt Warp files");
        let tag = CFData::from_buffer(Self::KEY_TYPE1.to_bytes());
        let access = unsafe {
            SecAccessControl::wrap_under_create_rule(SecAccessControlCreateWithFlags(
                null_mut(),
                kSecAttrAccessibleWhenUnlocked.to_void(),
                kSecAccessControlUserPresence,
                null_mut(),
            ))
        };

        unsafe { attrs.set(kSecClass.to_void(), kSecClassKey.to_void()) };
        unsafe { attrs.set(kSecValueData.to_void(), key.to_void()) };
        unsafe { attrs.set(kSecAttrLabel.to_void(), label.to_void()) };
        unsafe { attrs.set(kSecAttrDescription.to_void(), desc.to_void()) };
        unsafe { attrs.set(kSecAttrApplicationLabel.to_void(), id.to_void()) };
        unsafe { attrs.set(kSecAttrApplicationTag.to_void(), tag.to_void()) };
        unsafe { attrs.set(kSecAttrIsPermanent.to_void(), kCFBooleanTrue.to_void()) };
        unsafe { attrs.set(kSecAttrIsExtractable.to_void(), kCFBooleanTrue.to_void()) };
        unsafe { attrs.set(kSecAttrSynchronizable.to_void(), kCFBooleanTrue.to_void()) };
        unsafe { attrs.set(kSecAttrAccessControl.to_void(), access.to_void()) };
        unsafe { attrs.set(kSecReturnAttributes.to_void(), kCFBooleanTrue.to_void()) };

        unsafe {
            attrs.set(
                kSecUseDataProtectionKeychain.to_void(),
                kCFBooleanTrue.to_void(),
            )
        };

        unsafe {
            attrs.set(
                kSecAttrKeyClass.to_void(),
                kSecAttrKeyClassSymmetric.to_void(),
            )
        };

        // Add to keychain.
        let mut out = null();
        let status = unsafe { SecItemAdd(attrs.as_concrete_TypeRef(), &mut out) };

        if status != 0 {
            return Err(GenerateError::StoreKeyFailed(status));
        }

        // Get creation date.
        let attrs = unsafe { CFType::wrap_under_create_rule(out) };
        let attrs: CFDictionary = attrs.downcast_into().unwrap();
        let created = unsafe { attrs.get(kSecAttrCreationDate.to_void()) };
        let created = unsafe { CFType::wrap_under_get_rule(*created) };
        let created: CFDate = created.downcast_into().unwrap();
        let epoch = unsafe { kCFAbsoluteTimeIntervalSince1970 + created.abs_time() };

        Ok(UNIX_EPOCH + Duration::from_secs_f64(epoch))
    }

    #[cfg(target_os = "windows")]
    fn store(&self, id: &KeyId, mut key: Zeroizing<[u8; 16]>) -> Result<SystemTime, GenerateError> {
        use std::fs::{create_dir_all, remove_file, File};
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
        if let Err(e) = create_dir_all(&path) {
            return Err(GenerateError::CreateDirectoryFailed(path, e));
        }

        // Write the encrypted key.
        let mut file = match File::create_new(&path) {
            Ok(v) => v,
            Err(e) => return Err(GenerateError::CreateFileFailed(path, e)),
        };

        if let Err(e) = file.write_all(&data) {
            remove_file(&path).unwrap();
            return Err(GenerateError::WriteFileFailed(path, e));
        }

        // Get created time.
        let meta = match file.metadata() {
            Ok(v) => v,
            Err(e) => {
                remove_file(&path).unwrap();
                return Err(GenerateError::GetFileMetadataFailed(path, e));
            }
        };

        Ok(meta.created().unwrap())
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
        let created = self.store(&id, key)?;

        Ok(Key { id, created })
    }
}

/// Iterator to list all keys in the [`DefaultStore`].
#[derive(Default)]
struct KeyList {
    #[cfg(target_os = "macos")]
    items: Option<core_foundation::array::CFArray>,
    #[cfg(target_os = "macos")]
    next: core_foundation::base::CFIndex,
}

impl Iterator for KeyList {
    type Item = Result<Key, Box<dyn Error>>;

    #[cfg(target_os = "linux")]
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }

    #[cfg(target_os = "macos")]
    fn next(&mut self) -> Option<Self::Item> {
        use self::macos::{
            kSecAttrApplicationTag, kSecAttrCreationDate, kSecAttrSynchronizable,
            kSecAttrSynchronizableAny, kSecUseDataProtectionKeychain,
        };
        use core_foundation::base::{CFType, TCFType, ToVoid};
        use core_foundation::data::CFData;
        use core_foundation::date::{kCFAbsoluteTimeIntervalSince1970, CFDate};
        use core_foundation::dictionary::{CFDictionary, CFMutableDictionary};
        use core_foundation::number::kCFBooleanTrue;
        use security_framework_sys::base::errSecItemNotFound;
        use security_framework_sys::item::{
            kSecAttrKeyClass, kSecAttrKeyClassSymmetric, kSecClass, kSecClassKey, kSecMatchLimit,
            kSecMatchLimitAll, kSecReturnAttributes, kSecReturnData, kSecValueData,
        };
        use security_framework_sys::keychain_item::SecItemCopyMatching;
        use std::ptr::null;
        use std::time::{Duration, UNIX_EPOCH};

        // Get keychain items.
        let items = match &self.items {
            Some(v) => v,
            None => {
                // Setup query.
                let mut query = CFMutableDictionary::new();

                unsafe { query.set(kSecMatchLimit.to_void(), kSecMatchLimitAll.to_void()) };
                unsafe { query.set(kSecClass.to_void(), kSecClassKey.to_void()) };
                unsafe { query.set(kSecReturnAttributes.to_void(), kCFBooleanTrue.to_void()) };
                unsafe { query.set(kSecReturnData.to_void(), kCFBooleanTrue.to_void()) };

                unsafe {
                    query.set(
                        kSecAttrKeyClass.to_void(),
                        kSecAttrKeyClassSymmetric.to_void(),
                    )
                };

                unsafe {
                    query.set(
                        kSecAttrSynchronizable.to_void(),
                        kSecAttrSynchronizableAny.to_void(),
                    )
                };

                unsafe {
                    query.set(
                        kSecUseDataProtectionKeychain.to_void(),
                        kCFBooleanTrue.to_void(),
                    )
                };

                // Execute the query.
                let mut items = null();

                #[allow(non_upper_case_globals)]
                match unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut items) } {
                    0 => {}
                    errSecItemNotFound => return None,
                    v => return Some(Err(Box::new(ListError::ListKeysFailed(v)))),
                }

                // Set items.
                let items = unsafe { CFType::wrap_under_create_rule(items) };

                self.items.insert(items.downcast_into().unwrap())
            }
        };

        // Get next key.
        while let Some(item) = items.get(self.next) {
            let item = unsafe { CFType::wrap_under_get_rule(*item) };
            let attrs: CFDictionary = item.downcast_into().unwrap();

            self.next += 1;

            // Get key type.
            let ty: CFData = match unsafe { attrs.find(kSecAttrApplicationTag.to_void()) } {
                Some(v) => unsafe { CFType::wrap_under_get_rule(*v).downcast_into().unwrap() },
                None => continue,
            };

            if ty.bytes() != DefaultStore::KEY_TYPE1.to_bytes() {
                continue;
            }

            // Get key.
            let key = unsafe { attrs.get(kSecValueData.to_void()) };
            let key: CFData = unsafe { CFType::wrap_under_get_rule(*key).downcast_into().unwrap() };

            if let Ok(key) = key.bytes().try_into().map(Zeroizing::new) {
                let id = DefaultStore::get_id(&key);
                let created = unsafe { attrs.get(kSecAttrCreationDate.to_void()) };
                let created = unsafe { CFType::wrap_under_get_rule(*created) };
                let created: CFDate = created.downcast_into().unwrap();
                let created = unsafe { kCFAbsoluteTimeIntervalSince1970 + created.abs_time() };
                let created = UNIX_EPOCH + Duration::from_secs_f64(created);

                return Some(Ok(Key { id, created }));
            }
        }

        None
    }

    #[cfg(target_os = "windows")]
    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Represents an error when [`DefaultStore::list()`] fails.
#[derive(Debug, Error)]
enum ListError {
    #[cfg(target_os = "macos")]
    #[error("couldn't list keychain items (code: {0})")]
    ListKeysFailed(core_foundation::base::OSStatus),
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

    #[cfg(target_os = "windows")]
    #[error("couldn't get metadata of {0}")]
    GetFileMetadataFailed(std::path::PathBuf, #[source] std::io::Error),
}
