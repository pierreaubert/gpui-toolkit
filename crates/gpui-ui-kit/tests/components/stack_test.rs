//! Stack layout component tests

use gpui_ui_kit::stack::{
    Divider, HStack, Spacer, StackAlign, StackJustify, StackOverflow, StackSize, StackSpacing,
    VStack,
};
use gpui_ui_kit::theme::Theme;

#[test]
fn test_stack_spacing_variants() {
    let spacings = [
        StackSpacing::None,
        StackSpacing::Xs,
        StackSpacing::Sm,
        StackSpacing::Md,
        StackSpacing::Lg,
        StackSpacing::Xl,
        StackSpacing::Xxl,
        StackSpacing::Custom(gpui::px(12.0)),
    ];
    for spacing in &spacings {
        let _copy = *spacing;
    }
}

#[test]
fn test_stack_spacing_default() {
    let spacing = StackSpacing::default();
    assert_eq!(spacing, StackSpacing::Md);
}

#[test]
fn test_stack_align_variants() {
    let aligns = [
        StackAlign::Start,
        StackAlign::Center,
        StackAlign::End,
        StackAlign::Stretch,
        StackAlign::Baseline,
    ];
    for align in &aligns {
        let _copy = *align;
    }
}

#[test]
fn test_stack_align_default() {
    let align = StackAlign::default();
    assert_eq!(align, StackAlign::Center);
}

#[test]
fn test_stack_justify_variants() {
    let justifies = [
        StackJustify::Start,
        StackJustify::Center,
        StackJustify::End,
        StackJustify::SpaceBetween,
        StackJustify::SpaceAround,
        StackJustify::SpaceEvenly,
    ];
    for justify in &justifies {
        let _copy = *justify;
    }
}

#[test]
fn test_stack_justify_default() {
    let justify = StackJustify::default();
    assert_eq!(justify, StackJustify::Start);
}

#[test]
fn test_stack_overflow_variants() {
    let overflows = [
        StackOverflow::Visible,
        StackOverflow::Hidden,
        StackOverflow::Scroll,
        StackOverflow::Auto,
    ];
    for overflow in &overflows {
        let _copy = *overflow;
    }
}

#[test]
fn test_stack_overflow_default() {
    let overflow = StackOverflow::default();
    assert_eq!(overflow, StackOverflow::Visible);
}

#[test]
fn test_stack_size_variants() {
    let sizes = [
        StackSize::Auto,
        StackSize::Full,
        StackSize::Fixed(gpui::px(200.0)),
        StackSize::Fraction(0.5),
    ];
    for size in &sizes {
        let _copy = *size;
    }
}

#[test]
fn test_vstack_creation() {
    let stack = VStack::new();
    let _ = stack;
}

#[test]
fn test_vstack_configuration() {
    let stack = VStack::new()
        .spacing(StackSpacing::Lg)
        .align(StackAlign::Stretch)
        .justify(StackJustify::SpaceBetween)
        .width(StackSize::Full)
        .height(StackSize::Fixed(gpui::px(400.0)))
        .overflow(StackOverflow::Hidden);

    let _ = stack;
}

#[test]
fn test_vstack_flex_methods() {
    let stack = VStack::new()
        .full()
        .grow(1.0)
        .shrink(0.0)
        .basis(gpui::px(100.0))
        .flex_1();

    let _ = stack;
}

#[test]
fn test_vstack_min_max() {
    let stack = VStack::new()
        .min_w(gpui::px(100.0))
        .min_h(gpui::px(50.0))
        .max_w(gpui::px(800.0))
        .max_h(gpui::px(600.0));

    let _ = stack;
}

#[test]
fn test_vstack_overflow_axes() {
    let stack = VStack::new()
        .overflow_x(StackOverflow::Hidden)
        .overflow_y(StackOverflow::Scroll);

    let _ = stack;
}

#[test]
fn test_hstack_creation() {
    let stack = HStack::new();
    let _ = stack;
}

#[test]
fn test_hstack_configuration() {
    let stack = HStack::new()
        .spacing(StackSpacing::Sm)
        .align(StackAlign::End)
        .justify(StackJustify::Center)
        .width(StackSize::Full)
        .wrap(true);

    let _ = stack;
}

#[test]
fn test_spacer_creation() {
    let spacer = Spacer::new();
    let _ = spacer;
}

#[test]
fn test_divider_horizontal() {
    let divider = Divider::new().id("h-div");
    let _ = divider;
}

#[test]
fn test_divider_vertical() {
    let divider = Divider::vertical().id("v-div");
    let _ = divider;
}

#[test]
fn test_divider_configuration() {
    let divider = Divider::new()
        .id("styled-div")
        .color(gpui::rgba(0xCCCCCCFF))
        .hover_color(gpui::rgba(0xFFFFFFFF))
        .thickness(gpui::px(2.0))
        .interactive();

    let _ = divider;
}

#[test]
fn test_divider_uses_theme_border_color() {
    // Forest theme has border = 0x3a4a35, different from hardcoded 0x3a3a3a
    let theme = Theme::forest();
    // When no explicit color is set, build_with_theme should use theme.border
    // Currently it hardcodes rgb(0x3a3a3a), so this test will fail
    let divider = Divider::new().id("test-div");
    // We can't inspect the built Div's bg color directly, but we can verify
    // the theme's border color doesn't match the hardcoded fallback
    let hardcoded = gpui::rgb(0x3a3a3a);
    assert_ne!(
        hardcoded, theme.border,
        "Precondition: forest border differs from hardcoded"
    );
    // The build_with_theme currently ignores the theme and uses hardcoded 0x3a3a3a
    // This will pass once we fix build_with_theme to use theme.border
    // For now we test via a getter that exposes the resolved color
    let resolved = divider.resolve_color(&theme);
    assert_eq!(
        resolved, theme.border,
        "Divider default color should come from theme.border"
    );
}
