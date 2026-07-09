use super::stack_spacing::StackSpacing;
use super::types::StackAlign;
use super::types::StackJustify;
use super::types::StackOverflow;
use super::types::StackSize;
use crate::mobile::is_mobile;
use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{AnyElement, App, Div, ElementId, Pixels, Window, div, px, relative};
use gpui_design::DesignSystem;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static HSTACK_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_hstack_id() -> ElementId {
    ElementId::Name(format!("hstack-{}", HSTACK_COUNTER.fetch_add(1, Ordering::Relaxed)).into())
}

/// A horizontal stack (row) layout
///
/// Behaves like CSS `display: flex; flex-direction: row` with responsive sizing support.
pub struct HStack {
    pub(super) children: Vec<AnyElement>,
    pub(super) spacing: StackSpacing,
    pub(super) align: StackAlign,
    pub(super) justify: StackJustify,
    pub(super) wrap: bool,
    pub(super) width: Option<StackSize>,
    pub(super) height: Option<StackSize>,
    pub(super) flex_grow: Option<f32>,
    pub(super) flex_shrink: Option<f32>,
    pub(super) flex_basis: Option<Pixels>,
    pub(super) overflow_x: StackOverflow,
    pub(super) overflow_y: StackOverflow,
    pub(super) min_width: Option<Pixels>,
    pub(super) min_height: Option<Pixels>,
    pub(super) max_width: Option<Pixels>,
    pub(super) max_height: Option<Pixels>,
    pub(super) design: Option<Arc<DesignSystem>>,
    pub(super) id: ElementId,
    pub(super) scrollable_on_mobile: Option<bool>,
}

impl HStack {
    /// Create a new horizontal stack
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: StackSpacing::default(),
            align: StackAlign::Center,
            justify: StackJustify::default(),
            wrap: false,
            width: None,
            height: None,
            flex_grow: None,
            flex_shrink: None,
            flex_basis: None,
            overflow_x: StackOverflow::default(),
            overflow_y: StackOverflow::default(),
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            design: None,
            id: next_hstack_id(),
            scrollable_on_mobile: None,
        }
    }

    /// Add a child element
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }

    /// Set spacing between children
    pub fn spacing(mut self, spacing: StackSpacing) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set cross-axis alignment (vertical for HStack)
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    /// Set main-axis alignment (horizontal for HStack)
    pub fn justify(mut self, justify: StackJustify) -> Self {
        self.justify = justify;
        self
    }

    /// Enable flex wrap (items wrap to next line when they don't fit)
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Set width of the stack
    pub fn width(mut self, size: StackSize) -> Self {
        self.width = Some(size);
        self
    }

    /// Set height of the stack
    pub fn height(mut self, size: StackSize) -> Self {
        self.height = Some(size);
        self
    }

    /// Fill both width and height (100% of parent)
    pub fn full(mut self) -> Self {
        self.width = Some(StackSize::Full);
        self.height = Some(StackSize::Full);
        self
    }

    /// Set flex-grow (how much the stack grows to fill available space)
    /// Use `flex_1()` for common case of grow=1
    pub fn grow(mut self, factor: f32) -> Self {
        self.flex_grow = Some(factor);
        self
    }

    /// Shorthand for flex-grow: 1, flex-shrink: 1, flex-basis: 0 (like CSS flex: 1)
    pub fn flex_1(mut self) -> Self {
        self.flex_grow = Some(1.0);
        self.flex_shrink = Some(1.0);
        self.flex_basis = Some(px(0.0));
        self
    }

    /// Set flex-shrink (how much the stack shrinks when space is limited)
    pub fn shrink(mut self, factor: f32) -> Self {
        self.flex_shrink = Some(factor);
        self
    }

    /// Set flex-basis (initial size before growing/shrinking)
    pub fn basis(mut self, size: Pixels) -> Self {
        self.flex_basis = Some(size);
        self
    }

    /// Set horizontal overflow handling
    pub fn overflow_x(mut self, overflow: StackOverflow) -> Self {
        self.overflow_x = overflow;
        self
    }

    /// Set vertical overflow handling
    pub fn overflow_y(mut self, overflow: StackOverflow) -> Self {
        self.overflow_y = overflow;
        self
    }

    /// Set both overflow directions
    pub fn overflow(mut self, overflow: StackOverflow) -> Self {
        self.overflow_x = overflow;
        self.overflow_y = overflow;
        self
    }

    /// Set minimum width
    pub fn min_w(mut self, size: Pixels) -> Self {
        self.min_width = Some(size);
        self
    }

    /// Set minimum height
    pub fn min_h(mut self, size: Pixels) -> Self {
        self.min_height = Some(size);
        self
    }

    /// Set maximum width
    pub fn max_w(mut self, size: Pixels) -> Self {
        self.max_width = Some(size);
        self
    }

    /// Set maximum height
    pub fn max_h(mut self, size: Pixels) -> Self {
        self.max_height = Some(size);
        self
    }

    /// Set an explicit design system override.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set an explicit element ID.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Override whether this stack becomes horizontally scrollable on mobile.
    /// When `None` (the default), scrolling is enabled on mobile unless the
    /// overflow is explicitly set to `Hidden`.
    pub fn scrollable_on_mobile(mut self, scrollable: bool) -> Self {
        self.scrollable_on_mobile = Some(scrollable);
        self
    }

    /// Build into element
    pub fn build(self) -> Div {
        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        self.build_with_design(&design)
    }

    /// Build into element using explicit design-system defaults.
    pub fn build_with_design(self, design: &DesignSystem) -> Div {
        let mut stack = div().flex().gap(self.spacing.to_pixels_with_design(design));

        if self.wrap {
            stack = stack.flex_wrap();
        }

        // Apply width
        stack = match self.width {
            Some(StackSize::Auto) => stack,
            Some(StackSize::Full) => stack.w_full(),
            Some(StackSize::Fixed(px)) => stack.w(px),
            Some(StackSize::Fraction(f)) => stack.w(relative(f)),
            None => stack,
        };

        // Apply height
        stack = match self.height {
            Some(StackSize::Auto) => stack,
            Some(StackSize::Full) => stack.h_full(),
            Some(StackSize::Fixed(px)) => stack.h(px),
            Some(StackSize::Fraction(f)) => stack.h(relative(f)),
            None => stack,
        };

        // Apply flex properties
        if let Some(grow) = self.flex_grow {
            stack = stack.flex_grow(grow);
        }
        if let Some(shrink) = self.flex_shrink {
            stack = stack.flex_shrink(shrink);
        }
        if let Some(basis) = self.flex_basis {
            stack = stack.flex_basis(basis);
        }

        // Apply min/max constraints
        if let Some(min_w) = self.min_width {
            stack = stack.min_w(min_w);
        }
        if let Some(min_h) = self.min_height {
            stack = stack.min_h(min_h);
        }
        if let Some(max_w) = self.max_width {
            stack = stack.max_w(max_w);
        }
        if let Some(max_h) = self.max_height {
            stack = stack.max_h(max_h);
        }

        // Apply overflow (GPUI uses overflow_hidden; scroll is handled separately with scroll views)
        stack = match self.overflow_x {
            StackOverflow::Visible => stack,
            StackOverflow::Hidden | StackOverflow::Scroll | StackOverflow::Auto => {
                stack.overflow_x_hidden()
            }
        };
        stack = match self.overflow_y {
            StackOverflow::Visible => stack,
            StackOverflow::Hidden | StackOverflow::Scroll | StackOverflow::Auto => {
                stack.overflow_y_hidden()
            }
        };

        // Apply alignment
        stack = match self.align {
            StackAlign::Start => stack.items_start(),
            StackAlign::Center => stack.items_center(),
            StackAlign::End => stack.items_end(),
            StackAlign::Stretch => stack,
            StackAlign::Baseline => stack, // GPUI may not have baseline, fall back to default
        };

        // Apply justify
        stack = match self.justify {
            StackJustify::Start => stack.justify_start(),
            StackJustify::Center => stack.justify_center(),
            StackJustify::End => stack.justify_end(),
            StackJustify::SpaceBetween => stack.justify_between(),
            StackJustify::SpaceAround => stack.justify_around(),
            StackJustify::SpaceEvenly => stack,
        };

        for child in self.children {
            stack = stack.child(child);
        }

        stack
    }
}

impl Default for HStack {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for HStack {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let should_scroll = self.scrollable_on_mobile.unwrap_or(true)
            && self.overflow_x != StackOverflow::Hidden
            && is_mobile(window, cx);

        let design = self
            .design
            .clone()
            .unwrap_or_else(crate::design::neutral_design);
        let stack = self.build_with_design(&design);

        if should_scroll {
            div()
                .id((id, "scroll"))
                .w_full()
                .h_full()
                .overflow_x_scroll()
                .child(stack)
                .into_any_element()
        } else {
            stack.into_any_element()
        }
    }
}

impl IntoElement for HStack {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrollable_on_mobile_defaults_to_none() {
        let stack = HStack::new();
        assert!(stack.scrollable_on_mobile.is_none());
    }

    #[test]
    fn scrollable_on_mobile_builder_sets_flag() {
        let stack = HStack::new().scrollable_on_mobile(false);
        assert_eq!(stack.scrollable_on_mobile, Some(false));
    }
}
