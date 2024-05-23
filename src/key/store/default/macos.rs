use super::KeyData;
use crate::key::Key;
use core_foundation::array::CFArray;
use core_foundation::base::{CFIndex, CFType, TCFType, ToVoid};
use core_foundation::data::CFData;
use core_foundation::dictionary::{CFDictionary, CFMutableDictionary};
use core_foundation::number::kCFBooleanTrue;
use core_foundation::string::{CFString, CFStringRef};
use core_foundation::{declare_TCFType, impl_TCFType};
use security_framework_sys::access_control::SecAccessControlGetTypeID;
use security_framework_sys::base::{errSecItemNotFound, SecAccessControlRef};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecMatchLimit,
    kSecMatchLimitAll, kSecReturnAttributes,
};
use security_framework_sys::keychain_item::SecItemCopyMatching;
use std::error::Error;
use std::ptr::null;
use thiserror::Error;

pub const KEYCHAIN_SERVICE: &str = "default-keystore";

/// Iterator to list all keys in the macOS keychain.
#[derive(Default)]
pub struct KeyList {
    items: Option<CFArray>,
    next: CFIndex,
}

impl Iterator for KeyList {
    type Item = Result<Key, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        // Load items.
        let items = match &self.items {
            Some(v) => v,
            None => {
                // Setup query.
                let mut query = CFMutableDictionary::new();
                let service = CFString::from_static_string(KEYCHAIN_SERVICE);

                unsafe { query.set(kSecMatchLimit.to_void(), kSecMatchLimitAll.to_void()) };
                unsafe { query.set(kSecClass.to_void(), kSecClassGenericPassword.to_void()) };
                unsafe { query.set(kSecAttrService.to_void(), service.to_void()) };
                unsafe { query.set(kSecReturnAttributes.to_void(), kCFBooleanTrue.to_void()) };

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

        // Get next item.
        let item = items.get(self.next)?;
        let item = unsafe { CFType::wrap_under_get_rule(*item) };
        let attrs: CFDictionary = item.downcast_into().unwrap();

        // Get account name.
        let account: CFString = match unsafe { attrs.find(kSecAttrAccount.to_void()) } {
            Some(v) => unsafe { CFType::wrap_under_get_rule(*v).downcast_into().unwrap() },
            None => return Some(Err(Box::new(ListError::NoAccountName))),
        };

        // Get ID.
        let id = match account.to_string().parse() {
            Ok(v) => v,
            Err(_) => return Some(Err(Box::new(ListError::InvalidAccountName))),
        };

        // Get data.
        let data: CFData = match unsafe { attrs.find(kSecAttrGeneric.to_void()) } {
            Some(v) => unsafe { CFType::wrap_under_get_rule(*v).downcast_into().unwrap() },
            None => return Some(Err(Box::new(ListError::NoKeyData))),
        };

        // Deserialize data.
        let data: KeyData = match postcard::from_bytes(&data) {
            Ok(v) => v,
            Err(e) => return Some(Err(Box::new(ListError::InvalidKeyData(e)))),
        };

        // Move to next item.
        self.next += 1;

        Some(Ok(Key {
            id,
            created: data.created,
        }))
    }
}

declare_TCFType! { SecAccessControl, SecAccessControlRef }

impl_TCFType!(
    SecAccessControl,
    SecAccessControlRef,
    SecAccessControlGetTypeID
);

/// Represents an error when [`KeyList::next()`] fails.
#[derive(Debug, Error)]
enum ListError {
    #[error("couldn't list keychain items (code: {0})")]
    ListKeysFailed(core_foundation::base::OSStatus),

    #[error("no kSecAttrAccount on the item")]
    NoAccountName,

    #[error("kSecAttrAccount has invalid value")]
    InvalidAccountName,

    #[error("no kSecAttrGeneric on the item")]
    NoKeyData,

    #[error("kSecAttrGeneric has invalid value")]
    InvalidKeyData(#[source] postcard::Error),
}

extern "C" {
    pub static kSecAttrGeneric: CFStringRef;
    pub static kSecAttrSynchronizable: CFStringRef;
    pub static kSecAttrSynchronizableAny: CFStringRef;
    pub static kSecUseDataProtectionKeychain: CFStringRef;
}
