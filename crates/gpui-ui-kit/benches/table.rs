//! Headless table benchmarks: element build plus layout plus scene paint.
//!
//! Every render benchmark drives a real `Table` through a headless window
//! `draw`, so the numbers cover the full per-frame CPU path (element tree
//! build, text shaping, layout, scene paint) without GPU rasterization.
//! The text stack is cosmic-text with the bundled Plex/Lilex fonts, matching
//! the wasm platform, so shaping costs are representative on any host.

use criterion::{Criterion, criterion_group, criterion_main};
use gpui::{
    AnyWindowHandle, AppContext as _, Context, HeadlessAppContext, IntoElement,
    PlatformTextSystem as _, Render, SharedString, Window, WindowHandle, px, size,
};
use gpui_design::{DesignSystem, DesignSystemState};
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::data_navigation::DataVirtualWindow;
use gpui_ui_kit::i18n::I18nState;
use gpui_ui_kit::table::{Column, SelectionMode, Table};
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::collections::HashSet;
use std::hint::black_box;
use std::sync::Arc;

#[derive(Clone)]
struct BenchRow {
    id_text: SharedString,
    name: SharedString,
    email: SharedString,
    role: SharedString,
}

fn make_rows(count: usize) -> Vec<BenchRow> {
    const ROLES: [&str; 4] = ["admin", "editor", "viewer", "owner"];
    (0..count)
        .map(|id| BenchRow {
            // Memoized like the showcase `User::id_text`: cell renderers run
            // per frame, so per-cell formatting would dominate the numbers.
            id_text: id.to_string().into(),
            name: format!("User {id:05}").into(),
            email: format!("user{id:05}@example.com").into(),
            role: ROLES[id % ROLES.len()].into(),
        })
        .collect()
}

fn make_columns() -> Vec<Column<BenchRow>> {
    vec![
        Column::new("id", "ID")
            .width(px(60.0))
            .cell_render(|row: &BenchRow, _, _, _| row.id_text.clone()),
        Column::new("name", "Name").cell_render(|row: &BenchRow, _, _, _| row.name.clone()),
        Column::new("email", "Email").cell_render(|row: &BenchRow, _, _, _| row.email.clone()),
        Column::new("role", "Role")
            .width(px(100.0))
            .cell_render(|row: &BenchRow, _, _, _| row.role.clone()),
    ]
}

struct TableBenchView {
    rows: Vec<BenchRow>,
    page: usize,
    page_size: usize,
    selected: HashSet<usize>,
    window: Option<DataVirtualWindow>,
}

impl TableBenchView {
    fn new(rows: Vec<BenchRow>) -> Self {
        Self {
            rows,
            page: 0,
            page_size: usize::MAX,
            selected: HashSet::new(),
            window: None,
        }
    }
}

impl Render for TableBenchView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Mirror the showcase data flow: the app slices (and clones) the page
        // on every render; the table displays what it is given.
        let page_rows = if self.page_size == usize::MAX {
            self.rows.clone()
        } else {
            let start = (self.page * self.page_size).min(self.rows.len());
            let end = (start + self.page_size).min(self.rows.len());
            self.rows[start..end].to_vec()
        };
        let mut table = Table::new("bench-table", page_rows)
            .columns(make_columns())
            .selection_mode(SelectionMode::Multiple)
            .selected_indices(self.selected.clone())
            .alternating_rows(true);
        if let Some(window) = self.window {
            table = table.virtual_window(window.with_total(self.rows.len()));
        }
        table
    }
}

/// Headless context with the wasm-identical text stack and prod globals.
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

fn open_table_window(
    cx: &mut HeadlessAppContext,
    state: TableBenchView,
) -> WindowHandle<TableBenchView> {
    cx.open_window(size(px(1200.0), px(900.0)), |_, app| app.new(|_| state))
        .expect("headless window must open")
}

fn draw_frame(cx: &mut HeadlessAppContext, window: &WindowHandle<TableBenchView>) {
    let any: AnyWindowHandle = **window;
    cx.update_window(any, |_view, window, app| {
        let _ = window.draw(app);
    })
    .expect("headless draw must succeed");
}

fn bench_mount(c: &mut Criterion) {
    // Context (including parsed fonts) lives outside the loop: apps pay font
    // parsing once, while every mount pays window open plus first draw.
    let mut cx = bench_cx();
    c.bench_function("table_mount_100", |b| {
        b.iter_with_setup(
            || make_rows(100),
            |rows| {
                let window = open_table_window(&mut cx, TableBenchView::new(rows));
                draw_frame(&mut cx, &window);
                let any: AnyWindowHandle = *window;
                cx.update_window(any, |_view, window, _app| window.remove_window())
                    .expect("window must close");
                black_box(());
            },
        );
    });
}

fn bench_redraw(c: &mut Criterion) {
    let mut cx = bench_cx();
    let window = open_table_window(&mut cx, TableBenchView::new(make_rows(100)));
    draw_frame(&mut cx, &window);
    c.bench_function("table_redraw_100", |b| {
        b.iter(|| {
            window
                .update(&mut cx, |_, _, cx| cx.notify())
                .expect("notify must succeed");
            draw_frame(&mut cx, &window);
            black_box(());
        });
    });
}

fn bench_page_flip(c: &mut Criterion) {
    let mut cx = bench_cx();
    let mut state = TableBenchView::new(make_rows(10_000));
    state.page_size = 50;
    let window = open_table_window(&mut cx, state);
    draw_frame(&mut cx, &window);
    let pages = 10_000 / 50;
    let mut page = 0;
    c.bench_function("table_page_flip_10k", |b| {
        b.iter(|| {
            page = (page + 1) % pages;
            window
                .update(&mut cx, |view, _, cx| {
                    view.page = page;
                    cx.notify();
                })
                .expect("page update must succeed");
            draw_frame(&mut cx, &window);
            black_box(page);
        });
    });
}

fn bench_selection_toggle(c: &mut Criterion) {
    let mut cx = bench_cx();
    let window = open_table_window(&mut cx, TableBenchView::new(make_rows(1_000)));
    draw_frame(&mut cx, &window);
    let mut index = 0;
    c.bench_function("table_selection_toggle_1k", |b| {
        b.iter(|| {
            index = (index + 1) % 1_000;
            window
                .update(&mut cx, |view, _, cx| {
                    if !view.selected.remove(&index) {
                        view.selected.insert(index);
                    }
                    cx.notify();
                })
                .expect("selection update must succeed");
            draw_frame(&mut cx, &window);
            black_box(index);
        });
    });
}

fn bench_virtual_scroll(c: &mut Criterion) {
    let mut cx = bench_cx();
    let mut state = TableBenchView::new(make_rows(10_000));
    state.window = Some(DataVirtualWindow::new(10_000, 0, 30));
    let window = open_table_window(&mut cx, state);
    draw_frame(&mut cx, &window);
    let mut start = 0;
    c.bench_function("table_virtual_scroll_10k", |b| {
        b.iter(|| {
            start = (start + 7) % (10_000 - 30);
            window
                .update(&mut cx, |view, _, cx| {
                    view.window = Some(DataVirtualWindow::new(10_000, start, start + 30));
                    cx.notify();
                })
                .expect("scroll update must succeed");
            draw_frame(&mut cx, &window);
            black_box(start);
        });
    });
}

fn bench_sort(c: &mut Criterion) {
    let mut cx = bench_cx();
    let window = open_table_window(&mut cx, TableBenchView::new(make_rows(10_000)));
    draw_frame(&mut cx, &window);
    let mut ascending = false;
    c.bench_function("table_sort_10k", |b| {
        b.iter_with_setup(
            || make_rows(10_000),
            |mut rows| {
                // Mirror the showcase on_sort handler: sort the source, reset
                // selection (indices are row-based), redraw.
                ascending = !ascending;
                if ascending {
                    rows.sort_by(|a, b| a.name.cmp(&b.name));
                } else {
                    rows.sort_by(|a, b| b.name.cmp(&a.name));
                }
                window
                    .update(&mut cx, |view, _, cx| {
                        view.rows = rows;
                        view.selected.clear();
                        cx.notify();
                    })
                    .expect("sort update must succeed");
                draw_frame(&mut cx, &window);
                black_box(ascending);
            },
        );
    });
}

fn bench_pagination_math(c: &mut Criterion) {
    use gpui_ui_kit::table::PaginationState;
    c.bench_function("table_pagination_math", |b| {
        let mut page = 0;
        b.iter(|| {
            page = (page + 1) % 200;
            let state = PaginationState {
                current_page: page,
                page_size: 50,
                total_items: 10_000,
            };
            black_box((state.total_pages(), state.page_range()));
        });
    });
}

fn bench_virtual_window_math(c: &mut Criterion) {
    c.bench_function("table_virtual_window_math", |b| {
        let mut offset = 0.0f32;
        b.iter(|| {
            offset += 32.0;
            if offset > 320_000.0 {
                offset = 0.0;
            }
            black_box(DataVirtualWindow::from_viewport(
                black_box(10_000),
                black_box(offset),
                black_box(32.0),
                black_box(960.0),
                black_box(5),
            ));
        });
    });
}

criterion_group!(
    benches,
    bench_mount,
    bench_redraw,
    bench_page_flip,
    bench_selection_toggle,
    bench_virtual_scroll,
    bench_sort,
    bench_pagination_math,
    bench_virtual_window_math,
);
criterion_main!(benches);
