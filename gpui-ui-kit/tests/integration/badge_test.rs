//! Integration tests for Badge component
//!
//! Tests the Badge and BadgeDot components including:
//! - All variants (Default, Primary, Success, Warning, Error, Info)
//! - All sizes (Sm, Md, Lg)
//! - Rounded (pill) shape
//! - With icon
//! - BadgeDot variants

use gpui::{Context, TestAppContext, Window, div, prelude::*, px};
use gpui_ui_kit::badge::{Badge, BadgeDot, BadgeSize, BadgeVariant};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct BadgeTestView;

impl Render for BadgeTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Badge::new("New"))
    }
}

#[gpui::test]
async fn test_badge_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| BadgeTestView);
}

// ============================================================================
// Variant Tests
// ============================================================================

#[gpui::test]
async fn test_badge_default_variant(cx: &mut TestAppContext) {
    struct DefaultVariantView;

    impl Render for DefaultVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Default").variant(BadgeVariant::Default))
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultVariantView);
}

#[gpui::test]
async fn test_badge_primary_variant(cx: &mut TestAppContext) {
    struct PrimaryVariantView;

    impl Render for PrimaryVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Primary").variant(BadgeVariant::Primary))
        }
    }

    let _window = cx.add_window(|_window, _cx| PrimaryVariantView);
}

#[gpui::test]
async fn test_badge_success_variant(cx: &mut TestAppContext) {
    struct SuccessVariantView;

    impl Render for SuccessVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Success").variant(BadgeVariant::Success))
        }
    }

    let _window = cx.add_window(|_window, _cx| SuccessVariantView);
}

#[gpui::test]
async fn test_badge_warning_variant(cx: &mut TestAppContext) {
    struct WarningVariantView;

    impl Render for WarningVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Warning").variant(BadgeVariant::Warning))
        }
    }

    let _window = cx.add_window(|_window, _cx| WarningVariantView);
}

#[gpui::test]
async fn test_badge_error_variant(cx: &mut TestAppContext) {
    struct ErrorVariantView;

    impl Render for ErrorVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Error").variant(BadgeVariant::Error))
        }
    }

    let _window = cx.add_window(|_window, _cx| ErrorVariantView);
}

#[gpui::test]
async fn test_badge_info_variant(cx: &mut TestAppContext) {
    struct InfoVariantView;

    impl Render for InfoVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Info").variant(BadgeVariant::Info))
        }
    }

    let _window = cx.add_window(|_window, _cx| InfoVariantView);
}

#[gpui::test]
async fn test_badge_all_variants(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(Badge::new("Default").variant(BadgeVariant::Default))
                .child(Badge::new("Primary").variant(BadgeVariant::Primary))
                .child(Badge::new("Success").variant(BadgeVariant::Success))
                .child(Badge::new("Warning").variant(BadgeVariant::Warning))
                .child(Badge::new("Error").variant(BadgeVariant::Error))
                .child(Badge::new("Info").variant(BadgeVariant::Info))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_badge_size_sm(cx: &mut TestAppContext) {
    struct SmSizeView;

    impl Render for SmSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Small").size(BadgeSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmSizeView);
}

#[gpui::test]
async fn test_badge_size_md(cx: &mut TestAppContext) {
    struct MdSizeView;

    impl Render for MdSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Medium").size(BadgeSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdSizeView);
}

#[gpui::test]
async fn test_badge_size_lg(cx: &mut TestAppContext) {
    struct LgSizeView;

    impl Render for LgSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Large").size(BadgeSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgSizeView);
}

#[gpui::test]
async fn test_badge_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(Badge::new("Sm").size(BadgeSize::Sm))
                .child(Badge::new("Md").size(BadgeSize::Md))
                .child(Badge::new("Lg").size(BadgeSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Rounded Tests
// ============================================================================

#[gpui::test]
async fn test_badge_rounded(cx: &mut TestAppContext) {
    struct RoundedView;

    impl Render for RoundedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Pill").rounded(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| RoundedView);
}

#[gpui::test]
async fn test_badge_not_rounded(cx: &mut TestAppContext) {
    struct NotRoundedView;

    impl Render for NotRoundedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("Square").rounded(false))
        }
    }

    let _window = cx.add_window(|_window, _cx| NotRoundedView);
}

#[gpui::test]
async fn test_badge_rounded_comparison(cx: &mut TestAppContext) {
    struct RoundedComparisonView;

    impl Render for RoundedComparisonView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(Badge::new("Square").rounded(false))
                .child(Badge::new("Pill").rounded(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| RoundedComparisonView);
}

// ============================================================================
// Icon Tests
// ============================================================================

#[gpui::test]
async fn test_badge_with_icon(cx: &mut TestAppContext) {
    struct IconBadgeView;

    impl Render for IconBadgeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Badge::new("New").icon("⭐"))
        }
    }

    let _window = cx.add_window(|_window, _cx| IconBadgeView);
}

#[gpui::test]
async fn test_badge_icon_with_variant(cx: &mut TestAppContext) {
    struct IconVariantView;

    impl Render for IconVariantView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(
                    Badge::new("Success")
                        .variant(BadgeVariant::Success)
                        .icon("✓"),
                )
                .child(Badge::new("Error").variant(BadgeVariant::Error).icon("✗"))
                .child(
                    Badge::new("Warning")
                        .variant(BadgeVariant::Warning)
                        .icon("⚠"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| IconVariantView);
}

// ============================================================================
// BadgeDot Tests
// ============================================================================

#[gpui::test]
async fn test_badge_dot_renders(cx: &mut TestAppContext) {
    struct BadgeDotView;

    impl Render for BadgeDotView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(BadgeDot::new())
        }
    }

    let _window = cx.add_window(|_window, _cx| BadgeDotView);
}

#[gpui::test]
async fn test_badge_dot_all_variants(cx: &mut TestAppContext) {
    struct BadgeDotVariantsView;

    impl Render for BadgeDotVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(BadgeDot::new().variant(BadgeVariant::Default))
                .child(BadgeDot::new().variant(BadgeVariant::Primary))
                .child(BadgeDot::new().variant(BadgeVariant::Success))
                .child(BadgeDot::new().variant(BadgeVariant::Warning))
                .child(BadgeDot::new().variant(BadgeVariant::Error))
                .child(BadgeDot::new().variant(BadgeVariant::Info))
        }
    }

    let _window = cx.add_window(|_window, _cx| BadgeDotVariantsView);
}

#[gpui::test]
async fn test_badge_dot_custom_size(cx: &mut TestAppContext) {
    struct BadgeDotSizeView;

    impl Render for BadgeDotSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(BadgeDot::new().size(px(4.0)))
                .child(BadgeDot::new().size(px(8.0)))
                .child(BadgeDot::new().size(px(12.0)))
                .child(BadgeDot::new().size(px(16.0)))
        }
    }

    let _window = cx.add_window(|_window, _cx| BadgeDotSizeView);
}

#[gpui::test]
async fn test_badge_dot_default(cx: &mut TestAppContext) {
    struct BadgeDotDefaultView;

    impl Render for BadgeDotDefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(BadgeDot::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| BadgeDotDefaultView);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_badge_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;

    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Badge::new("Featured")
                    .variant(BadgeVariant::Success)
                    .size(BadgeSize::Lg)
                    .rounded(true)
                    .icon("★"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}

#[gpui::test]
async fn test_badge_numeric_labels(cx: &mut TestAppContext) {
    struct NumericLabelsView;

    impl Render for NumericLabelsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(Badge::new("1").variant(BadgeVariant::Error).rounded(true))
                .child(Badge::new("99").variant(BadgeVariant::Error).rounded(true))
                .child(Badge::new("99+").variant(BadgeVariant::Error).rounded(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| NumericLabelsView);
}
