//! End-to-end showcase frame benchmarks: cold-open a section and draw it.
//!
//! Each iteration opens a headless window, selects the section through the
//! public [`Showcase::select_section`] API, draws one full frame, and closes
//! the window. This covers the whole per-section CPU path: view
//! construction, element build, text shaping, layout, and scene paint. GPU
//! rasterization is out of scope on purpose: pixel pushing is stable, while
//! CPU frame cost is what UI work changes.

use criterion::{Criterion, criterion_group, criterion_main};
use gpui::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, PlatformTextSystem as _, px, size,
};
use gpui_design::{DesignSystem, DesignSystemState};
use gpui_showcase::Showcase;
use gpui_showcase::showcase::ShowcaseSection;
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::i18n::I18nState;
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::hint::black_box;
use std::sync::Arc;

/// Cold-open one section and draw its first frame; criterion measures the
/// closure, so context construction stays outside the timing.
fn cold_open_section(cx: &mut HeadlessAppContext, section: ShowcaseSection) {
    let handle = cx
        .open_window(size(px(1200.0), px(900.0)), |_, app| app.new(Showcase::new))
        .expect("headless window must open");
    handle
        .update(cx, |showcase, _, cx| showcase.select_section(section, cx))
        .expect("section must select");
    let any: AnyWindowHandle = *handle;
    cx.update_window(any, |_view, window, app| {
        let _ = window.draw(app);
    })
    .expect("headless draw must succeed");
    cx.update_window(any, |_view, window, _app| window.remove_window())
        .expect("window must close");
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

fn section_by_label(label: &str) -> ShowcaseSection {
    ShowcaseSection::all()
        .iter()
        .copied()
        .find(|section| section.label() == label)
        .unwrap_or_else(|| panic!("unknown showcase section label {label:?}"))
}

fn bench_section(c: &mut Criterion, id: &str, label: &str) {
    let section = section_by_label(label);
    let mut cx = bench_cx();
    // Warm once so lazy caches (shaping, atlases) settle before measuring.
    cold_open_section(&mut cx, section);
    c.bench_function(id, |b| {
        b.iter(|| {
            cold_open_section(&mut cx, section);
            black_box(());
        });
    });
}

fn bench_frames(c: &mut Criterion) {
    // Light (controls), heavy rows (table), deep tree (tree-view).
    bench_section(c, "showcase_frame_buttons", "Buttons");
    bench_section(c, "showcase_frame_table", "Table");
    bench_section(c, "showcase_frame_tree_view", "Tree View");
}

criterion_group!(benches, bench_frames);
criterion_main!(benches);
