use gpui::prelude::FluentBuilder;
use gpui::*;
#[cfg(all(test, feature = "builder"))]
use gpui_builder::{Axis, SolvedNode};
use gpui_design::{DesignExt, DesignSystem};
#[cfg(feature = "builder")]
use std::cell::RefCell;

/// Default MiniApp shell that applies designer defaults and solves the content
/// slot with gpui-builder when the default `builder` feature is enabled.
pub(super) struct MiniAppShell {
    pub(super) inner: AnyView,
    pub(super) scrollable: bool,
}

#[cfg(feature = "builder")]
#[derive(Clone, Copy, PartialEq)]
struct ContentSizeCache {
    window_size: (f32, f32),
    min_content_width: f32,
    content_size: (f32, f32),
}

#[cfg(feature = "builder")]
thread_local! {
    static CONTENT_SIZE_CACHE: RefCell<Option<ContentSizeCache>> = const { RefCell::new(None) };
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
            children: vec![SolvedNode {
                id: "content",
                width,
                height,
                visible: true,
                active_tier: None,
                collapse_label: None,
                resolved_axis: None,
                children: Vec::new(),
            }],
        }
    }

    #[cfg(feature = "builder")]
    pub(super) fn content_size(width: f32, height: f32, design: &DesignSystem) -> (f32, f32) {
        let min_content_width = design.spacing.grid_unit.max(0.0);
        if let Some(cache) = CONTENT_SIZE_CACHE.with(|c| *c.borrow())
            && cache.window_size == (width, height)
            && cache.min_content_width == min_content_width
        {
            return cache.content_size;
        }
        // The shell layout is trivial: the single content slot fills the root.
        let result = (width, height);
        CONTENT_SIZE_CACHE.with(|c| {
            *c.borrow_mut() = Some(ContentSizeCache {
                window_size: (width, height),
                min_content_width,
                content_size: result,
            });
        });
        result
    }

    #[cfg(not(feature = "builder"))]
    pub(super) fn content_size(width: f32, height: f32, _design: &DesignSystem) -> (f32, f32) {
        (width, height)
    }

    #[cfg(test)]
    #[cfg(feature = "builder")]
    pub(super) fn clear_content_size_cache() {
        CONTENT_SIZE_CACHE.with(|c| *c.borrow_mut() = None);
    }

    #[cfg(test)]
    #[cfg(feature = "builder")]
    pub(super) fn content_size_cache_is_populated() -> bool {
        CONTENT_SIZE_CACHE.with(|c| c.borrow().is_some())
    }
}

impl Render for MiniAppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.bounds();
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let design = cx.design();
        let (content_width, content_height) = Self::content_size(width, height, &design);

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
