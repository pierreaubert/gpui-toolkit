use freetype_sys :: { FT_Done_FreeType } ;
use std::mem;
use std::ptr;
use super::FtLibrary;

impl Drop for FtLibrary {
    fn drop(&mut self) {
        unsafe {
            let mut library = ptr::null_mut();
            mem::swap(&mut library, &mut self.0);
            FT_Done_FreeType(library);
        }
    }
}

