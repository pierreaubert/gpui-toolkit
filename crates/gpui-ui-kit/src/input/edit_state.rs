//! Text-editing state with grapheme-cluster cursor semantics.
//!
//! Cursor and selection positions are character indices (the contract
//! with the rest of `input.rs`), but every movement, extension, and
//! deletion lands on extended grapheme cluster boundaries
//! (`unicode_segmentation`): combining sequences, ZWJ emoji, flags, and
//! keycaps move and delete as one unit. Positions handed in from
//! outside (mouse, platform UTF-16 ranges) are snapped down to the
//! nearest grapheme boundary.
//!
//! Movement is logical-order: `move_forward` always advances toward the
//! end of the string. Bidi visual-order movement is an explicit
//! non-goal for this state machine; renderers that shape bidirectional
//! text must map visual directions to logical movement themselves.

use unicode_segmentation::UnicodeSegmentation;

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
    /// IME marked (composition) range in character indices, while a
    /// platform input method is composing. Retained across
    /// `replace_and_mark_text_in_range` calls until commit/unmark.
    pub(super) marked_range: Option<(usize, usize)>,
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
            .field("marked_range", &self.marked_range)
            .field("is_dragging", &self.is_dragging)
            .field("undo_depth", &self.undo_stack.len())
            .field("redo_depth", &self.redo_stack.len())
            .finish()
    }
}

impl EditState {
    const MAX_UNDO_HISTORY: usize = 200;

    pub(super) fn sync_external_value(&mut self, previous: &str, current: &str) {
        // Preserve uncommitted edits when the parent value did not change.
        // A changed parent value can come from external undo, import, or reset.
        if previous != current && self.editing && self.text != current {
            current.clone_into(&mut self.text);
            self.cursor = self.cursor.min(self.text.chars().count());
            self.marked_range = None;
            self.clear_selection();
        }
    }

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
        self.marked_range = None;
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
        self.marked_range = None;
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
        self.marked_range = None;
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
            marked_range: None,
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
            .map_or(self.text.len(), |(i, _)| i)
    }

    /// Convert a byte index to a character index (single pass).
    fn byte_to_char_index(&self, byte_idx: usize) -> usize {
        self.text[..byte_idx.min(self.text.len())].chars().count()
    }

    /// Grapheme boundaries (character indices) around `char_idx`:
    /// `(previous, next)` cluster boundaries on either side.
    ///
    /// One pass over the text with the extended-grapheme iterator; no
    /// allocation. `previous` is the start of the cluster ending at or
    /// containing `char_idx`; `next` is the end of the cluster starting
    /// at or containing it. Both clamp to the text ends.
    fn grapheme_around(&self, char_idx: usize) -> (usize, usize) {
        let byte = self.char_index_to_byte(char_idx);
        let mut char_pos = 0usize;
        let mut previous_start = 0usize;
        for (grapheme_byte, grapheme) in self.text.grapheme_indices(true) {
            let grapheme_chars = grapheme.chars().count();
            if grapheme_byte == byte {
                // Exactly on this cluster's start: previous is the prior
                // cluster's start, next is this cluster's end.
                return (previous_start, char_pos + grapheme_chars);
            }
            if grapheme_byte + grapheme.len() > byte {
                // char_idx sits inside this cluster.
                return (char_pos, char_pos + grapheme_chars);
            }
            previous_start = char_pos;
            char_pos += grapheme_chars;
        }
        (previous_start, char_pos)
    }

    /// Start of the grapheme cluster at or before `char_idx`.
    ///
    /// A `char_idx` already on a cluster boundary returns itself.
    pub(super) fn snap_to_grapheme(&self, char_idx: usize) -> usize {
        let byte = self.char_index_to_byte(char_idx);
        let mut char_pos = 0usize;
        for (grapheme_byte, grapheme) in self.text.grapheme_indices(true) {
            if grapheme_byte + grapheme.len() > byte {
                return char_pos;
            }
            char_pos += grapheme.chars().count();
        }
        char_pos
    }

    /// Character index one grapheme cluster after `char_idx`.
    fn next_grapheme(&self, char_idx: usize) -> usize {
        self.grapheme_around(char_idx).1
    }

    /// Character index one grapheme cluster before `char_idx`.
    fn prev_grapheme(&self, char_idx: usize) -> usize {
        self.grapheme_around(char_idx).0
    }

    /// Set the IME marked (composition) range, clamped and grapheme-snapped.
    pub(super) fn set_marked_range(&mut self, range: Option<(usize, usize)>) {
        self.marked_range = range.map(|(start, end)| {
            let start = self.snap_to_grapheme(start);
            let end = self.snap_to_grapheme(end).max(start);
            (start, end)
        });
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
        self.cursor = self.next_grapheme(self.cursor);
        self.clear_selection();
    }

    pub fn move_backward(&mut self) {
        self.cursor = self.prev_grapheme(self.cursor);
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
        self.cursor = self.snap_to_grapheme(new_cursor);
    }

    pub fn extend_backward(&mut self) {
        let new = self.prev_grapheme(self.cursor);
        self.extend_selection_to(new);
    }

    pub fn extend_forward(&mut self) {
        let new = self.next_grapheme(self.cursor);
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
            // Delete the whole grapheme cluster before the cursor.
            let start = self.prev_grapheme(self.cursor);
            let start_byte = self.char_index_to_byte(start);
            let end_byte = self.char_index_to_byte(self.cursor);
            self.text.replace_range(start_byte..end_byte, "");
            self.cursor = start;
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
            // Delete the whole grapheme cluster at the cursor.
            let end = self.next_grapheme(self.cursor);
            let start_byte = self.char_index_to_byte(self.cursor);
            let end_byte = self.char_index_to_byte(end);
            self.text.replace_range(start_byte..end_byte, "");
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
            .map_or(self.text.len(), |(i, _)| i);
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
            .map_or(self.text.len(), |(i, _)| i);
        self.text.insert(byte_pos, ch);
        self.cursor += 1;
    }

    /// Start a selection at the given position
    pub fn start_selection(&mut self, pos: usize) {
        let pos = self.snap_to_grapheme(pos);
        self.cursor = pos;
        self.selection_anchor = Some(pos);
        self.is_dragging = true;
    }

    /// Select word at the given position
    #[allow(dead_code)]
    pub fn select_word_at(&mut self, pos: usize) {
        if self.text.is_empty() {
            return;
        }
        // Byte-stepping scans (char_before_byte/char_after_byte are O(1)
        // per step): the whole selection is O(text) for the two index
        // conversions plus O(word) of scanning, never O(n^2) `nth` calls.
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let pos_byte = self.char_index_to_byte(pos);
        let char_at_pos = self.text[pos_byte..].chars().next();
        let char_before_pos = self.char_before_byte(pos_byte);

        let mut start_byte = pos_byte;
        if let Some(curr) = char_at_pos
            && !is_word_char(curr)
            && let Some((prev_byte, prev)) = char_before_pos
            && is_word_char(prev)
        {
            start_byte = prev_byte;
        }

        // On a non-word char (like whitespace), select the run of
        // whitespace/symbols, matching standard double-click behavior.
        let target_is_word = self.text[start_byte..]
            .chars()
            .next()
            .is_some_and(is_word_char);

        while let Some((prev_byte, prev)) = self.char_before_byte(start_byte) {
            if is_word_char(prev) != target_is_word {
                break;
            }
            start_byte = prev_byte;
        }

        let mut end_byte = pos_byte.max(start_byte);
        while let Some((next_byte, curr)) = self.char_after_byte(end_byte) {
            if is_word_char(curr) != target_is_word {
                break;
            }
            end_byte = next_byte;
        }

        self.selection_anchor = Some(self.byte_to_char_index(start_byte));
        self.cursor = self.byte_to_char_index(end_byte);
    }

    /// Update selection during drag
    pub fn update_selection(&mut self, pos: usize) {
        self.cursor = self.snap_to_grapheme(pos);
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
    fn grapheme_clusters_move_and_delete_as_units() {
        // "a" + "e\u{301}" (combining) + family ZWJ emoji + flag + keycap.
        let text = "ae\u{301}👨\u{200d}👩\u{200d}👧🇫🇷1\u{fe0f}\u{20e3}z";
        let mut state = EditState::new(text);
        state.clear_selection();
        state.move_to_start();
        // Five clusters: a | é | family | flag | keycap, then z.
        for expected in [1, 3, 8, 10, 13, 14] {
            state.move_forward();
            assert_eq!(state.cursor, expected, "forward over {text:?}");
        }
        state.move_forward();
        assert_eq!(state.cursor, 14, "clamped at end");
        for expected in [13, 10, 8, 3, 1, 0] {
            state.move_backward();
            assert_eq!(state.cursor, expected, "backward over {text:?}");
        }

        // Backspace from the end removes whole clusters only.
        state.move_to_end();
        state.do_backspace();
        assert_eq!(
            state.text,
            "ae\u{301}👨\u{200d}👩\u{200d}👧🇫🇷1\u{fe0f}\u{20e3}"
        );
        state.do_backspace();
        assert_eq!(state.text, "ae\u{301}👨\u{200d}👩\u{200d}👧🇫🇷");
        state.do_backspace();
        assert_eq!(state.text, "ae\u{301}👨\u{200d}👩\u{200d}👧");
        state.do_backspace();
        assert_eq!(state.text, "ae\u{301}");
        state.do_backspace();
        assert_eq!(state.text, "a");
        assert_eq!(state.cursor, 1);

        // Forward delete removes the cluster at the cursor.
        state.cursor = 0;
        state.do_delete();
        assert_eq!(state.text, "");
    }

    #[test]
    fn grapheme_snap_and_extend_never_land_inside_clusters() {
        let mut state = EditState::new("e\u{301}👍🏽x");
        state.clear_selection();
        // Skin-tone modifier cluster: chars 2..4.
        assert_eq!(state.snap_to_grapheme(3), 2);
        assert_eq!(state.snap_to_grapheme(2), 2);
        assert_eq!(state.snap_to_grapheme(1), 0);
        state.cursor = 0;
        state.extend_forward();
        assert_eq!(state.cursor, 2);
        state.extend_forward();
        assert_eq!(state.cursor, 4);
        state.extend_backward();
        assert_eq!(state.cursor, 2);
        // External positions snap on entry.
        state.start_selection(3);
        assert_eq!(state.cursor, 2);
        state.update_selection(1);
        assert_eq!(state.cursor, 0);
        // CRLF is one cluster.
        let mut crlf = EditState::new("a\r\nb");
        crlf.clear_selection();
        crlf.move_to_start();
        crlf.move_forward();
        crlf.move_forward();
        assert_eq!(crlf.cursor, 3);
        crlf.do_backspace();
        assert_eq!(crlf.text, "ab");
    }

    #[test]
    fn select_word_at_scales_to_long_inputs() {
        let text = "word ".repeat(4000); // 20k chars
        let mut state = EditState::new(&text);
        state.select_word_at(10_002);
        assert_eq!(state.selection_range(), Some((10_000, 10_004)));
    }

    #[test]
    fn marked_range_is_retained_clamped_and_cleared() {
        let mut state = EditState::new("ae\u{301}b");
        state.set_marked_range(Some((1, 3)));
        assert_eq!(state.marked_range, Some((1, 3)));
        // Mid-cluster ends snap to the cluster start.
        state.set_marked_range(Some((2, 99)));
        assert_eq!(state.marked_range, Some((1, 4)));
        state.finish_edit();
        assert_eq!(state.marked_range, None);
    }

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
