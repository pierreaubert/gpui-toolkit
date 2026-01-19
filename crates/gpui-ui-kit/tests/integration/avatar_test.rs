//! Integration tests for Avatar component
//!
//! Tests the Avatar and AvatarGroup components including:
//! - All sizes (Xs to Xxl)
//! - Shapes (Circle, Square)
//! - Status indicators (Online, Offline, Away, Busy)
//! - Initials generation from names
//! - AvatarGroup with overflow

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::avatar::{Avatar, AvatarGroup, AvatarShape, AvatarSize, AvatarStatus};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct AvatarTestView;

impl Render for AvatarTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Avatar::new().name("Test User"))
    }
}

#[gpui::test]
async fn test_avatar_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| AvatarTestView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_size_xs(cx: &mut TestAppContext) {
    struct XsAvatarView;

    impl Render for XsAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("XS User").size(AvatarSize::Xs))
        }
    }

    let _window = cx.add_window(|_window, _cx| XsAvatarView);
}

#[gpui::test]
async fn test_avatar_size_sm(cx: &mut TestAppContext) {
    struct SmAvatarView;

    impl Render for SmAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("SM User").size(AvatarSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmAvatarView);
}

#[gpui::test]
async fn test_avatar_size_md(cx: &mut TestAppContext) {
    struct MdAvatarView;

    impl Render for MdAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("MD User").size(AvatarSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdAvatarView);
}

#[gpui::test]
async fn test_avatar_size_lg(cx: &mut TestAppContext) {
    struct LgAvatarView;

    impl Render for LgAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("LG User").size(AvatarSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgAvatarView);
}

#[gpui::test]
async fn test_avatar_size_xl(cx: &mut TestAppContext) {
    struct XlAvatarView;

    impl Render for XlAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("XL User").size(AvatarSize::Xl))
        }
    }

    let _window = cx.add_window(|_window, _cx| XlAvatarView);
}

#[gpui::test]
async fn test_avatar_size_xxl(cx: &mut TestAppContext) {
    struct XxlAvatarView;

    impl Render for XxlAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("XXL User").size(AvatarSize::Xxl))
        }
    }

    let _window = cx.add_window(|_window, _cx| XxlAvatarView);
}

#[gpui::test]
async fn test_avatar_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_end()
                .gap_2()
                .child(Avatar::new().name("XS").size(AvatarSize::Xs))
                .child(Avatar::new().name("SM").size(AvatarSize::Sm))
                .child(Avatar::new().name("MD").size(AvatarSize::Md))
                .child(Avatar::new().name("LG").size(AvatarSize::Lg))
                .child(Avatar::new().name("XL").size(AvatarSize::Xl))
                .child(Avatar::new().name("XX").size(AvatarSize::Xxl))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Shape Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_circle_shape(cx: &mut TestAppContext) {
    struct CircleAvatarView;

    impl Render for CircleAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("Circle User").shape(AvatarShape::Circle))
        }
    }

    let _window = cx.add_window(|_window, _cx| CircleAvatarView);
}

#[gpui::test]
async fn test_avatar_square_shape(cx: &mut TestAppContext) {
    struct SquareAvatarView;

    impl Render for SquareAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("Square User").shape(AvatarShape::Square))
        }
    }

    let _window = cx.add_window(|_window, _cx| SquareAvatarView);
}

#[gpui::test]
async fn test_avatar_both_shapes(cx: &mut TestAppContext) {
    struct BothShapesView;

    impl Render for BothShapesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    Avatar::new()
                        .name("Circle")
                        .shape(AvatarShape::Circle)
                        .size(AvatarSize::Lg),
                )
                .child(
                    Avatar::new()
                        .name("Square")
                        .shape(AvatarShape::Square)
                        .size(AvatarSize::Lg),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| BothShapesView);
}

// ============================================================================
// Status Indicator Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_status_online(cx: &mut TestAppContext) {
    struct OnlineAvatarView;

    impl Render for OnlineAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Avatar::new()
                    .name("Online User")
                    .status(AvatarStatus::Online),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| OnlineAvatarView);
}

#[gpui::test]
async fn test_avatar_status_offline(cx: &mut TestAppContext) {
    struct OfflineAvatarView;

    impl Render for OfflineAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Avatar::new()
                    .name("Offline User")
                    .status(AvatarStatus::Offline),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| OfflineAvatarView);
}

#[gpui::test]
async fn test_avatar_status_away(cx: &mut TestAppContext) {
    struct AwayAvatarView;

    impl Render for AwayAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("Away User").status(AvatarStatus::Away))
        }
    }

    let _window = cx.add_window(|_window, _cx| AwayAvatarView);
}

#[gpui::test]
async fn test_avatar_status_busy(cx: &mut TestAppContext) {
    struct BusyAvatarView;

    impl Render for BusyAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("Busy User").status(AvatarStatus::Busy))
        }
    }

    let _window = cx.add_window(|_window, _cx| BusyAvatarView);
}

#[gpui::test]
async fn test_avatar_all_statuses(cx: &mut TestAppContext) {
    struct AllStatusesView;

    impl Render for AllStatusesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    Avatar::new()
                        .name("Online")
                        .status(AvatarStatus::Online)
                        .size(AvatarSize::Lg),
                )
                .child(
                    Avatar::new()
                        .name("Offline")
                        .status(AvatarStatus::Offline)
                        .size(AvatarSize::Lg),
                )
                .child(
                    Avatar::new()
                        .name("Away")
                        .status(AvatarStatus::Away)
                        .size(AvatarSize::Lg),
                )
                .child(
                    Avatar::new()
                        .name("Busy")
                        .status(AvatarStatus::Busy)
                        .size(AvatarSize::Lg),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllStatusesView);
}

// ============================================================================
// Initials Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_single_name(cx: &mut TestAppContext) {
    struct SingleNameView;

    impl Render for SingleNameView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("Alice").size(AvatarSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| SingleNameView);
}

#[gpui::test]
async fn test_avatar_two_names(cx: &mut TestAppContext) {
    struct TwoNamesView;

    impl Render for TwoNamesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::new().name("John Doe").size(AvatarSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| TwoNamesView);
}

#[gpui::test]
async fn test_avatar_three_names(cx: &mut TestAppContext) {
    struct ThreeNamesView;

    impl Render for ThreeNamesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            // Should only show first two initials
            div().child(Avatar::new().name("John Middle Doe").size(AvatarSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| ThreeNamesView);
}

#[gpui::test]
async fn test_avatar_no_name(cx: &mut TestAppContext) {
    struct NoNameView;

    impl Render for NoNameView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            // Should show "?" as fallback
            div().child(Avatar::new().size(AvatarSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| NoNameView);
}

// ============================================================================
// Image Source Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_with_src(cx: &mut TestAppContext) {
    struct SrcAvatarView;

    impl Render for SrcAvatarView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            // Image loading shows initials as fallback
            div().child(
                Avatar::new()
                    .name("Image User")
                    .src("https://example.com/avatar.png")
                    .size(AvatarSize::Lg),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SrcAvatarView);
}

// ============================================================================
// AvatarGroup Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_group_basic(cx: &mut TestAppContext) {
    struct GroupBasicView;

    impl Render for GroupBasicView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(AvatarGroup::new().avatars(vec![
                Avatar::new().name("Alice"),
                Avatar::new().name("Bob"),
                Avatar::new().name("Charlie"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| GroupBasicView);
}

#[gpui::test]
async fn test_avatar_group_with_overflow(cx: &mut TestAppContext) {
    struct GroupOverflowView;

    impl Render for GroupOverflowView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                AvatarGroup::new()
                    .avatars(vec![
                        Avatar::new().name("Alice"),
                        Avatar::new().name("Bob"),
                        Avatar::new().name("Charlie"),
                        Avatar::new().name("Diana"),
                        Avatar::new().name("Eve"),
                        Avatar::new().name("Frank"),
                    ])
                    .max_display(4), // Should show +2
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| GroupOverflowView);
}

#[gpui::test]
async fn test_avatar_group_custom_size(cx: &mut TestAppContext) {
    struct GroupSizeView;

    impl Render for GroupSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                AvatarGroup::new()
                    .avatars(vec![
                        Avatar::new().name("Alice"),
                        Avatar::new().name("Bob"),
                        Avatar::new().name("Charlie"),
                    ])
                    .size(AvatarSize::Lg),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| GroupSizeView);
}

// ============================================================================
// Default Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_default(cx: &mut TestAppContext) {
    struct DefaultView;

    impl Render for DefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Avatar::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultView);
}

#[gpui::test]
async fn test_avatar_group_default(cx: &mut TestAppContext) {
    struct GroupDefaultView;

    impl Render for GroupDefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(AvatarGroup::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| GroupDefaultView);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_avatar_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;

    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Avatar::new()
                    .name("Full Featured User")
                    .size(AvatarSize::Xl)
                    .shape(AvatarShape::Circle)
                    .status(AvatarStatus::Online),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}
