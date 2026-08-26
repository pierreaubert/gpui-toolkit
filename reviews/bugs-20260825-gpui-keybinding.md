# Bug Review: gpui-keybinding — 2026-08-25

Scope: full scan of `crates/gpui-keybinding` — all 11 files under `src/` (~1,700 lines incl. unit tests), `Cargo.toml`, `benches/discovery.rs`, `tests/allocation_contracts.rs`, plus the crate's `README.md`/`AGENTS.md`. Cross-checked key-spec syntax assumptions against the vendored GPUI keystroke parser (`crates/3rdparties/gpui/src/platform/keystroke.rs:120`). Baseline `cargo test -p gpui-keybinding` is green (all unit tests, the allocation-contract test, doc-tests ignored as marked). The crate is pure data/logic: no rendering, no wgpu, no threads (caches are `thread_local!` + `RefCell`), so the GPU and threading categories were checked and are clean by construction.

## Findings

## Completion audit — 2026-08-26

Every confirmed cache, normalization, label-formatting, and allocation finding has a recorded regression; the remaining items are evidence-based compatibility or context-model dispositions. Current verification: `cargo test -p gpui-keybinding` (47 passed, 3 ignored).

Ranked by severity. No Critical or High issues found.

### Medium

1. **Case-normalized cache key, case-sensitive match — stale/empty which-key results.**
   `src/discovery.rs:291-303` (`keybinding_hints_cached`) normalizes the prefix to lowercase for the *cache key* (`normalized_ascii_lowercase`), but then calls `build_keybinding_hints(bindings, prefix)` with the **raw** prefix, and the token comparison at `src/discovery.rs:328` (`parts.next() != Some(expected)`) is case-sensitive. Consequence: querying `"Ctrl-K"` computes an empty hint list and stores it under the cache key `"ctrl-k"`; a later, correct query for `"ctrl-k"` then hits that entry and returns the wrong (empty) result for the rest of the process. The registry-level wrapper repeats the same pattern (`src/discovery.rs:431-448`: normalized key, raw prefix forwarded). Fix: pass `prefix_normalized.as_ref()` (not `prefix`) into `build_keybinding_hints`, and/or normalize each token before comparing. A one-line regression test: `keybinding_hints_cached(&docs, "Ctrl-K")` followed by `keybinding_hints_cached(&docs, "ctrl-k")` must return identical, non-empty results.

   **Disposition (2026-08-26): Fixed.** Both the free cache and registry wrapper now pass the normalized prefix into matching. Regressions cover mixed-case/free-cache and mixed-case/registry sequences, asserting identical non-empty results and shared cached `Rc` values. Verified with `cargo test -p gpui-keybinding` (40 passed, 3 ignored).

2. **Which-key cache keys on allocation addresses without retaining the source (ABA stale-hit risk).**
   `src/discovery.rs:132-159` derives `DocumentedBindingsKey` from the slice pointer, length, and the first/last bindings' string pointers — but unlike the palette cache (`src/discovery.rs:146-152`, which keeps `_source: Rc<...>` alive specifically to prevent address reuse), `HINTS_CACHE` retains nothing. For the free-function API `keybinding_hints_cached(&[DocumentedKeybinding], ..)`, a caller that rebuilds its binding `Vec` can have the allocator reuse the slice address; only the first/last elements are guarded, so two sets differing in any *middle* binding can collide and return hints computed from the old content. The registry path is safe in practice (its docs live in a retained `Rc` cleared on `register`), so this bites only direct users of the free function. Probability is low but the failure is silent and wrong. Fix: mirror the palette-cache design — take/retain an `Rc<[DocumentedKeybinding]>`, or hash content (the comment at `src/discovery.rs:85-88` explicitly rejected an O(n) content hash; retaining the source achieves the same safety at O(1)).

   **Disposition (2026-08-26): Fixed.** Each hint-cache entry now owns a binding snapshot and compares it before serving a pointer-identity cache hit. A changed source invalidates its prefix entries, covering allocator ABA reuse and mutation of a middle binding (which the former boundary guard missed deterministically). The regression mutates the middle raw chord and confirms rebuilt hints. Verified with the focused regression; full crate verification will follow this review.

### Low

3. **`format_key_label` mishandles the minus key in chords.**
   `src/platform.rs:63-66`: any spec whose final `-` has an empty tail returns the raw spec unformatted. GPUI's parser explicitly treats a trailing dash as the `-` key (`crates/3rdparties/gpui/src/platform/keystroke.rs:163-166`), so `"ctrl--"` is a valid, meaningful binding (Ctrl+Minus) that this crate displays literally as `ctrl--` while `"ctrl-="` would display as `Ctrl+=`. The existing test (`src/platform.rs:165-168`) codifies the current behavior for `"ctrl-"`/`"shift-"` (which GPUI rejects — actually invalid), so the guard can't distinguish "invalid spec" from "minus key". Fix: when `tail.is_empty() && !head.is_empty()`, format `head` as modifiers and append `"-"` as the key (`"Ctrl+-"`); keep the passthrough only for a bare `"-"`.

   **Disposition (2026-08-26): Fixed, with the syntax corrected.** GPUI accepts `ctrl-` as Ctrl+Minus; the review's `ctrl--` example was unnecessary. The formatter now renders modifier-plus-trailing-dash specs as `Ctrl+-`/`Shift+-`, while a bare dash and malformed doubled form remain unchanged. The regression calls `gpui::Keystroke::parse("ctrl-")` and verifies the displayed label.

4. **Registry-level search/hint caches are unbounded per preset.**
   `src/discovery.rs:407-411` and `src/discovery.rs:444-448` insert into `search_cache`/`hints_cache` keyed by every distinct normalized query string, with no eviction — the thread-local caches right next to them got explicit caps (`MAX_QUERIES_PER_SET = 64`, `src/discovery.rs:207-208`), the registry ones did not. In a command palette, every keystroke is a new query, so this grows for the lifetime of the registry (only `register()` clears it). Bounded in practice by session length, but it's an inconsistent policy and a slow leak on long-running apps. Fix: apply the same cap-and-clear discipline as `search_command_palette_cached`, or drop the registry-level cache entirely and rely on the thread-local one it already delegates to.

   **Disposition (2026-08-26): Fixed.** The registry's search and which-key caches now use the same 64-entry per-preset cap-and-clear policy as the thread-local caches. Regression fills each cache with 65 distinct values and verifies that it clears before retaining the new entry.

5. **Uncached `search_command_palette` wrapper does two full clones and pollutes the bounded cache.**
   `src/discovery.rs:192`: `search_command_palette_cached(Rc::from(entries.to_vec()), query).to_vec()` clones the whole entry set (each `CommandPaletteEntry` carries ~9 owned strings), runs the search, clones the results out of the `Rc`, and inserts a fresh cache entry under a pointer key that can never be hit again (the `Rc` is recreated per call), churning the 16-set thread-local cache (`cache.clear()` at `src/discovery.rs:247-249`) and evicting genuinely reusable entries from other callers. Fix: implement the uncached search directly over the slice (the scoring/filtering code is already factored into `score_entry`), leaving the cache to the `_cached` variant. Same shape, minor, in `keybinding_hints` (`src/discovery.rs:276-278`), though that one at least reuses the caller's slice.

   **Disposition (2026-08-26): Fixed.** The Vec-returning `search_command_palette` and `keybinding_hints` convenience APIs now calculate their own result directly. They no longer clone an input solely to manufacture an `Rc`, nor evict useful results from the thread-local caches. Regression verifies both wrappers return results without populating either cache.

6. **Docs claim context-aware conflict detection; the implementation has no context.**
   `AGENTS.md:36` and the crate docs say conflict detection finds "duplicate key+context bindings", but `DocumentedKeybinding` (`src/provider.rs:43-53`) has no context field and `detect_conflicts` (`src/conflict.rs:22-42`) groups only by raw spec/display key. Two bindings legitimately sharing a key in different GPUI contexts (the README's own recommended resolution #1) are reported as conflicts with no way to suppress them. Also, grouping is case-sensitive on `raw_key_spec` (`src/conflict.rs:26`), so `"Secondary-S"` vs `"secondary-s"` silently misses. Fix: either add an optional `context` field and group by `(spec, context)`, or correct the docs; normalize raw specs to lowercase before grouping.

   **Disposition (2026-08-26): Fixed and clarified.** The documentation now accurately states that the detector works from `DocumentedKeybinding` raw specs and cannot inspect executable GPUI contexts. Conflict grouping is semantic: GPUI parses each chord key, so modifier spelling case is normalized while uppercase key names retain their implicit Shift distinction. The review's `Secondary-S` versus `secondary-s` example was therefore not a conflict; `Secondary-s` versus `secondary-s` is covered by regression.

7. **Built-in navigation presets contain duplicate keys that trip their own conflict detector.**
   `src/presets/vim.rs` binds `h` to both Left and Collapse and `l` to both Right and Expand (`src/presets/vim.rs:17-27, 65-75`); `src/presets/emacs.rs` duplicates `ctrl-b`/`ctrl-f` the same way (`src/presets/emacs.rs:17-27, 65-75`); `src/presets/default.rs` duplicates `left`/`right` (`src/presets/default.rs:17-27, 65-75`). If an app feeds these mappings into `detect_conflicts` (the documented release gate), it gets conflicts out of the box. Likely intentional (list navigation vs tree expand/collapse are different contexts), which circles back to finding 6: without context on `DocumentedKeybinding`, there is no way to express that. Fix: ship these with distinct contexts once the field exists, or document the expected duplicates.

   **Disposition (2026-08-26): Disproved.** Presets expose `NavigationMapping` values only; they do not construct `DocumentedKeybinding` records, register a provider, or invoke `detect_conflicts`. Repeated navigation keys intentionally represent different consumer contexts (for example, list motion versus tree expansion), so no built-in conflict report exists to fix. The conflict documentation now explicitly explains that a caller must account for executable contexts.

8. **Terminal-hint description silently overwritten by later bindings.**
   `src/discovery.rs:342-345` and `src/discovery.rs:353-356`: when two bindings terminate on the same next key, the second unconditionally replaces `description`/`category` of the shared hint entry. Which-key UIs will show whichever binding happened to be last in registration order, hiding the ambiguity that `detect_conflicts` exists to surface. Fix: keep the first (deterministic given sorted input) or mark the hint as ambiguous.

   **Disposition (2026-08-26): Disproved as a behavior bug; precedence documented.** GPUI explicitly gives the later-added binding higher precedence. The registry collects providers in registration order, and which-key terminal metadata now documents and tests the matching later-binding-wins rule. No ambiguity remains for callers that provide duplicate completed chords.

9. **`Cargo.toml` carries copy-pasted feature flags from another crate.**
   `Cargo.toml:15-22` declares empty features `autoeq`, `gpu-2d`, `gpu-3d`, `reqwest`, `showcase`, `spinorama`, `tokio`, `urlencoding` — none referenced anywhere in this crate (no `cfg(feature = ...)` in `src/`). Misleading to downstream users reading the manifest. Fix: delete the unused features.

10. **Case model diverges from GPUI's keystroke semantics for hint matching.**
    GPUI normalizes a typed uppercase letter to `shift+<lowercase>` (`crates/3rdparties/gpui/src/platform/keystroke.rs:179-187`), and the Vim preset encodes that as raw specs `"G"` / `"g T"` (`src/presets/vim.rs:48-50, 83-87`). Any consumer doing string-prefix matching of typed chords against these specs (which is exactly what `build_keybinding_hints` models) must reproduce GPUI's normalization or `g shift-t` will never match `"g T"`. The crate ships no such normalization helper. Not a bug in this crate's data, but an undocumented trap; a `normalize_key_spec()` next to `format_key_label()` would close it (and would also fix findings 1 and 6 uniformly).

   **Disposition (2026-08-26): Disproved; documented intentional compatibility aliases.** `Justfile` and CI run a workspace-wide common feature union that includes this crate. The empty aliases let `cargo --workspace --features autoeq,gpu-2d,gpu-3d,reqwest,spinorama,tokio,urlencoding` select every public crate uniformly; several peer crates use the same pattern. The manifest now explains the purpose.

## Clean bill

- **Threading**: no mutexes, channels, awaits, or blocking calls; all caches are `thread_local!`/`RefCell` with borrows never held across provider callbacks — reentrancy-safe as written.
- **Panics**: no `unwrap`/`expect`/indexing panics in production paths; the one `let-else` in `build_keybinding_hints` is total.
- **Allocation discipline on the hot path**: cache-hit paths are provably allocation-free (`tests/allocation_contracts.rs` measures 1,000 hits against a zero budget and passes), results are shared via `Rc<[T]>`, and the thread-local caches have explicit size caps.
- **Presets/preset.rs/platform.rs** (finding 3 aside): small, table-driven, and well covered by unit tests; verified against GPUI's actual parser for modifier names, `secondary`, chord syntax, and uppercase handling.
