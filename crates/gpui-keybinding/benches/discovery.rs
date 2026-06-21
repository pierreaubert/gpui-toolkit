use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gpui_keybinding::{
    command_palette_entries, format_key_label, search_command_palette_cached,
    DocumentedKeybinding, KeybindingCategory,
};
use std::rc::Rc;

fn bench_format_key_label(c: &mut Criterion) {
    let specs = [
        "up",
        "space",
        "enter",
        "ctrl-s",
        "ctrl-shift-k",
        "ctrl-k ctrl-s",
        "secondary-s",
    ];

    c.bench_function("format_key_label", |b| {
        b.iter(|| {
            for spec in specs {
                black_box(format_key_label(black_box(spec)));
            }
        })
    });
}

fn bench_search_command_palette_cached(c: &mut Criterion) {
    let bindings = generate_bindings();
    let entries: Rc<_> = command_palette_entries(&bindings).into();

    // Warm the thread-local cache so the benchmarked calls are cache hits.
    let _warm = search_command_palette_cached(Rc::clone(&entries), "");

    let queries = ["", "file", "save"];

    c.bench_function("search_command_palette_cached", |b| {
        b.iter(|| {
            for query in queries {
                black_box(search_command_palette_cached(
                    black_box(Rc::clone(&entries)),
                    black_box(query),
                ));
            }
        })
    });
}

fn generate_bindings() -> Vec<DocumentedKeybinding> {
    let categories = [
        KeybindingCategory::Navigation,
        KeybindingCategory::Editing,
        KeybindingCategory::FileOps,
        KeybindingCategory::Formatting,
        KeybindingCategory::View,
        KeybindingCategory::Search,
        KeybindingCategory::Playback,
        KeybindingCategory::System,
    ];
    let verbs = [
        "open", "close", "save", "find", "toggle", "show", "hide", "create", "delete", "move",
        "copy", "paste", "undo", "redo", "select", "jump", "switch", "focus", "reload", "quit",
    ];
    let nouns = [
        "file",
        "folder",
        "window",
        "panel",
        "tab",
        "search",
        "palette",
        "setting",
        "account",
        "view",
        "document",
        "project",
        "buffer",
        "symbol",
        "reference",
        "terminal",
        "sidebar",
        "notification",
        "workspace",
        "session",
    ];
    let keys = [
        "ctrl-a",
        "ctrl-b",
        "ctrl-c",
        "ctrl-d",
        "ctrl-e",
        "ctrl-f",
        "ctrl-g",
        "ctrl-h",
        "ctrl-i",
        "ctrl-j",
        "ctrl-k",
        "ctrl-l",
        "ctrl-m",
        "ctrl-n",
        "ctrl-o",
        "ctrl-p",
        "ctrl-q",
        "ctrl-r",
        "ctrl-s",
        "ctrl-t",
        "ctrl-u",
        "ctrl-v",
        "ctrl-w",
        "ctrl-x",
        "ctrl-y",
        "ctrl-z",
        "secondary-s",
        "secondary-o",
        "secondary-n",
        "secondary-w",
        "secondary-q",
        "secondary-z",
        "shift-up",
        "shift-down",
        "shift-left",
        "shift-right",
        "enter",
        "space",
        "escape",
        "up",
        "down",
        "left",
        "right",
        "ctrl-shift-k",
        "ctrl-shift-p",
        "alt-left",
        "alt-right",
        "ctrl-k ctrl-s",
        "ctrl-k ctrl-o",
        "ctrl-k ctrl-k",
        "g g",
        "z o",
        "ctrl-x ctrl-s",
    ];

    let mut bindings = Vec::with_capacity(120);
    for (i, key) in keys.iter().cycle().take(120).enumerate() {
        let verb = verbs[i % verbs.len()];
        let noun = nouns[(i / verbs.len()) % nouns.len()];
        let category = categories[i % categories.len()].clone();
        let description = format!("{verb} {noun} {i}");
        bindings.push(
            DocumentedKeybinding::new(format_key_label(key), description, category)
                .with_raw_key_spec(*key),
        );
    }
    bindings
}

criterion_group!(benches, bench_format_key_label, bench_search_command_palette_cached);
criterion_main!(benches);
