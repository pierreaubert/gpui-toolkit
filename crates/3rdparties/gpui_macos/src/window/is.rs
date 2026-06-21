use crate :: { TISCopyCurrentKeyboardInputSource , TISGetInputSourceProperty , kTISPropertyInputSourceIsASCIICapable , kTISPropertyInputSourceType , kTISTypeKeyboardInputMode } ;
use cocoa :: { base :: { nil } , foundation :: { NSOperatingSystemVersion , NSProcessInfo } } ;
use core_foundation::base::{CFRelease, CFTypeRef};
use core_foundation_sys::base::CFEqual;
use core_foundation_sys::number::{CFBooleanGetValue, CFBooleanRef};
use std :: { ffi :: { c_void } } ;

/// Returns true if the current keyboard input source is a composition-based IME
/// (e.g. Japanese Hiragana, Korean, Chinese Pinyin) that produces non-ASCII output.
///
/// This checks two properties:
/// 1. The source type is `kTISTypeKeyboardInputMode` (an IME input mode, not a plain
///    keyboard layout). This excludes non-ASCII layouts like Armenian and Ukrainian
///    that map keys directly without composition.
/// 2. The source is not ASCII-capable, which excludes modes like Japanese Romaji that
///    produce ASCII characters and should allow multi-stroke keybindings like `jj`.
pub(super) unsafe fn is_ime_input_source_active() -> bool {
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return false;
        }

        let source_type =
            TISGetInputSourceProperty(source, kTISPropertyInputSourceType as *const c_void);
        let is_input_mode = !source_type.is_null()
            && CFEqual(
                source_type as CFTypeRef,
                kTISTypeKeyboardInputMode as CFTypeRef,
            ) != 0;

        let is_ascii = TISGetInputSourceProperty(
            source,
            kTISPropertyInputSourceIsASCIICapable as *const c_void,
        );
        let is_ascii_capable = !is_ascii.is_null() && CFBooleanGetValue(is_ascii as CFBooleanRef);

        CFRelease(source as CFTypeRef);

        is_input_mode && !is_ascii_capable
    }
}

pub(crate) fn is_macos_version_at_least(version: NSOperatingSystemVersion) -> bool {
    unsafe { NSProcessInfo::processInfo(nil).isOperatingSystemAtLeastVersion(version) }
}

