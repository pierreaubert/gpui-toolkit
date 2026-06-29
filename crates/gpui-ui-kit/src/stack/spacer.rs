use gpui::prelude::{IntoElement, Styled};
use gpui::{Div, div};

/// A spacer element that fills available space
pub struct Spacer;

impl Spacer {
    /// Create a new spacer
    pub fn new() -> Self {
        Self
    }

    /// Build into element
    pub fn build(self) -> Div {
        div().flex_1()
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for Spacer {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        self.build()
    }
}
