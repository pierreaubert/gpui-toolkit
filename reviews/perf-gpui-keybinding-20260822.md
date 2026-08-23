# Perf review: gpui-keybinding

Date: 2026-08-22

## Role and hot paths

`gpui-keybinding` (~1,400 lines under `crates/gpui-keybinding/src/`) is a pure
data-model crate: preset enums (`preset.rs`), provider trait +
`DocumentedKeybinding` (`provider.rs`), a caching `KeybindingRegistry`
(`registry.rs`), conflict detection (`conflict.rs`), platform-aware key label
formatting (`platform.rs`), and discovery data for command palettes / which-key
overlays (`discovery.rs`). Preset tables in `presets/` are `&'static` slices —
zero runtime cost.

There is **no key-dispatch hot path in this crate**: actual keystroke matching
lives in GPUI's keymap system; this crate only *produces* `KeyBinding` lists
(`registry.rs:57-71`) and help/palette data. The genuine hot paths are:

- Palette search per typed keystroke: `search_command_palette_cached`
  (`src/discovery.rs:154-203`), reached per keystroke while a palette is open.
- Which-key hint rebuild per chord keystroke: `keybinding_hints_cached`
  (`src/discovery.rs:222-247`) → `build_keybinding_hints` (`:249-288`).
- One-time (per registry change) entry build: `CommandPaletteEntry::from_binding`
  (`:34-59`) and `command_palette_entries` (`:123-130`).
- `format_key_label` (`src/platform.rs:37-51`) — has allocation-free fast paths
  for modifier-less and known keys; fine.

The crate already has a criterion bench (`benches/discovery.rs`) and a
zero-allocation cache-hit contract test (`tests/allocation_contracts.rs`, using
`gpui-profiler`), so the baseline discipline is in place.

## Findings

1. **[Alloc] O(N×strings) SipHash pass on every cached query, including hits.**
   `search_command_palette_cached` calls `hash_palette_entries(&entries)`
   (`src/discovery.rs:159`) on *every* call; that hashes ~8 heap strings per
   entry through `DefaultHasher` (`:95-101`). Same pattern in
   `keybinding_hints_cached` → `hash_documented_bindings` (`:227`, `:84-93`).
   So the "cache hit" path is still O(total string bytes of all bindings) —
   the cost the cache exists to avoid. For 120 bindings this is ~1000+ string
   hashes per keystroke. Also, a u64 hash is the *sole* cache key: two
   different binding sets colliding would return stale results (correctness
   hazard, not just perf). Fix: key the cache by `Rc::as_ptr` identity (the
   registry already hands out shared `Rc` slices, `registry.rs:96-107`) or by
   an explicit registry version counter bumped in `clear_caches`.

2. **[Alloc] Convenience wrappers clone the full entry list twice per call.**
   `search_command_palette` does
   `search_command_palette_cached(Rc::from(entries.to_vec()), query).to_vec()`
   (`src/discovery.rs:140-145`) — a full deep-ish clone of every
   `CommandPaletteEntry` (each holds 8 `String` fields, `:16-30`) going in,
   and another full clone of the result coming out. The registry-level
   `search_command_palette` / `keybinding_hints` also `.to_vec()` the cached
   `Rc` result per keystroke (`:303-309`, `:344-346`). Callers in palette UIs
   typing at key-repeat rate hit this per keystroke. Doc comments already steer
   to the `_cached` variants; the wrappers should be deprecated or take `Rc`
   directly. (needs profiling to confirm callers actually use the wrappers.)

3. **[Alloc] Per-binding `Vec` in hint building.** `build_keybinding_hints`
   collects `spec.split_whitespace().collect::<Vec<&str>>()` for every binding
   on each cache miss (`src/discovery.rs:255`; prefix parts at `:250` are
   fine). Chord-depth is 1–3; an iterator-based `starts_with` comparison
   removes the per-binding allocation entirely.

4. **[Alloc] Thread-local caches are unbounded and never invalidated.**
   `SEARCH_CACHE` / `HINTS_CACHE` (`src/discovery.rs:108-111`) grow by one
   entry per (bindings-hash, query) pair for the process lifetime, and
   `KeybindingRegistry::clear_caches` (`src/registry.rs:109-115`) does not —
   and structurally cannot — clear them. After provider re-registration or
   preset switches in a long session, stale `Rc` result sets accumulate. With
   finding 1 fixed (pointer/version keys), add an eviction bound or clear hook.

5. **[Alloc] `CommandPaletteEntry` stores 6+ heap strings per entry.**
   `from_binding` builds `search_index` plus four separate `*_lower` strings
   (`src/discovery.rs:25-29`, `:44-58`) — ~6 allocations per binding at build
   time. This is once per registry change, so low priority; a single arena
   `String` with range fields would cut it to one allocation. Not worth doing
   unless entry counts grow large (needs profiling).

**GPU / roundtrip audit: none.** The crate contains no rendering, no wgpu/vello
usage, no `map_async`/`read_texture`/`device.poll`. Its work is string
matching over tens-to-hundreds of bindings — far below any GPU threshold. The
only GPU-relevant note is that `KeyBinding` lists it produces feed GPUI's
native keymap, which handles dispatch itself. Nothing to move.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Replace hash-of-contents cache keys with `Rc::as_ptr` identity or a registry version counter; add equality guard against collisions | 1 | S | Removes O(N·strings) SipHash per keystroke; closes stale-result hazard |
| 2 | Make wrapper APIs borrow/`Rc`-based (deprecate `Vec`-returning `search_command_palette`, `keybinding_hints` in favor of `_cached`) | 2 | S | Removes 2 full-list clones per keystroke for wrapper users |
| 3 | Iterator-based prefix match in `build_keybinding_hints` instead of `Vec` collect | 3 | S | One alloc per binding per chord keystroke removed |
| 4 | Bound or invalidate the thread-local caches; tie into registry `clear_caches` or add TTL/LRU | 4 | S–M | Stops unbounded memory growth in long sessions |
| 5 | Extend `tests/allocation_contracts.rs` to cover cache-hit path of `keybinding_hints_cached` and the finding-1 fix | 1, 4 | S | Locks in the win; the harness already exists |

## Quick wins

- Finding 1: pointer/version-keyed caches — a ~20-line change in
  `src/discovery.rs` plus the existing alloc-contract test to verify.
- Finding 3: drop the per-binding `collect()` in `build_keybinding_hints`.
- Finding 2 (partial): change `KeybindingRegistry::search_command_palette` /
  `keybinding_hints` to return `Rc<[…]>` (or document the `_cached` variants
  as the only supported palette path).

Overall: the crate's perf surface is small and already partly optimized (fast
paths in `format_key_label`, `Rc`-shared caches, criterion bench, alloc
contract test). The one real inefficiency is paying a full-content hash on
every cache lookup — fix that and this crate is done for the campaign.
