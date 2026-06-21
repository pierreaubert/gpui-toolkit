use std::ops::Deref;
use std::sync::Arc;

#[derive(Clone)]
pub(super) enum FontData {
    Unavailable,
    Memory(Arc<Vec<u8>>),
}

impl Deref for FontData {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        match *self {
            FontData::Unavailable => panic!("Font data unavailable!"),
            FontData::Memory(ref data) => &***data,
        }
    }
}

