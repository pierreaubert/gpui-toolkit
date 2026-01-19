//! Integration tests for Stack layout components
//!
//! Tests the stack layout components including:
//! - VStack (vertical stack)
//! - HStack (horizontal stack)
//! - Spacer
//! - Divider
//! - Spacing variants
//! - Alignment options
//! - Justify options
//! - Flex properties
//! - Overflow handling
//! - Size constraints

use gpui::{Context, Rgba, TestAppContext, Window, div, prelude::*, px};
use gpui_ui_kit::stack::{
    Divider, HStack, Spacer, StackAlign, StackJustify, StackOverflow, StackSize, StackSpacing,
    VStack,
};

// ============================================================================
// VStack Tests
// ============================================================================

struct VStackTestView;

impl Render for VStackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        VStack::new()
            .child(div().child("Item 1"))
            .child(div().child("Item 2"))
            .child(div().child("Item 3"))
    }
}

#[gpui::test]
async fn test_vstack_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| VStackTestView);
}

#[gpui::test]
async fn test_vstack_with_spacing(cx: &mut TestAppContext) {
    struct SpacingView;

    impl Render for SpacingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VStack::new()
                        .spacing(StackSpacing::None)
                        .child(div().child("No space"))
                        .child(div().child("Between")),
                )
                .child(
                    VStack::new()
                        .spacing(StackSpacing::Xs)
                        .child(div().child("XS"))
                        .child(div().child("Space")),
                )
                .child(
                    VStack::new()
                        .spacing(StackSpacing::Sm)
                        .child(div().child("SM"))
                        .child(div().child("Space")),
                )
                .child(
                    VStack::new()
                        .spacing(StackSpacing::Md)
                        .child(div().child("MD"))
                        .child(div().child("Space")),
                )
                .child(
                    VStack::new()
                        .spacing(StackSpacing::Lg)
                        .child(div().child("LG"))
                        .child(div().child("Space")),
                )
                .child(
                    VStack::new()
                        .spacing(StackSpacing::Xl)
                        .child(div().child("XL"))
                        .child(div().child("Space")),
                )
                .child(
                    VStack::new()
                        .spacing(StackSpacing::Xxl)
                        .child(div().child("XXL"))
                        .child(div().child("Space")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SpacingView);
}

#[gpui::test]
async fn test_vstack_custom_spacing(cx: &mut TestAppContext) {
    struct CustomSpacingView;

    impl Render for CustomSpacingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .spacing(StackSpacing::Custom(px(12.0)))
                .child(div().child("Custom"))
                .child(div().child("Spacing"))
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomSpacingView);
}

#[gpui::test]
async fn test_vstack_alignment(cx: &mut TestAppContext) {
    struct AlignmentView;

    impl Render for AlignmentView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VStack::new()
                        .align(StackAlign::Start)
                        .width(StackSize::Fixed(px(100.0)))
                        .child(div().child("Start")),
                )
                .child(
                    VStack::new()
                        .align(StackAlign::Center)
                        .width(StackSize::Fixed(px(100.0)))
                        .child(div().child("Center")),
                )
                .child(
                    VStack::new()
                        .align(StackAlign::End)
                        .width(StackSize::Fixed(px(100.0)))
                        .child(div().child("End")),
                )
                .child(
                    VStack::new()
                        .align(StackAlign::Stretch)
                        .width(StackSize::Fixed(px(100.0)))
                        .child(div().child("Stretch")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| AlignmentView);
}

#[gpui::test]
async fn test_vstack_justify(cx: &mut TestAppContext) {
    struct JustifyView;

    impl Render for JustifyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VStack::new()
                        .justify(StackJustify::Start)
                        .height(StackSize::Fixed(px(200.0)))
                        .child(div().child("Start"))
                        .child(div().child("Justify")),
                )
                .child(
                    VStack::new()
                        .justify(StackJustify::Center)
                        .height(StackSize::Fixed(px(200.0)))
                        .child(div().child("Center"))
                        .child(div().child("Justify")),
                )
                .child(
                    VStack::new()
                        .justify(StackJustify::End)
                        .height(StackSize::Fixed(px(200.0)))
                        .child(div().child("End"))
                        .child(div().child("Justify")),
                )
                .child(
                    VStack::new()
                        .justify(StackJustify::SpaceBetween)
                        .height(StackSize::Fixed(px(200.0)))
                        .child(div().child("Space"))
                        .child(div().child("Between")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| JustifyView);
}

#[gpui::test]
async fn test_vstack_size_options(cx: &mut TestAppContext) {
    struct SizeView;

    impl Render for SizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .h(px(300.0))
                .child(
                    VStack::new()
                        .width(StackSize::Fixed(px(100.0)))
                        .height(StackSize::Full)
                        .child(div().child("Fixed width, full height")),
                )
                .child(VStack::new().full().child(div().child("Full size")))
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeView);
}

#[gpui::test]
async fn test_vstack_flex_properties(cx: &mut TestAppContext) {
    struct FlexView;

    impl Render for FlexView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .h(px(300.0))
                .child(VStack::new().flex_1().child(div().child("Flex 1")))
                .child(VStack::new().grow(2.0).child(div().child("Grow 2")))
        }
    }

    let _window = cx.add_window(|_window, _cx| FlexView);
}

#[gpui::test]
async fn test_vstack_overflow(cx: &mut TestAppContext) {
    struct OverflowView;

    impl Render for OverflowView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VStack::new()
                        .overflow(StackOverflow::Hidden)
                        .height(StackSize::Fixed(px(50.0)))
                        .child(div().child("Hidden overflow"))
                        .child(div().child("More content"))
                        .child(div().child("Even more")),
                )
                .child(
                    VStack::new()
                        .overflow(StackOverflow::Visible)
                        .height(StackSize::Fixed(px(50.0)))
                        .child(div().child("Visible overflow"))
                        .child(div().child("More content")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| OverflowView);
}

#[gpui::test]
async fn test_vstack_min_max_constraints(cx: &mut TestAppContext) {
    struct ConstraintsView;

    impl Render for ConstraintsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .min_w(px(100.0))
                .max_w(px(300.0))
                .min_h(px(50.0))
                .max_h(px(200.0))
                .child(div().child("Constrained"))
        }
    }

    let _window = cx.add_window(|_window, _cx| ConstraintsView);
}

#[gpui::test]
async fn test_vstack_children_method(cx: &mut TestAppContext) {
    struct ChildrenView;

    impl Render for ChildrenView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let items = vec!["A", "B", "C", "D"];
            VStack::new().children(items.iter().map(|item| div().child(*item)))
        }
    }

    let _window = cx.add_window(|_window, _cx| ChildrenView);
}

// ============================================================================
// HStack Tests
// ============================================================================

struct HStackTestView;

impl Render for HStackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        HStack::new()
            .child(div().child("Left"))
            .child(div().child("Center"))
            .child(div().child("Right"))
    }
}

#[gpui::test]
async fn test_hstack_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| HStackTestView);
}

#[gpui::test]
async fn test_hstack_with_spacing(cx: &mut TestAppContext) {
    struct HSpacingView;

    impl Render for HSpacingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .spacing(StackSpacing::Lg)
                .child(div().child("A"))
                .child(div().child("B"))
                .child(div().child("C"))
        }
    }

    let _window = cx.add_window(|_window, _cx| HSpacingView);
}

#[gpui::test]
async fn test_hstack_wrap(cx: &mut TestAppContext) {
    struct WrapView;

    impl Render for WrapView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .wrap(true)
                .width(StackSize::Fixed(px(200.0)))
                .children((1..=10).map(|i| div().w(px(50.0)).child(format!("{}", i))))
        }
    }

    let _window = cx.add_window(|_window, _cx| WrapView);
}

#[gpui::test]
async fn test_hstack_alignment(cx: &mut TestAppContext) {
    struct HAlignView;

    impl Render for HAlignView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    HStack::new()
                        .align(StackAlign::Start)
                        .height(StackSize::Fixed(px(60.0)))
                        .child(div().h(px(20.0)).child("Start"))
                        .child(div().h(px(40.0)).child("Align")),
                )
                .child(
                    HStack::new()
                        .align(StackAlign::Center)
                        .height(StackSize::Fixed(px(60.0)))
                        .child(div().h(px(20.0)).child("Center"))
                        .child(div().h(px(40.0)).child("Align")),
                )
                .child(
                    HStack::new()
                        .align(StackAlign::End)
                        .height(StackSize::Fixed(px(60.0)))
                        .child(div().h(px(20.0)).child("End"))
                        .child(div().h(px(40.0)).child("Align")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| HAlignView);
}

#[gpui::test]
async fn test_hstack_justify(cx: &mut TestAppContext) {
    struct HJustifyView;

    impl Render for HJustifyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    HStack::new()
                        .justify(StackJustify::SpaceAround)
                        .width(StackSize::Full)
                        .child(div().child("Space"))
                        .child(div().child("Around")),
                )
                .child(
                    HStack::new()
                        .justify(StackJustify::SpaceEvenly)
                        .width(StackSize::Full)
                        .child(div().child("Space"))
                        .child(div().child("Evenly")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| HJustifyView);
}

// ============================================================================
// Spacer Tests
// ============================================================================

#[gpui::test]
async fn test_spacer_renders(cx: &mut TestAppContext) {
    struct SpacerView;

    impl Render for SpacerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .width(StackSize::Full)
                .child(div().child("Left"))
                .child(Spacer::new())
                .child(div().child("Right"))
        }
    }

    let _window = cx.add_window(|_window, _cx| SpacerView);
}

#[gpui::test]
async fn test_spacer_in_vstack(cx: &mut TestAppContext) {
    struct VSpacerView;

    impl Render for VSpacerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .height(StackSize::Fixed(px(200.0)))
                .child(div().child("Top"))
                .child(Spacer::new())
                .child(div().child("Bottom"))
        }
    }

    let _window = cx.add_window(|_window, _cx| VSpacerView);
}

#[gpui::test]
async fn test_multiple_spacers(cx: &mut TestAppContext) {
    struct MultiSpacerView;

    impl Render for MultiSpacerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .width(StackSize::Full)
                .child(div().child("A"))
                .child(Spacer::new())
                .child(div().child("B"))
                .child(Spacer::new())
                .child(div().child("C"))
        }
    }

    let _window = cx.add_window(|_window, _cx| MultiSpacerView);
}

// ============================================================================
// Divider Tests
// ============================================================================

#[gpui::test]
async fn test_horizontal_divider_renders(cx: &mut TestAppContext) {
    struct HDividerView;

    impl Render for HDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .width(StackSize::Fixed(px(200.0)))
                .child(div().child("Above"))
                .child(Divider::new())
                .child(div().child("Below"))
        }
    }

    let _window = cx.add_window(|_window, _cx| HDividerView);
}

#[gpui::test]
async fn test_vertical_divider_renders(cx: &mut TestAppContext) {
    struct VDividerView;

    impl Render for VDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .height(StackSize::Fixed(px(100.0)))
                .child(div().child("Left"))
                .child(Divider::vertical())
                .child(div().child("Right"))
        }
    }

    let _window = cx.add_window(|_window, _cx| VDividerView);
}

#[gpui::test]
async fn test_divider_with_id(cx: &mut TestAppContext) {
    struct IdDividerView;

    impl Render for IdDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .child(div().child("Above"))
                .child(Divider::new().id("my-divider"))
                .child(div().child("Below"))
        }
    }

    let _window = cx.add_window(|_window, _cx| IdDividerView);
}

#[gpui::test]
async fn test_divider_custom_color(cx: &mut TestAppContext) {
    struct ColorDividerView;

    impl Render for ColorDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .child(div().child("Above"))
                .child(Divider::new().color(Rgba {
                    r: 1.0,
                    g: 0.0,
                    b: 0.0,
                    a: 1.0,
                }))
                .child(div().child("Below"))
        }
    }

    let _window = cx.add_window(|_window, _cx| ColorDividerView);
}

#[gpui::test]
async fn test_divider_custom_thickness(cx: &mut TestAppContext) {
    struct ThickDividerView;

    impl Render for ThickDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .child(div().child("Above"))
                .child(Divider::new().thickness(px(4.0)))
                .child(div().child("Below"))
        }
    }

    let _window = cx.add_window(|_window, _cx| ThickDividerView);
}

#[gpui::test]
async fn test_divider_interactive(cx: &mut TestAppContext) {
    struct InteractiveDividerView;

    impl Render for InteractiveDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .height(StackSize::Fixed(px(100.0)))
                .child(div().child("Left"))
                .child(
                    Divider::vertical()
                        .id("interactive-divider")
                        .interactive()
                        .hover_color(Rgba {
                            r: 0.0,
                            g: 0.48,
                            b: 0.8,
                            a: 1.0,
                        }),
                )
                .child(div().child("Right"))
        }
    }

    let _window = cx.add_window(|_window, _cx| InteractiveDividerView);
}

#[gpui::test]
async fn test_divider_build_simple(cx: &mut TestAppContext) {
    struct SimpleDividerView;

    impl Render for SimpleDividerView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .child(div().child("Above"))
                .child(Divider::new().build_simple())
                .child(div().child("Below"))
        }
    }

    let _window = cx.add_window(|_window, _cx| SimpleDividerView);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_empty_vstack(cx: &mut TestAppContext) {
    struct EmptyVStackView;

    impl Render for EmptyVStackView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
        }
    }

    let _window = cx.add_window(|_window, _cx| EmptyVStackView);
}

#[gpui::test]
async fn test_empty_hstack(cx: &mut TestAppContext) {
    struct EmptyHStackView;

    impl Render for EmptyHStackView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
        }
    }

    let _window = cx.add_window(|_window, _cx| EmptyHStackView);
}

#[gpui::test]
async fn test_nested_stacks(cx: &mut TestAppContext) {
    struct NestedStacksView;

    impl Render for NestedStacksView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            VStack::new()
                .child(
                    HStack::new()
                        .child(div().child("A"))
                        .child(div().child("B")),
                )
                .child(
                    HStack::new().child(div().child("C")).child(
                        VStack::new()
                            .child(div().child("D1"))
                            .child(div().child("D2")),
                    ),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| NestedStacksView);
}

#[gpui::test]
async fn test_stack_fraction_size(cx: &mut TestAppContext) {
    struct FractionView;

    impl Render for FractionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            HStack::new()
                .width(StackSize::Full)
                .child(
                    VStack::new()
                        .width(StackSize::Fraction(0.3))
                        .child(div().child("30%")),
                )
                .child(
                    VStack::new()
                        .width(StackSize::Fraction(0.7))
                        .child(div().child("70%")),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| FractionView);
}
