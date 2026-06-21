
pub(super) mod fc {
    #![allow(dead_code)]

    use super::*;
    use fontconfig_sys as ffi;
    use fontconfig_sys::ffi_dispatch;

    #[cfg(feature = "source-fontconfig-dlopen")]
    use ffi::statics::LIB;
    #[cfg(not(feature = "source-fontconfig-dlopen"))]
    use ffi::*;

    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_uchar};
    use std::ptr;

    #[derive(Clone, Copy)]
    pub enum Error {
        NoMatch,
        TypeMismatch,
        NoId,
        OutOfMemory,
    }

    #[derive(Clone, Copy)]
    pub enum MatchKind {
        Pattern,
        Font,
        Scan,
    }

    impl MatchKind {
        fn to_u32(self) -> u32 {
            match self {
                MatchKind::Pattern => ffi::FcMatchPattern,
                MatchKind::Font => ffi::FcMatchFont,
                MatchKind::Scan => ffi::FcMatchScan,
            }
        }
    }

    // https://www.freedesktop.org/software/fontconfig/fontconfig-devel/x19.html
    #[derive(Clone, Copy)]
    pub enum Object {
        Family,
        File,
        Index,
        PostScriptName,
    }

    impl Object {
        fn as_bytes(&self) -> &[u8] {
            match self {
                Object::Family => b"family\0",
                Object::File => b"file\0",
                Object::Index => b"index\0",
                Object::PostScriptName => b"postscriptname\0",
            }
        }

        fn as_ptr(&self) -> *const libc::c_char {
            self.as_bytes().as_ptr() as *const libc::c_char
        }
    }

    pub struct Config {
        d: *mut ffi::FcConfig,
    }

    impl Config {
        // FcInitLoadConfigAndFonts
        pub fn new() -> Self {
            unsafe {
                Config {
                    d: ffi_dispatch!(
                        feature = "source-fontconfig-dlopen",
                        LIB,
                        FcInitLoadConfigAndFonts,
                    ),
                }
            }
        }
    }

    impl Drop for Config {
        fn drop(&mut self) {
            unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcConfigDestroy,
                    self.d
                );
            }
        }
    }

    pub struct Pattern {
        d: *mut ffi::FcPattern,
        c_strings: Vec<CString>,
    }

    impl Pattern {
        fn from_ptr(d: *mut ffi::FcPattern) -> Self {
            Pattern {
                d,
                c_strings: vec![],
            }
        }

        // FcPatternCreate
        pub fn new() -> Self {
            unsafe {
                Pattern::from_ptr(ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcPatternCreate,
                ))
            }
        }

        // FcNameParse
        pub fn from_name(name: &str) -> Self {
            let c_name = CString::new(name).unwrap();
            unsafe {
                Pattern::from_ptr(ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcNameParse,
                    c_name.as_ptr() as *mut c_uchar
                ))
            }
        }

        // FcPatternAddString
        pub fn push_string(&mut self, object: Object, value: String) {
            unsafe {
                let c_string = CString::new(value).unwrap();
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcPatternAddString,
                    self.d,
                    object.as_ptr(),
                    c_string.as_ptr() as *const c_uchar
                );

                // We have to keep this string, because `FcPattern` has a pointer to it now.
                self.c_strings.push(c_string)
            }
        }

        // FcConfigSubstitute
        pub fn config_substitute(&mut self, match_kind: MatchKind) {
            unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcConfigSubstitute,
                    ptr::null_mut(),
                    self.d,
                    match_kind.to_u32()
                );
            }
        }

        // FcDefaultSubstitute
        pub fn default_substitute(&mut self) {
            unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcDefaultSubstitute,
                    self.d
                );
            }
        }

        // FcFontSort
        pub fn sorted(&self, config: &Config) -> Result<FontSet, Error> {
            let mut res = ffi::FcResultMatch;
            let d = unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcFontSort,
                    config.d,
                    self.d,
                    1,
                    ptr::null_mut(),
                    &mut res
                )
            };

            match res {
                ffi::FcResultMatch => Ok(FontSet { d, idx: 0 }),
                ffi::FcResultTypeMismatch => Err(Error::TypeMismatch),
                ffi::FcResultNoId => Err(Error::NoId),
                ffi::FcResultOutOfMemory => Err(Error::OutOfMemory),
                _ => Err(Error::NoMatch),
            }
        }

        // FcFontList
        pub fn list(&self, config: &Config, set: ObjectSet) -> Result<FontSet, Error> {
            let d = unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcFontList,
                    config.d,
                    self.d,
                    set.d
                )
            };
            if !d.is_null() {
                Ok(FontSet { d, idx: 0 })
            } else {
                Err(Error::NoMatch)
            }
        }
    }

    impl Drop for Pattern {
        #[inline]
        fn drop(&mut self) {
            unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcPatternDestroy,
                    self.d
                )
            }
        }
    }

    // A read-only `FcPattern` without a destructor.
    pub struct PatternRef {
        d: *mut ffi::FcPattern,
    }

    impl PatternRef {
        // FcPatternGetString
        pub fn get_string(&self, object: Object) -> Option<String> {
            unsafe {
                let mut string = ptr::null_mut();
                let res = ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcPatternGetString,
                    self.d,
                    object.as_ptr(),
                    0,
                    &mut string
                );
                if res != ffi::FcResultMatch {
                    return None;
                }

                if string.is_null() {
                    return None;
                }

                CStr::from_ptr(string as *const c_char)
                    .to_str()
                    .ok()
                    .map(|string| string.to_owned())
            }
        }

        // FcPatternGetInteger
        pub fn get_integer(&self, object: Object) -> Option<i32> {
            unsafe {
                let mut integer = 0;
                let res = ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcPatternGetInteger,
                    self.d,
                    object.as_ptr(),
                    0,
                    &mut integer
                );
                if res != ffi::FcResultMatch {
                    return None;
                }

                Some(integer)
            }
        }
    }

    pub struct FontSet {
        d: *mut ffi::FcFontSet,
        idx: usize,
    }

    impl FontSet {
        pub fn is_empty(&self) -> bool {
            self.len() == 0
        }

        pub fn len(&self) -> usize {
            unsafe { (*self.d).nfont as usize }
        }
    }

    impl Iterator for FontSet {
        type Item = PatternRef;

        fn next(&mut self) -> Option<Self::Item> {
            if self.idx == self.len() {
                return None;
            }

            let idx = self.idx;
            self.idx += 1;

            let d = unsafe { *(*self.d).fonts.add(idx) };
            Some(PatternRef { d })
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (0, Some(self.len()))
        }
    }

    impl Drop for FontSet {
        fn drop(&mut self) {
            unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcFontSetDestroy,
                    self.d
                )
            }
        }
    }

    pub struct ObjectSet {
        d: *mut ffi::FcObjectSet,
    }

    impl ObjectSet {
        // FcObjectSetCreate
        pub fn new() -> Self {
            unsafe {
                ObjectSet {
                    d: ffi_dispatch!(feature = "source-fontconfig-dlopen", LIB, FcObjectSetCreate,),
                }
            }
        }

        // FcObjectSetAdd
        pub fn push_string(&mut self, object: Object) {
            unsafe {
                // Returns `false` if the property name cannot be inserted
                // into the set (due to allocation failure).
                assert_eq!(
                    ffi_dispatch!(
                        feature = "source-fontconfig-dlopen",
                        LIB,
                        FcObjectSetAdd,
                        self.d,
                        object.as_ptr()
                    ),
                    1
                );
            }
        }
    }

    impl Drop for ObjectSet {
        fn drop(&mut self) {
            unsafe {
                ffi_dispatch!(
                    feature = "source-fontconfig-dlopen",
                    LIB,
                    FcObjectSetDestroy,
                    self.d
                )
            }
        }
    }
}

