/// An internal point-in-time text state for undo/redo.
#[derive(Clone)]
struct EditSnapshot {
    text: String,
    cursor: usize,
    selection_anchor: Option<usize>,
}

/// Internal editing state for input.
#[derive(Clone, Default)]
pub struct EditState {
    /// Whether currently editing
    pub(super) editing: bool,
    /// Current edit text
    pub(super) text: String,
    /// Cursor position (character index)
    pub(super) cursor: usize,
    /// Selection anchor (where selection started). If Some, selection is from anchor to cursor.
    pub(super) selection_anchor: Option<usize>,
    /// Whether currently dragging to select
    pub(super) is_dragging: bool,
    /// Previous text states. This is intentionally not exposed through Debug:
    /// password inputs may retain their own undo history here.
    undo_stack: Vec<EditSnapshot>,
    /// States reverted by undo that may be restored by redo.
    redo_stack: Vec<EditSnapshot>,
}

impl std::fmt::Debug for EditState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditState")
            .field("editing", &self.editing)
            .field("text", &"<redacted>")
            .field("text_char_count", &self.text.chars().count())
            .field("cursor", &self.cursor)
            .field("selection_anchor", &self.selection_anchor)
            .field("is_dragging", &self.is_dragging)
            .field("undo_depth", &self.undo_stack.len())
            .field("redo_depth", &self.redo_stack.len())
            .finish()
    }
}

impl EditState {
    const MAX_UNDO_HISTORY: usize = 200;

    fn snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
            selection_anchor: self.selection_anchor,
        }
    }

    fn restore_snapshot(&mut self, snapshot: EditSnapshot) {
        self.text = snapshot.text;
        self.cursor = snapshot.cursor;
        self.selection_anchor = snapshot.selection_anchor;
        self.is_dragging = false;
    }

    /// Start a single undoable text mutation. Cursor-only operations must not
    /// call this, so undo returns to the previous text edit rather than a
    /// navigation position.
    pub(super) fn begin_text_edit(&mut self) {
        if self.undo_stack.len() == Self::MAX_UNDO_HISTORY {
            // Recycle the evicted snapshot's text allocation. Once the
            // bounded history is warm, routine edits must not allocate just
            // to preserve undo state.
            let mut snapshot = self.undo_stack.remove(0);
            snapshot.text.clear();
            snapshot.text.push_str(&self.text);
            snapshot.cursor = self.cursor;
            snapshot.selection_anchor = self.selection_anchor;
            self.undo_stack.push(snapshot);
        } else {
            self.undo_stack.push(self.snapshot());
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo_stack.pop() else {
            return false;
        };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else {
            return false;
        };
        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(next);
        true
    }

    /// End an edit and return its raw value to the caller. The edit buffer and
    /// history are cleared immediately so a password is not retained in the
    /// thread-local widget state after focus leaves the field.
    pub(super) fn finish_edit(&mut self) -> String {
        self.editing = false;
        self.clear_selection();
        self.is_dragging = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }

    /// Discard an uncommitted edit without retaining its raw text or history.
    pub(super) fn abandon_edit(&mut self) {
        self.editing = false;
        self.clear_selection();
        self.is_dragging = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.cursor = 0;
        self.text.clear();
    }
    pub fn new(value: &str) -> Self {
        let len = value.chars().count();
        Self {
            editing: true,
            text: value.to_string(),
            cursor: len,
            selection_anchor: Some(0), // Select all by default
            is_dragging: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Convert a character index to the byte index of the start of that character.
    /// Returns `self.text.len()` if `char_idx` is past the end of the text.
    fn char_index_to_byte(&self, char_idx: usize) -> usize {
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    /// Return the character immediately before `byte_pos`, along with its starting byte index.
    fn char_before_byte(&self, byte_pos: usize) -> Option<(usize, char)> {
        if byte_pos == 0 {
            return None;
        }
        let prev_byte = self.text.floor_char_boundary(byte_pos - 1);
        self.text[prev_byte..]
            .chars()
            .next()
            .map(|c| (prev_byte, c))
    }

    /// Return the character immediately at/after `byte_pos`, along with the byte index just after it.
    fn char_after_byte(&self, byte_pos: usize) -> Option<(usize, char)> {
        if byte_pos >= self.text.len() {
            return None;
        }
        self.text[byte_pos..].chars().next().map(|c| {
            let next_byte = byte_pos + c.len_utf8();
            (next_byte, c)
        })
    }

    /// Check if there's any selection
    #[allow(dead_code)]
    pub fn has_selection(&self) -> bool {
        if let Some(anchor) = self.selection_anchor {
            anchor != self.cursor
        } else {
            false
        }
    }

    /// Get selection range (start, end) where start <= end
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor);
            let end = anchor.max(self.cursor);
            (start, end)
        })
    }

    /// Check if all text is selected
    #[allow(dead_code)]
    pub fn is_all_selected(&self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            start == 0 && end == self.text.chars().count()
        } else {
            false
        }
    }

    /// Get the currently selected text
    pub fn get_selected_text(&self) -> Option<String> {
        if let Some((start, end)) = self.selection_range()
            && start != end
        {
            let start_byte = self.char_index_to_byte(start);
            let end_byte = self.char_index_to_byte(end);
            return Some(self.text[start_byte..end_byte].to_string());
        }
        None
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn move_to_start(&mut self) {
        self.cursor = 0;
        self.clear_selection();
    }

    pub fn move_to_end(&mut self) {
        self.cursor = self.text.chars().count();
        self.clear_selection();
    }

    pub fn move_forward(&mut self) {
        let len = self.text.chars().count();
        if self.cursor < len {
            self.cursor += 1;
        }
        self.clear_selection();
    }

    pub fn move_backward(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        self.clear_selection();
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.text.chars().count();
    }

    pub fn kill_to_end(&mut self) {
        self.begin_text_edit();
        let byte_pos = self.char_index_to_byte(self.cursor);
        self.text.truncate(byte_pos);
        self.clear_selection();
    }

    pub fn kill_to_start(&mut self) {
        self.begin_text_edit();
        let byte_pos = self.char_index_to_byte(self.cursor);
        self.text.replace_range(0..byte_pos, "");
        self.cursor = 0;
        self.clear_selection();
    }

    pub fn kill_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.begin_text_edit();
        let new_pos = self.word_start_backward();
        let start_byte = self.char_index_to_byte(new_pos);
        let end_byte = self.char_index_to_byte(self.cursor);
        self.text.replace_range(start_byte..end_byte, "");
        self.cursor = new_pos;
        self.clear_selection();
    }

    pub fn kill_word_forward(&mut self) {
        self.begin_text_edit();
        let new_pos = self.word_end_forward();
        let start_byte = self.char_index_to_byte(self.cursor);
        let end_byte = self.char_index_to_byte(new_pos);
        self.text.replace_range(start_byte..end_byte, "");
        self.clear_selection();
    }

    pub fn word_start_backward(&self) -> usize {
        let len = self.text.chars().count();
        let cursor = self.cursor.min(len);
        let mut byte_pos = self.char_index_to_byte(cursor);
        let mut char_pos = cursor;

        // Skip trailing whitespace
        while let Some((prev_byte, ch)) = self.char_before_byte(byte_pos) {
            if !ch.is_whitespace() {
                break;
            }
            byte_pos = prev_byte;
            char_pos -= 1;
        }
        // Skip word characters
        while let Some((prev_byte, ch)) = self.char_before_byte(byte_pos) {
            if ch.is_whitespace() {
                break;
            }
            byte_pos = prev_byte;
            char_pos -= 1;
        }
        char_pos
    }

    pub fn word_end_forward(&self) -> usize {
        let len = self.text.chars().count();
        let cursor = self.cursor.min(len);
        let mut byte_pos = self.char_index_to_byte(cursor);
        let mut char_pos = cursor;

        // Skip leading whitespace
        while let Some((next_byte, ch)) = self.char_after_byte(byte_pos) {
            if !ch.is_whitespace() {
                break;
            }
            byte_pos = next_byte;
            char_pos += 1;
        }
        // Skip word characters
        while let Some((next_byte, ch)) = self.char_after_byte(byte_pos) {
            if ch.is_whitespace() {
                break;
            }
            byte_pos = next_byte;
            char_pos += 1;
        }
        char_pos
    }

    pub fn move_word_backward(&mut self) {
        self.cursor = self.word_start_backward();
        self.clear_selection();
    }

    pub fn move_word_forward(&mut self) {
        self.cursor = self.word_end_forward();
        self.clear_selection();
    }

    pub fn extend_selection_to(&mut self, new_cursor: usize) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = new_cursor;
    }

    pub fn extend_backward(&mut self) {
        let new = if self.cursor > 0 { self.cursor - 1 } else { 0 };
        self.extend_selection_to(new);
    }

    pub fn extend_forward(&mut self) {
        let new = (self.cursor + 1).min(self.text.chars().count());
        self.extend_selection_to(new);
    }

    pub fn extend_to_start(&mut self) {
        self.extend_selection_to(0);
    }

    pub fn extend_to_end(&mut self) {
        self.extend_selection_to(self.text.chars().count());
    }

    pub fn extend_word_backward(&mut self) {
        let new = self.word_start_backward();
        self.extend_selection_to(new);
    }

    pub fn extend_word_forward(&mut self) {
        let new = self.word_end_forward();
        self.extend_selection_to(new);
    }

    /// Delete selected text, returning true if something was deleted
    pub fn delete_selection(&mut self) -> bool {
        if self
            .selection_range()
            .is_some_and(|(start, end)| start != end)
        {
            self.begin_text_edit();
            self.delete_selection_inner()
        } else {
            false
        }
    }

    fn delete_selection_inner(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range()
            && start != end
        {
            let start_byte = self.char_index_to_byte(start);
            let end_byte = self.char_index_to_byte(end);
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor = start;
            self.clear_selection();
            return true;
        }
        false
    }

    pub fn do_backspace(&mut self) {
        if self
            .selection_range()
            .is_some_and(|(start, end)| start != end)
        {
            self.begin_text_edit();
            self.delete_selection_inner();
            return;
        }
        if self.cursor > 0 {
            self.begin_text_edit();
            // Find byte positions for character before cursor
            let byte_pos = self
                .text
                .char_indices()
                .nth(self.cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let next_byte = self
                .text
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.text.len());
            self.text.replace_range(byte_pos..next_byte, "");
            self.cursor -= 1;
        }
    }

    pub fn do_delete(&mut self) {
        if self
            .selection_range()
            .is_some_and(|(start, end)| start != end)
        {
            self.begin_text_edit();
            self.delete_selection_inner();
            return;
        }
        let len = self.text.chars().count();
        if self.cursor < len {
            self.begin_text_edit();
            // Find byte positions for character at cursor
            let byte_pos = self
                .text
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.text.len());
            let next_byte = self
                .text
                .char_indices()
                .nth(self.cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.text.len());
            self.text.replace_range(byte_pos..next_byte, "");
        }
    }

    pub fn insert_text(&mut self, char_text: &str) {
        self.begin_text_edit();
        self.delete_selection_inner();
        // Find byte position for insertion
        let byte_pos = self
            .text
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        self.text.insert_str(byte_pos, char_text);
        self.cursor += char_text.chars().count();
    }

    /// Insert a single character without allocating a temporary `String`.
    pub fn insert_char(&mut self, ch: char) {
        self.begin_text_edit();
        self.delete_selection_inner();
        // Find byte position for insertion
        let byte_pos = self
            .text
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        self.text.insert(byte_pos, ch);
        self.cursor += 1;
    }

    /// Start a selection at the given position
    pub fn start_selection(&mut self, pos: usize) {
        self.cursor = pos;
        self.selection_anchor = Some(pos);
        self.is_dragging = true;
    }

    /// Select word at the given position
    #[allow(dead_code)]
    pub fn select_word_at(&mut self, pos: usize) {
        let len = self.text.chars().count();
        if len == 0 {
            return;
        }
        let pos = pos.min(len);

        // Helper to check if char is part of a word (alphanumeric or underscore)
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        // Find char at pos and char at pos - 1
        let char_at_pos = self.text.chars().nth(pos);
        let char_before_pos = if pos > 0 {
            self.text.chars().nth(pos - 1)
        } else {
            None
        };

        let mut start = pos;
        if let Some(curr) = char_at_pos
            && !is_word_char(curr)
            && start > 0
            && let Some(prev) = char_before_pos
            && is_word_char(prev)
        {
            start -= 1;
        }

        // If we are on a non-word char (like whitespace), select the run of whitespace/symbols?
        // Standard behavior: double click on whitespace selects whitespace run.
        let target_is_word = self.text.chars().nth(start).is_some_and(is_word_char);

        // Find start of word
        let mut scan_start = start;
        while scan_start > 0 {
            let prev = self.text.chars().nth(scan_start - 1).unwrap();
            if is_word_char(prev) != target_is_word {
                break;
            }
            scan_start -= 1;
        }
        start = scan_start;

        // Find end of word
        let mut end = pos;
        // Ensure we start searching from at least 'start'
        if end < start {
            end = start;
        }

        let mut scan_end = end;
        while scan_end < len {
            let curr = self.text.chars().nth(scan_end).unwrap();
            if is_word_char(curr) != target_is_word {
                break;
            }
            scan_end += 1;
        }
        end = scan_end;

        self.selection_anchor = Some(start);
        self.cursor = end;
    }

    /// Update selection during drag
    pub fn update_selection(&mut self, pos: usize) {
        self.cursor = pos;
    }

    /// End selection drag
    pub fn end_selection(&mut self) {
        self.is_dragging = false;
        // If no actual selection (anchor == cursor), clear the anchor
        if let Some(anchor) = self.selection_anchor
            && anchor == self.cursor
        {
            self.selection_anchor = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_insert_delete_backspace() {
        let mut state = EditState::new("αβγ δε");
        state.clear_selection();
        state.cursor = 4; // After "αβγ "

        state.insert_text("🔥");
        assert_eq!(state.text, "αβγ 🔥δε");
        assert_eq!(state.cursor, 5);

        state.do_backspace();
        assert_eq!(state.text, "αβγ δε");
        assert_eq!(state.cursor, 4);

        state.cursor = 3; // After "αβγ"
        state.do_delete();
        assert_eq!(state.text, "αβγδε");
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn unicode_kill_word_backward() {
        let mut state = EditState::new("hello αβγ world");
        state.clear_selection();
        state.cursor = 10; // Between the space before "world" and "w"

        state.kill_word_backward();
        assert_eq!(state.text, "hello world");
        assert_eq!(state.cursor, 6);

        state.kill_word_backward();
        assert_eq!(state.text, "world");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn unicode_kill_word_forward() {
        let mut state = EditState::new("hello αβγ world");
        state.clear_selection();
        state.cursor = 0;

        state.kill_word_forward();
        assert_eq!(state.text, " αβγ world");
        assert_eq!(state.cursor, 0);

        state.kill_word_forward();
        assert_eq!(state.text, " world");
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn unicode_kill_to_start_and_end() {
        let mut state = EditState::new("αβγ δε");
        state.clear_selection();
        state.cursor = 4;

        state.kill_to_start();
        assert_eq!(state.text, "δε");
        assert_eq!(state.cursor, 0);

        let mut state = EditState::new("αβγ δε");
        state.clear_selection();
        state.cursor = 4;

        state.kill_to_end();
        assert_eq!(state.text, "αβγ ");
        assert_eq!(state.cursor, 4);
    }

    #[test]
    fn unicode_get_selected_text_and_delete_selection() {
        let mut state = EditState::new("αβγ δε");
        state.selection_anchor = Some(1);
        state.cursor = 4;

        assert_eq!(state.get_selected_text(), Some("βγ ".to_string()));

        assert!(state.delete_selection());
        assert_eq!(state.text, "αδε");
        assert_eq!(state.cursor, 1);
        assert!(state.selection_anchor.is_none());
    }

    #[test]
    fn unicode_word_boundaries() {
        let state = EditState::new("αβγ  δε");
        // cursor after "δε"
        let mut state = state;
        state.cursor = 7;
        assert_eq!(state.word_start_backward(), 5);
        assert_eq!(state.word_end_forward(), 7);
    }

    #[test]
    fn unicode_select_word_at() {
        let mut state = EditState::new("αβγ δε");
        state.select_word_at(2); // Inside "αβγ"
        assert_eq!(state.selection_anchor, Some(0));
        assert_eq!(state.cursor, 3);

        state.select_word_at(3); // On space between words
        assert_eq!(state.selection_anchor, Some(0));
        assert_eq!(state.cursor, 3);
    }

    #[test]
    fn emoji_single_codepoint_behavior() {
        // Each emoji is a single char for this API, even though multi-byte.
        let mut state = EditState::new("🔥 family");
        state.clear_selection();
        state.cursor = 1; // After the fire emoji

        state.do_backspace();
        assert_eq!(state.text, " family");
        assert_eq!(state.cursor, 0);

        state.move_forward();
        state.move_forward();
        state.move_forward();
        state.move_forward();
        state.move_forward();
        state.move_forward();
        assert_eq!(state.cursor, 6); // " family" has 6 chars (space + family)
    }

    #[test]
    fn selection_helpers_and_movement() {
        let mut state = EditState::new("hello world");
        assert!(state.has_selection());
        assert!(state.is_all_selected());

        state.clear_selection();
        assert!(!state.has_selection());
        assert!(!state.is_all_selected());

        state.move_to_start();
        assert_eq!(state.cursor, 0);
        state.move_forward();
        assert_eq!(state.cursor, 1);
        state.move_backward();
        assert_eq!(state.cursor, 0);
        state.move_to_end();
        assert_eq!(state.cursor, 11);
    }

    #[test]
    fn extend_selection_operations() {
        let mut state = EditState::new("hello world");
        state.clear_selection();
        state.cursor = 0;

        state.extend_forward();
        assert_eq!(state.cursor, 1);
        assert!(state.has_selection());

        state.extend_to_end();
        assert_eq!(state.cursor, 11);

        state.extend_to_start();
        assert_eq!(state.cursor, 0);

        state.clear_selection();
        state.cursor = 6;
        state.extend_word_forward();
        assert_eq!(state.cursor, 11);

        state.extend_word_backward();
        assert_eq!(state.cursor, 6);
    }

    #[test]
    fn start_update_end_selection() {
        let mut state = EditState::new("hello");
        state.start_selection(2);
        assert_eq!(state.cursor, 2);
        assert!(state.is_dragging);

        state.update_selection(4);
        assert_eq!(state.cursor, 4);

        state.end_selection();
        assert!(!state.is_dragging);
        assert_eq!(state.selection_range(), Some((2, 4)));

        state.cursor = 2;
        state.selection_anchor = Some(2);
        state.end_selection();
        assert!(state.selection_anchor.is_none());
    }

    #[test]
    fn delete_selection_returns_false_when_empty() {
        let mut state = EditState {
            text: "abc".into(),
            cursor: 1,
            selection_anchor: None,
            ..Default::default()
        };
        assert!(!state.delete_selection());
        assert_eq!(state.text, "abc");
    }

    #[test]
    fn select_word_at_edges() {
        let mut state = EditState::new("hello world");
        state.select_word_at(0);
        assert_eq!(state.selection_range(), Some((0, 5)));

        state.select_word_at(6);
        assert_eq!(state.selection_range(), Some((6, 11)));

        // On whitespace with word before should select previous word
        state.select_word_at(5);
        assert_eq!(state.selection_range(), Some((0, 5)));
    }

    #[test]
    fn char_before_after_byte_positions() {
        let state = EditState::new("αβ");
        assert!(state.char_before_byte(0).is_none());
        assert_eq!(state.char_before_byte(2).unwrap().1, 'α');
        assert_eq!(state.char_after_byte(0).unwrap().1, 'α');
        assert!(state.char_after_byte(4).is_none());
    }
}

#[cfg(test)]
mod password_history_tests {
    use super::EditState;

    #[test]
    fn undo_redo_restores_text_cursor_and_selection() {
        let mut state = EditState::new("sëcret");
        state.clear_selection();
        state.cursor = 1;
        state.insert_text("•");
        assert_eq!(state.text, "s•ëcret");
        assert_eq!(state.cursor, 2);

        assert!(state.undo());
        assert_eq!(state.text, "sëcret");
        assert_eq!(state.cursor, 1);
        assert!(state.redo());
        assert_eq!(state.text, "s•ëcret");
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn debug_output_redacts_edit_text_and_history() {
        let mut state = EditState::new("not-for-debug-output");
        state.clear_selection();
        state.insert_char('!');

        let dump = format!("{state:?}");
        assert!(dump.contains("<redacted>"));
        assert!(!dump.contains("not-for-debug-output"));
    }

    #[test]
    fn undo_history_is_bounded() {
        let mut state = EditState::new("");

        for _ in 0..(EditState::MAX_UNDO_HISTORY + 5) {
            state.insert_char('x');
        }

        assert_eq!(state.undo_stack.len(), EditState::MAX_UNDO_HISTORY);
        for _ in 0..EditState::MAX_UNDO_HISTORY {
            assert!(state.undo());
        }
        assert!(!state.undo());
    }

    #[test]
    fn finish_edit_releases_text_and_undo_history() {
        let mut state = EditState::new("not-retained-after-focus-loss");
        state.clear_selection();
        state.insert_char('!');

        assert_eq!(state.finish_edit(), "not-retained-after-focus-loss!");
        assert!(state.text.is_empty());
        assert!(!state.undo());
        assert!(!state.redo());
    }
}
