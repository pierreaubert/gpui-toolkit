use crate :: error :: { FontLoadingError } ;

pub(super) fn get_slice_from_start(slice: &[u8], start: usize) -> Result<&[u8], FontLoadingError> {
    slice.get(start..).ok_or(FontLoadingError::Parse)
}

