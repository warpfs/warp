use core_foundation::base::TCFType;
use core_foundation::string::CFStringRef;
use core_foundation::{declare_TCFType, impl_TCFType};
use security_framework_sys::access_control::SecAccessControlGetTypeID;
use security_framework_sys::base::SecAccessControlRef;

declare_TCFType! { SecAccessControl, SecAccessControlRef }

impl_TCFType!(
    SecAccessControl,
    SecAccessControlRef,
    SecAccessControlGetTypeID
);

extern "C" {
    pub static kSecAttrApplicationTag: CFStringRef;
    pub static kSecAttrCreationDate: CFStringRef;
    pub static kSecAttrIsExtractable: CFStringRef;
    pub static kSecAttrSynchronizable: CFStringRef;
    pub static kSecAttrSynchronizableAny: CFStringRef;
    pub static kSecUseDataProtectionKeychain: CFStringRef;
}
