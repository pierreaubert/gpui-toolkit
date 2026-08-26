use gpui::prelude::FluentBuilder;
use gpui::*;
#[cfg(all(test, feature = "builder"))]
use gpui_builder::{Axis, SolvedNode};

/// Default MiniApp shell that applies designer defaults and solves the content
/// slot with gpui-builder when the default `builder` feature is enabled.
pub(super) struct MiniAppShell {
    pub(super) inner: AnyView,
    pub(super) scrollable: bool,
}

impl MiniAppShell {
    #[cfg(all(test, feature = "builder"))]
    pub(super) fn solve_content_layout(
        width: f32,
        height: f32,
        _min_content_width: f32,
    ) -> SolvedNode<'static> {
        // The shell uses a single content slot that fills the root container.
        // Construct the solved tree directly instead of running the full solver
        // so the result can own static string references.
        SolvedNode {
            id: "root",
            width,
            height,
            visible: true,
            active_tier: None,
            collapse_label: None,
            resolved_axis: Some(Axis::Vertical),
            divider_size: 0.0,
            children: vec![SolvedNode {
                id: "content",
                width,
                height,
                visible: true,
                active_tier: None,
                collapse_label: None,
                resolved_axis: None,
                divider_size: 0.0,
                children: Vec::new(),
            }],
        }
    }

    pub(super) fn content_size(width: f32, height: f32) -> (f32, f32) {
        (width, height)
    }
}

impl Render for MiniAppShell {
    fn render(&mut self, window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let (content_width, content_height) = Self::content_size(width, height);

        div().id("miniapp-shell").size_full().child(
            div()
                .id(if self.scrollable {
                    "miniapp-scroll-container"
                } else {
                    "miniapp-content-container"
                })
                .w(px(content_width))
                .h(px(content_height))
                .when(self.scrollable, |el| el.overflow_y_scroll())
                .child(self.inner.clone()),
        )
    }
}
