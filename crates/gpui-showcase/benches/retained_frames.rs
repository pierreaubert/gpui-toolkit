//! Steady-state frame benchmarks for the gpui-fast port (perf-cache):
//! retained views, retained Taffy layout nodes, and list layers.
//!
//! `e2e_frames` measures cold-open first frames, which retention cannot
//! help. These measure the frames after: a window stays open, one small
//! thing changes (one panel of sixty is notified, a list scrolls, the
//! showcase root is refreshed), and the frame is drawn again. Every
//! scenario runs twice — `*_retained` against `*_scratch`, the latter with
//! `Window::set_view_retention(false)`, which is upstream GPUI's drawing
//! model — so the pair is the retained speedup on this machine, in the
//! paired style of `qa/perf/`.
//!
//! At setup each scenario also prints its `Window::layout_stats()` delta
//! over ten warm frames (nodes reused vs created, style writes), which is
//! the mechanism-level evidence behind the timing pair. Headless CPU path
//! only, like `e2e_frames`: GPU rasterization is out of scope.

use criterion::{Criterion, criterion_group, criterion_main};
use gpui::{
    AnyWindowHandle, AppContext as _, Context, Entity, HeadlessAppContext, IntoElement,
    ParentElement as _, PlatformTextSystem as _, Render, ScrollStrategy, Styled as _,
    UniformListScrollHandle, Window, div, px, size, uniform_list,
};
use gpui_design::{DesignSystem, DesignSystemState};
use gpui_showcase::Showcase;
use gpui_showcase::showcase::ShowcaseSection;
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::i18n::I18nState;
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::hint::black_box;
use std::sync::Arc;

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

fn set_retention(cx: &mut HeadlessAppContext, window: AnyWindowHandle, enabled: bool) {
    cx.update_window(window, |_view, window, _app| {
        window.set_view_retention(enabled);
    })
    .expect("window must exist");
}

/// Prints the layout-stats delta produced by `frames` repetitions of
/// `step` (change + draw), so a run records *why* the pair differs.
fn report_stats(
    label: &str,
    cx: &mut HeadlessAppContext,
    window: AnyWindowHandle,
    frames: usize,
    mut step: impl FnMut(&mut HeadlessAppContext),
) {
    cx.update_window(window, |_view, window, _app| window.reset_layout_stats())
        .expect("window must exist");
    for _ in 0..frames {
        step(cx);
        draw(cx, window);
    }
    let stats = cx
        .update_window(window, |_view, window, _app| window.layout_stats())
        .expect("window must exist");
    eprintln!(
        "[{label}] {frames} frames: nodes reused={} created={} freed={} \
         style writes={} compares={} children writes={} measures={} kept={}",
        stats.nodes_reused,
        stats.nodes_created,
        stats.nodes_freed,
        stats.style_writes,
        stats.style_compares,
        stats.children_writes,
        stats.measure_calls,
        stats.measurements_kept,
    );
}

// ---------------------------------------------------------------- panels

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

    /// Notifies one panel, as a streaming quote tick would.
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

fn bench_panels(c: &mut Criterion) {
    for (suffix, retained) in [("retained", true), ("scratch", false)] {
        let mut cx = bench_cx();
        let handle = cx
            .open_window(size(px(1200.0), px(900.0)), |_, app| app.new(Panels::new))
            .expect("window must open");
        let window: AnyWindowHandle = *handle;
        set_retention(&mut cx, window, retained);
        draw(&mut cx, window);
        report_stats(
            &format!("panels_one_change_{suffix}"),
            &mut cx,
            window,
            10,
            |cx| {
                handle
                    .update(cx, |panels, _, cx| panels.tick(cx))
                    .expect("tick must apply");
            },
        );
        c.bench_function(&format!("retained_panels_one_change_{suffix}"), |b| {
            b.iter(|| {
                handle
                    .update(&mut cx, |panels, _, cx| panels.tick(cx))
                    .expect("tick must apply");
                draw(&mut cx, window);
                black_box(());
            });
        });
        c.bench_function(&format!("retained_panels_no_change_{suffix}"), |b| {
            b.iter(|| {
                handle
                    .update(&mut cx, |_panels, _, cx| cx.notify())
                    .expect("notify must apply");
                draw(&mut cx, window);
                black_box(());
            });
        });
    }
}

// ----------------------------------------------------------- uniform list

struct Rows {
    handle: UniformListScrollHandle,
    offset: f32,
}

impl Rows {
    fn scroll(&mut self, cx: &mut Context<Self>) {
        self.offset += 137.0;
        if self.offset > 4000.0 {
            self.offset = 0.0;
        }
        // 28px rows: advance roughly five rows per frame, like a wheel.
        let ix = (self.offset / 28.0) as usize % 4900;
        self.handle.scroll_to_item_strict(ix, ScrollStrategy::Top);
        cx.notify();
    }
}

impl Render for Rows {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            uniform_list(
                "bench-rows",
                5000,
                cx.processor(|_this, range: std::ops::Range<usize>, _window, _cx| {
                    range
                        .map(|ix| {
                            div()
                                .h(px(28.))
                                .flex()
                                .child(format!("Row {ix}"))
                                .child(format!("  value {}", ix * 31 % 977))
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.handle)
            .h_full(),
        )
    }
}

fn bench_rows(c: &mut Criterion) {
    for (suffix, retained) in [("retained", true), ("scratch", false)] {
        let mut cx = bench_cx();
        let handle = cx
            .open_window(size(px(800.0), px(600.0)), |_, app| {
                app.new(|_| Rows {
                    handle: UniformListScrollHandle::new(),
                    offset: 0.0,
                })
            })
            .expect("window must open");
        let window: AnyWindowHandle = *handle;
        set_retention(&mut cx, window, retained);
        draw(&mut cx, window);
        report_stats(
            &format!("uniform_list_scroll_{suffix}"),
            &mut cx,
            window,
            10,
            |cx| {
                handle
                    .update(cx, |rows, _, cx| rows.scroll(cx))
                    .expect("scroll must apply");
            },
        );
        c.bench_function(&format!("retained_uniform_list_scroll_{suffix}"), |b| {
            b.iter(|| {
                handle
                    .update(&mut cx, |rows, _, cx| rows.scroll(cx))
                    .expect("scroll must apply");
                draw(&mut cx, window);
                black_box(());
            });
        });
    }
}

// ------------------------------------------------------- showcase section

fn bench_showcase_table(c: &mut Criterion) {
    let section = ShowcaseSection::all()
        .iter()
        .copied()
        .find(|section| section.label() == "Table")
        .expect("Table section must exist");
    for (suffix, retained) in [("retained", true), ("scratch", false)] {
        let mut cx = bench_cx();
        let handle = cx
            .open_window(size(px(1200.0), px(900.0)), |_, app| app.new(Showcase::new))
            .expect("window must open");
        handle
            .update(&mut cx, |showcase, _, cx| {
                showcase.select_section(section, cx);
            })
            .expect("section must select");
        let window: AnyWindowHandle = *handle;
        set_retention(&mut cx, window, retained);
        draw(&mut cx, window);
        report_stats(
            &format!("showcase_table_refresh_{suffix}"),
            &mut cx,
            window,
            10,
            |cx| {
                handle
                    .update(cx, |_showcase, _, cx| cx.notify())
                    .expect("notify must apply");
            },
        );
        c.bench_function(&format!("retained_showcase_table_refresh_{suffix}"), |b| {
            b.iter(|| {
                handle
                    .update(&mut cx, |_showcase, _, cx| cx.notify())
                    .expect("notify must apply");
                draw(&mut cx, window);
                black_box(());
            });
        });
    }
}

fn bench_retained(c: &mut Criterion) {
    bench_panels(c);
    bench_rows(c);
    bench_showcase_table(c);
}

criterion_group!(benches, bench_retained);
criterion_main!(benches);
