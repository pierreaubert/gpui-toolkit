//! Stack layout components
//!
//! Vertical and horizontal stack layouts with spacing.
//! Behaves like CSS flexbox with responsive resizing support.

use gpui::prelude::*;
use gpui::*;

/// Spacing values
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackSpacing {
    /// No spacing
    None,
    /// Extra small (2px)
    Xs,
    /// Small (4px)
    Sm,
    /// Medium (8px, default)
    #[default]
    Md,
    /// Large (16px)
    Lg,
    /// Extra large (24px)
    Xl,
    /// 2X large (32px)
    Xxl,
    /// Custom spacing
    Custom(Pixels),
}

impl StackSpacing {
    fn to_pixels(&self) -> Pixels {
        match self {
            StackSpacing::None => px(0.0),
            StackSpacing::Xs => px(2.0),
            StackSpacing::Sm => px(4.0),
            StackSpacing::Md => px(8.0),
            StackSpacing::Lg => px(16.0),
            StackSpacing::Xl => px(24.0),
            StackSpacing::Xxl => px(32.0),
            StackSpacing::Custom(p) => *p,
        }
    }
}

/// Alignment options (cross-axis alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackAlign {
    /// Align to start
    Start,
    /// Center alignment (default)
    #[default]
    Center,
    /// Align to end
    End,
    /// Stretch to fill
    Stretch,
    /// Baseline alignment (useful for text)
    Baseline,
}

/// Justify options (main-axis alignment)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackJustify {
    /// Justify to start (default)
    #[default]
    Start,
    /// Center justify
    Center,
    /// Justify to end
    End,
    /// Space between items
    SpaceBetween,
    /// Space around items
    SpaceAround,
    /// Space evenly
    SpaceEvenly,
}

/// Overflow handling options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackOverflow {
    /// Show all content (default)
    #[default]
    Visible,
    /// Hide overflow
    Hidden,
    /// Scroll when needed
    Scroll,
    /// Always show scrollbar
    Auto,
}

/// Size specification for stack dimensions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackSize {
    /// Auto size (fit content)
    Auto,
    /// Fill available space (100%)
    Full,
    /// Fixed size in pixels
    Fixed(Pixels),
    /// Fraction of available space (0.0 to 1.0)
    Fraction(f32),
}

/// A vertical stack (column) layout
///
/// Behaves like CSS `display: flex; flex-direction: column` with responsive sizing support.
pub struct VStack {
    children: Vec<AnyElement>,
    spacing: StackSpacing,
    align: StackAlign,
    justify: StackJustify,
    width: Option<StackSize>,
    height: Option<StackSize>,
    flex_grow: Option<f32>,
    flex_shrink: Option<f32>,
    flex_basis: Option<Pixels>,
    overflow_x: StackOverflow,
    overflow_y: StackOverflow,
    min_width: Option<Pixels>,
    min_height: Option<Pixels>,
    max_width: Option<Pixels>,
    max_height: Option<Pixels>,
}

impl VStack {
    /// Create a new vertical stack
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            spacing: StackSpacing::default(),
            align: StackAlign::Stretch,
            justify: StackJustify::default(),
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

    /// Set cross-axis alignment (horizontal for VStack)
    pub fn align(mut self, align: StackAlign) -> Self {
        self.align = align;
        self
    }

    /// Set main-axis alignment (vertical for VStack)
    pub fn justify(mut self, justify: StackJustify) -> Self {
        self.justify = justify;
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

    /// Build into element
    pub fn build(self) -> Div {
        let mut stack = div().flex().flex_col().gap(self.spacing.to_pixels());

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
            stack = stack.flex_grow();
            if grow != 1.0 {
                // GPUI uses flex_grow() for grow=1, for other values we'd need custom styling
                // For now, flex_grow() sets grow to 1
            }
        }
        if let Some(_shrink) = self.flex_shrink {
            stack = stack.flex_shrink();
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

impl Default for VStack {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for VStack {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        self.build()
    }
}

/// A horizontal stack (row) layout
///
/// Behaves like CSS `display: flex; flex-direction: row` with responsive sizing support.
pub struct HStack {
    children: Vec<AnyElement>,
    spacing: StackSpacing,
    align: StackAlign,
    justify: StackJustify,
    wrap: bool,
    width: Option<StackSize>,
    height: Option<StackSize>,
    flex_grow: Option<f32>,
    flex_shrink: Option<f32>,
    flex_basis: Option<Pixels>,
    overflow_x: StackOverflow,
    overflow_y: StackOverflow,
    min_width: Option<Pixels>,
    min_height: Option<Pixels>,
    max_width: Option<Pixels>,
    max_height: Option<Pixels>,
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

    /// Build into element
    pub fn build(self) -> Div {
        let mut stack = div().flex().gap(self.spacing.to_pixels());

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
            stack = stack.flex_grow();
            if grow != 1.0 {
                // GPUI uses flex_grow() for grow=1, for other values we'd need custom styling
                // For now, flex_grow() sets grow to 1
            }
        }
        if let Some(_shrink) = self.flex_shrink {
            stack = stack.flex_shrink();
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

impl IntoElement for HStack {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        self.build()
    }
}

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

/// A divider line
pub struct Divider {
    id: Option<SharedString>,
    vertical: bool,
    color: Option<Rgba>,
    hover_color: Option<Rgba>,
    thickness: Option<Pixels>,
    interactive: bool,
}

impl Divider {
    /// Create a new horizontal divider
    pub fn new() -> Self {
        Self {
            id: None,
            vertical: false,
            color: None,
            hover_color: None,
            thickness: None,
            interactive: false,
        }
    }

    /// Create a vertical divider
    pub fn vertical() -> Self {
        Self {
            id: None,
            vertical: true,
            color: None,
            hover_color: None,
            thickness: None,
            interactive: false,
        }
    }

    /// Set an ID for the divider (required for interactive dividers)
    pub fn id(mut self, id: impl Into<SharedString>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set custom color
    pub fn color(mut self, color: Rgba) -> Self {
        self.color = Some(color);
        self
    }

    /// Set hover color (for interactive dividers)
    pub fn hover_color(mut self, color: Rgba) -> Self {
        self.hover_color = Some(color);
        self
    }

    /// Set custom thickness
    pub fn thickness(mut self, thickness: Pixels) -> Self {
        self.thickness = Some(thickness);
        self
    }

    /// Make this an interactive resize divider
    pub fn interactive(mut self) -> Self {
        self.interactive = true;
        self
    }

    /// Build into a stateful element that can have handlers attached
    /// Use this when you need to add event handlers (e.g., for resize dividers)
    pub fn build(self) -> Stateful<Div> {
        let color = self.color.unwrap_or(rgb(0x3a3a3a));
        let id = self.id.unwrap_or_else(|| SharedString::from("divider"));

        let base = if self.vertical {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().id(id).w(thickness).h_full().bg(color)
        } else {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().id(id).h(thickness).w_full().bg(color)
        };

        if self.interactive {
            let hover_color = self.hover_color.unwrap_or(rgb(0x007acc));
            let cursor = if self.vertical {
                gpui::CursorStyle::ResizeLeftRight
            } else {
                gpui::CursorStyle::ResizeUpDown
            };
            base.cursor(cursor)
                .hover(move |style| style.bg(hover_color))
        } else {
            base
        }
    }

    /// Build into a non-stateful element (for simple visual dividers)
    pub fn build_simple(self) -> Div {
        let color = self.color.unwrap_or(rgb(0x3a3a3a));

        if self.vertical {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().w(thickness).h_full().bg(color)
        } else {
            let thickness = self.thickness.unwrap_or(px(1.0));
            div().h(thickness).w_full().bg(color)
        }
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoElement for Divider {
    type Element = Stateful<Div>;

    fn into_element(self) -> Self::Element {
        self.build()
    }
}
