use std::borrow::Cow;
use winapi::um::dwrite::DWRITE_READING_DIRECTION;
use winapi::um::dwrite::DWRITE_READING_DIRECTION_LEFT_TO_RIGHT;

pub(super) struct MyTextAnalysisSource {
    pub(super) text_utf16_len: u32,
    pub(super) locale: String,
}

impl dwrote::TextAnalysisSourceMethods for MyTextAnalysisSource {
    fn get_locale_name<'a>(&'a self, text_pos: u32) -> (Cow<'a, str>, u32) {
        (self.locale.as_str().into(), self.text_utf16_len - text_pos)
    }

    fn get_paragraph_reading_direction(&self) -> DWRITE_READING_DIRECTION {
        DWRITE_READING_DIRECTION_LEFT_TO_RIGHT
    }
}

