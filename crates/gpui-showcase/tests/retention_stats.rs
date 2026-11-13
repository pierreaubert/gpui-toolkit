//! Asserting gate for fork view/Taffy retention (audit-20261113 step 11).
//!
//! The paired numbers live in `benches/retained_frames.rs` and
//! `qa/perf/20261107-retained-frames-paired.json`; this test turns the
//! mechanism-level evidence into an assertion: with retention on, a
//! one-panel change reuses layout nodes and creates almost none, while
//! scratch mode rebuilds. If retention silently stops working (or the
//! stats API disappears), this fails instead of printing stats nobody
//! reads.

// Rust guideline compliant 2026-02-21

use gpui::{
    AnyWindowHandle, AppContext as _, Context, Entity, HeadlessAppContext, IntoElement,
    ParentElement as _, PlatformTextSystem as _, Render, Styled as _, Window, div, px, size,
};
use gpui_design::{DesignSystem, DesignSystemState};
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::i18n::I18nState;
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::sync::Arc;

struct Panel {
    index: usize,
    value: u64,
}

impl Render for Panel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(140.))
            .p_2()
            .flex_col()
            .child(format!("Panel {}", self.index))
            .child(format!("value {}", self.value))
            .child("static row one")
            .child("static row two")
    }
}

struct Panels {
    panels: Vec<Entity<Panel>>,
    next: usize,
}

impl Panels {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            panels: (0..60)
                .map(|index| cx.new(|_| Panel { index, value: 0 }))
                .collect(),
            next: 0,
        }
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        let panel = self.panels[self.next % self.panels.len()].clone();
        self.next += 1;
        panel.update(cx, |panel, cx| {
            panel.value += 1;
            cx.notify();
        });
    }
}

impl Render for Panels {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .size_full()
            .children(self.panels.iter().cloned())
    }
}

fn bench_cx() -> HeadlessAppContext {
    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new_without_system_fonts(
        "IBM Plex Sans",
    ));
    text_system
        .add_fonts(gpui_miniapp::bundled_ui_fonts())
        .expect("bundled fonts must load");
    let text_system: Arc<dyn gpui::PlatformTextSystem> = text_system;
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || Ok(None));
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Dark));
        app.set_global(DesignSystemState::with_system(DesignSystem::neutral()));
        app.set_global(AccessibilityTree::new());
        app.set_global(I18nState::new());
    });
    cx
}

fn draw(cx: &mut HeadlessAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_view, window, app| {
        let _ = window.draw(app);
    })
    .expect("headless draw must succeed");
}

fn stats_over_ticks(retained: bool) -> gpui::LayoutStats {
    let mut cx = bench_cx();
    let handle = cx
        .open_window(size(px(1200.0), px(900.0)), |_, app| app.new(Panels::new))
        .expect("window must open");
    let window: AnyWindowHandle = *handle;
    cx.update_window(window, |_view, window, _app| {
        window.set_view_retention(retained);
    })
    .expect("window must exist");
    draw(&mut cx, window);
    cx.update_window(window, |_view, window, _app| window.reset_layout_stats())
        .expect("window must exist");
    for _ in 0..10 {
        handle
            .update(&mut cx, |panels, _, cx| panels.tick(cx))
            .expect("tick must apply");
        draw(&mut cx, window);
    }
    cx.update_window(window, |_view, window, _app| window.layout_stats())
        .expect("window must exist")
}

#[test]
fn retained_frames_reuse_layout_nodes_for_one_panel_change() {
    let retained = stats_over_ticks(true);
    assert!(
        retained.nodes_reused > 1000,
        "retained mode must reuse layout nodes: {retained:?}"
    );
    assert_eq!(
        retained.nodes_created, 0,
        "retained mode must create no nodes per one-panel change: {retained:?}"
    );
    assert!(
        retained.views_reused > 0,
        "retained mode must replay unchanged views: {retained:?}"
    );

    let scratch = stats_over_ticks(false);
    assert_eq!(
        scratch.views_reused, 0,
        "scratch mode must not replay views (sanity that the gate can tell modes apart)"
    );
    assert!(
        scratch.style_compares > retained.style_compares * 10,
        "scratch mode must re-compare styles broadly: scratch={scratch:?} retained={retained:?}"
    );
}
