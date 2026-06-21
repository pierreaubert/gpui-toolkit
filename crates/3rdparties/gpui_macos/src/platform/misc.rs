use cocoa :: { base :: { id } , foundation :: { NSString , NSURL } } ;
use core_foundation :: { base :: { CFTypeRef , OSStatus } , dictionary :: { CFDictionaryRef } , string :: { CFStringRef } } ;
use gpui :: Result ;
use objc :: { msg_send , runtime :: { Object } , sel , sel_impl } ;
use std :: { ffi :: { CStr , OsStr , c_void } , os :: { raw :: c_char , unix :: ffi :: OsStrExt } , path :: { PathBuf } } ;

pub(super) unsafe fn ns_url_to_path(url: id) -> Result<PathBuf> {
    let path: *mut c_char = msg_send![url, fileSystemRepresentation];
    anyhow::ensure!(!path.is_null(), "url is not a file path: {}", unsafe {
        CStr::from_ptr(url.absoluteString().UTF8String()).to_string_lossy()
    });
    Ok(PathBuf::from(OsStr::from_bytes(unsafe {
        CStr::from_ptr(path).to_bytes()
    })))
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    pub(in super::super) fn TISCopyCurrentKeyboardLayoutInputSource() -> *mut Object;
    pub(in super::super) fn TISCopyCurrentKeyboardInputSource() -> *mut Object;
    pub(in super::super) fn TISGetInputSourceProperty(
        inputSource: *mut Object,
        propertyKey: *const c_void,
    ) -> *mut Object;

    pub(in super::super) fn UCKeyTranslate(
        keyLayoutPtr: *const ::std::os::raw::c_void,
        virtualKeyCode: u16,
        keyAction: u16,
        modifierKeyState: u32,
        keyboardType: u32,
        keyTranslateOptions: u32,
        deadKeyState: *mut u32,
        maxStringLength: usize,
        actualStringLength: *mut usize,
        unicodeString: *mut u16,
    ) -> u32;
    pub(in super::super) fn LMGetKbdType() -> u16;
    pub(in super::super) static kTISPropertyUnicodeKeyLayoutData: CFStringRef;
    pub(in super::super) static kTISPropertyInputSourceID: CFStringRef;
    pub(in super::super) static kTISPropertyLocalizedName: CFStringRef;
    pub(in super::super) static kTISPropertyInputSourceIsASCIICapable: CFStringRef;
    pub(in super::super) static kTISPropertyInputSourceType: CFStringRef;
    pub(in super::super) static kTISTypeKeyboardInputMode: CFStringRef;
}

pub(super) mod security {
    #![allow(non_upper_case_globals)]
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

    pub const errSecSuccess: OSStatus = 0;
    pub const errSecUserCanceled: OSStatus = -128;
    pub const errSecItemNotFound: OSStatus = -25300;
}
