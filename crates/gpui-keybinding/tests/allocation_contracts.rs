use gpui_keybinding::{
    DocumentedKeybinding, KeybindingCategory, command_palette_entries, keybinding_hints_cached,
    search_command_palette_cached,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;
use std::rc::Rc;

#[test]
fn normalized_command_palette_cache_hits_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
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
    assert_hint_cache_hits_are_allocation_free();
}

fn assert_hint_cache_hits_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    let bindings =
        [
            DocumentedKeybinding::new("Ctrl+K Ctrl+S", "Save all", KeybindingCategory::FileOps)
                .with_raw_key_spec("ctrl-k ctrl-s"),
        ];
    black_box(keybinding_hints_cached(&bindings, "ctrl-k"));

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1_000 {
        black_box(keybinding_hints_cached(&bindings, "ctrl-k"));
    }
    AllocationBudget::zero("keybinding-hint-cache-hit-1000x")
        .assert_contains(probe.sample("keybinding-hint-cache-hit-1000x"));
}
