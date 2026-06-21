
pub(super) fn align_offset(offset: &mut usize) {
    *offset = (*offset).div_ceil(256) * 256;
}

