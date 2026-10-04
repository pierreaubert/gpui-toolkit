use super::is::is_arabic_no_space_trailing_punctuation;
use super::is::is_closing_quote;
use super::is::is_left_sticky_punctuation;
use super::is::is_myanmar_medial_glue;
use super::misc::contains_arabic_script;

pub fn ends_with_closing_quote(text: &str) -> bool {
    for ch in text.chars().rev() {
        if is_closing_quote(ch) {
            return true;
        }
        if !is_left_sticky_punctuation(ch) {
            return false;
        }
    }
    false
}

pub(super) fn ends_with_arabic_no_space_punctuation(segment: &str) -> bool {
    if !contains_arabic_script(segment) || segment.is_empty() {
        return false;
    }
    segment
        .chars()
        .next_back()
        .is_some_and(is_arabic_no_space_trailing_punctuation)
}

pub(super) fn ends_with_myanmar_medial_glue(segment: &str) -> bool {
    segment
        .chars()
        .next_back()
        .is_some_and(is_myanmar_medial_glue)
}
