#[derive(Debug, Clone)]
pub(super) struct StringIndexConverter<'a> {
    pub(super) text: &'a str,
    pub(super) utf8_ix: usize,
    pub(super) utf16_ix: usize,
}

impl<'a> StringIndexConverter<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        Self {
            text,
            utf8_ix: 0,
            utf16_ix: 0,
        }
    }

    pub(super) fn advance_to_utf16_ix(&mut self, utf16_target: usize) {
        for (ix, c) in self.text[self.utf8_ix..].char_indices() {
            if self.utf16_ix >= utf16_target {
                self.utf8_ix += ix;
                return;
            }
            self.utf16_ix += c.len_utf16();
        }
        self.utf8_ix = self.text.len();
    }

    pub(super) fn rewind_to_utf16_ix(&mut self, utf16_target: usize) {
        if self.utf16_ix > utf16_target {
            self.utf8_ix = 0;
            self.utf16_ix = 0;
        }
        self.advance_to_utf16_ix(utf16_target);
    }
}
