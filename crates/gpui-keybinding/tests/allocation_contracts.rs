use gpui_keybinding::{
    DocumentedKeybinding, KeybindingCategory, command_palette_entries,
    search_command_palette_cached,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;
use std::rc::Rc;

#[test]
fn normalized_command_palette_cache_hits_are_allocation_free() {
    let bindings = [DocumentedKeybinding::new(
        "Ctrl+S",
        "Save file",
        KeybindingCategory::FileOps,
    )];
    let entries: Rc<_> = command_palette_entries(&bindings).into();

    // Warm the exact normalized-query cache entry before measuring.
    black_box(search_command_palette_cached(Rc::clone(&entries), "save"));

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1_000 {
        black_box(search_command_palette_cached(Rc::clone(&entries), "save"));
    }

    AllocationBudget::zero("command-palette-cache-hit-1000x")
        .assert_contains(probe.sample("command-palette-cache-hit-1000x"));
}
