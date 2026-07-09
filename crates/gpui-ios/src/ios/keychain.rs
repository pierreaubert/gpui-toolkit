use anyhow::{Context as _, Result};
use core_foundation::{
    base::{CFType, CFTypeRef, OSStatus, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::{CFDictionaryRef, CFMutableDictionary},
    string::{CFString, CFStringRef},
};
use std::ptr;

#[allow(non_upper_case_globals)]
mod security {
    use super::*;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        pub static kSecClass: CFStringRef;
        pub static kSecClassInternetPassword: CFStringRef;
        pub static kSecAttrServer: CFStringRef;
        pub static kSecAttrAccount: CFStringRef;
        pub static kSecValueData: CFStringRef;
        pub static kSecReturnAttributes: CFStringRef;
        pub static kSecReturnData: CFStringRef;

        pub fn SecItemAdd(attributes: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
        pub fn SecItemUpdate(query: CFDictionaryRef, attributes: CFDictionaryRef) -> OSStatus;
        pub fn SecItemDelete(query: CFDictionaryRef) -> OSStatus;
        pub fn SecItemCopyMatching(query: CFDictionaryRef, result: *mut CFTypeRef) -> OSStatus;
    }

    pub const ERR_SEC_SUCCESS: OSStatus = 0;
    pub const ERR_SEC_USER_CANCELED: OSStatus = -128;
    pub const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
}

pub fn write_credentials(url: &str, username: &str, password: &[u8]) -> Result<()> {
    let url = CFString::from(url);
    let username = CFString::from(username);
    let password = CFData::from_buffer(password);

    unsafe {
        use security::*;

        let mut query_attrs = CFMutableDictionary::with_capacity(2);
        query_attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
        query_attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());

        let mut attrs = CFMutableDictionary::with_capacity(4);
        attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
        attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());
        attrs.set(kSecAttrAccount as *const _, username.as_CFTypeRef());
        attrs.set(kSecValueData as *const _, password.as_CFTypeRef());

        let mut verb = "updating";
        let mut status = SecItemUpdate(
            query_attrs.as_concrete_TypeRef(),
            attrs.as_concrete_TypeRef(),
        );
        if status == ERR_SEC_ITEM_NOT_FOUND {
            verb = "creating";
            status = SecItemAdd(attrs.as_concrete_TypeRef(), ptr::null_mut());
        }

        anyhow::ensure!(
            status == ERR_SEC_SUCCESS,
            "{verb} iOS keychain item failed: {status}"
        );
        Ok(())
    }
}

pub fn read_credentials(url: &str) -> Result<Option<(String, Vec<u8>)>> {
    let url = CFString::from(url);
    let cf_true = CFBoolean::true_value().as_CFTypeRef();

    unsafe {
        use security::*;

        let mut attrs = CFMutableDictionary::with_capacity(4);
        attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
        attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());
        attrs.set(kSecReturnAttributes as *const _, cf_true);
        attrs.set(kSecReturnData as *const _, cf_true);

        let mut result = CFTypeRef::from(ptr::null());
        let status = SecItemCopyMatching(attrs.as_concrete_TypeRef(), &mut result);
        match status {
            ERR_SEC_SUCCESS => {}
            ERR_SEC_ITEM_NOT_FOUND | ERR_SEC_USER_CANCELED => return Ok(None),
            _ => anyhow::bail!("reading iOS keychain item failed: {status}"),
        }

        let result = CFType::wrap_under_create_rule(result)
            .downcast::<core_foundation::dictionary::CFDictionary>()
            .context("iOS keychain item was not a dictionary")?;
        let username = result
            .find(kSecAttrAccount as *const _)
            .context("account was missing from iOS keychain item")?;
        let username = CFType::wrap_under_get_rule(*username)
            .downcast::<CFString>()
            .context("account was not a string in iOS keychain item")?;
        let password = result
            .find(kSecValueData as *const _)
            .context("password was missing from iOS keychain item")?;
        let password = CFType::wrap_under_get_rule(*password)
            .downcast::<CFData>()
            .context("password was not data in iOS keychain item")?;

        Ok(Some((username.to_string(), password.bytes().to_vec())))
    }
}

pub fn delete_credentials(url: &str) -> Result<()> {
    let url = CFString::from(url);

    unsafe {
        use security::*;

        let mut query_attrs = CFMutableDictionary::with_capacity(2);
        query_attrs.set(kSecClass as *const _, kSecClassInternetPassword as *const _);
        query_attrs.set(kSecAttrServer as *const _, url.as_CFTypeRef());

        let status = SecItemDelete(query_attrs.as_concrete_TypeRef());
        anyhow::ensure!(
            matches!(status, ERR_SEC_SUCCESS | ERR_SEC_ITEM_NOT_FOUND),
            "deleting iOS keychain item failed: {status}"
        );
        Ok(())
    }
}
