# Perf review: gpui-au

Date: 2026-08-22

## Role and hot paths

macOS Audio Unit platform backend: embeds GPUI inside an AUv3 ViewController's
NSView via CAMetalLayer + wgpu (Metal). The crate is thin (~3.6k lines); most
frame cost lives in vendored `gpui`/`gpui_wgpu`, but the crate owns three hot
entry surfaces:

- **Per-frame paint**: Swift CVDisplayLink/timer → `ffi::gpui_au_request_frame`
  (`src/ffi.rs:233`) → `AuWindow::draw` (`src/window/au_window.rs:475`) →
  `WgpuRenderer::draw`. GPUI core gates real work behind
  `invalidator.is_dirty()` (`crates/3rdparties/gpui/src/window.rs:1526`), so
  idle frames are cheap (mutex + `handle.update` round-trips only).
- **Per-event marshalling**: `ffi.rs` mouse/keyboard/text entry points →
  `dispatch_to_window` → `AuWindow::dispatch_input` (`au_window.rs:255`).
- **Text**: `AuTextSystem` — CoreText shaping (`layout_line`) and CoreGraphics
  glyph rasterization (`rasterize_glyph`), called during scene building and
  atlas fill on every text change.

No GPU readback/offscreen→paint pattern exists in this crate: rendering is a
direct wgpu→CAMetalLayer present. The `map_async`/`device.poll` readback in
`gpui_wgpu` (`wgpu_renderer.rs:2130-2201`) is the headless-QA path only and is
never reached from gpui-au's interactive draw.

## Findings

1. **[Roundtrip] Synchronous `device.poll(Wait)` on the resize path** —
   `AuWindow::handle_resize` (`au_window.rs:208-252`) calls
   `renderer.update_drawable_size`, which blocks the UI thread in
   `device.poll(PollType::Wait)` before destroying/recreating intermediate
   textures (`crates/3rdparties/gpui_wgpu/src/wgpu_renderer.rs:952-957`). During
   live resize in a DAW host this stalls the host's main thread per resize
   event. Impact: moderate, resize-only. Fix belongs in gpui_wgpu (defer
   destruction via per-frame retire queue instead of blocking).

2. **[Alloc] Unbounded `layout_cache` in the text system** —
   `AuTextSystemState::layout_cache: HashMap<LayoutCacheKey, Arc<LineLayout>>`
   (`src/text_system/au_text_system_state.rs:138`) inserts on every miss
   (`au_text_system_state.rs:445-453`) and never evicts. Audio-plugin UIs with
   live numeric readouts (dB, Hz, gain) produce distinct strings every frame,
   so the cache grows without bound — each entry holds an `Arc<str>` copy of
   the text plus shaped runs. Impact: slow memory leak proportional to text
   churn. Test at `src/text_system/tests.rs:101-133` asserts caching but not
   boundedness.

3. **[Alloc] Per-glyph double buffer in `rasterize_glyph`** — a fresh
   `vec![0; needed]` is allocated for every rasterized glyph
   (`au_text_system_state.rs:304`) *in addition to* the reusable thread-local
   `GLYPH_BITMAP_SCRATCH` (line 45), then scratch is copied into it
   (lines 414-423). The final `Vec<u8>` must be owned by the caller, but the
   scratch could be written directly into the returned buffer (render into
   `bitmap`, drop the copy) when the cached context matches size. Glyph
   rasterization is atlas-cached upstream, so cost is per-new-glyph, not
   per-frame — still wasteful during first paint / font / size changes.

4. **[Alloc] Per-glyph CTFont and CGColorSpace creation** —
   `rasterize_glyph` creates a new color space
   (`CGColorSpace::create_device_gray/rgb`, lines 315-326) and a
   `clone_with_font_size` CTFont copy (line 396) for every glyph. Both are
   cacheable per (font_id, size) / per colorspace-kind. Same frequency as
   finding 3. Also note the cached-context growth policy (lines 339-359): one
   oversized glyph permanently enlarges the cached context, and every later
   small glyph clears the *full* enlarged scratch (`scratch[..cached_bytes].fill(0)`,
   line 371) and does row-by-row copy (lines 417-422).

5. **[Alloc] Cache-hit path of `layout_line` clones shaped runs** — every cache
   hit returns `Self::clone_layout(cached)` (`au_text_system_state.rs:442`),
   which clones `Vec<ShapedRun>` incl. each run's `Vec<ShapedGlyph>`
   (lines 456-465). The `PlatformTextSystem` trait forces a by-value return, so
   this needs either an upstream trait change (return `Arc<LineLayout>`) or a
   cheaper representation. Impact: one vec clone per laid-out line per frame —
   matters for meter/value-heavy plugin UIs.

6. **[Alloc] `layout_line` takes a write lock even for cache hits** —
   `AuTextSystem::layout_line` uses `self.0.write()`
   (`src/text_system/au_text_system.rs:135`), serializing all text queries;
   `upgradable_read` would keep hits read-locked. Minor contention risk, e.g.
   vs. `rasterize_glyph`'s read lock during atlas fill.

7. **[Alloc] Per-keyevent String churn in FFI** — `key_event` allocates up to
   three `String`s per keystroke (`src/ffi.rs:360-405`: `optional_c_string`,
   `mac_key_code_to_key` incl. `format!("keycode-{key_code}")`). Low frequency
   (human typing), minor — but the named-key match could return `&'static str`.

8. **[GPU] No missed GPU opportunity** — glyph rasterization is CPU
   CoreGraphics, but it is atlas-cached and matches upstream GPUI's macOS text
   stack; moving it to GPU is out of scope. Scene rasterization already goes
   straight to Metal via wgpu with zero copies. The crate's own demo view
   (`AuRootView`, `ffi.rs:28-79`) allocates labels only on click, not per frame.

## Existing perf notes

- CHANGELOG 0.7.5 documents the layout cache addition (`CHANGELOG.md:17-21`).
- Thread-local glyph context/scratch reuse with a creation-count test
  (`tests.rs:136-173`, `au_text_system_state.rs:43-58`).
- Fallback atlas memoized via `OnceLock` (`au_window.rs:27-29`) with a reuse
  test (`au_window.rs:541-547`).
- No criterion benches; no `qa/perf` entries reference gpui-au.
- Hygiene (not perf): `AuWindow::request_frame` is dead code
  (`au_window.rs:195-205`); `ffi.rs:237-243` duplicates its take/replace logic
  inline.

## Recommendations

| # | Action | Finding | Effort | Payoff |
|---|--------|---------|--------|--------|
| 1 | Bound `layout_cache` (LRU cap or clear-at-N entries, e.g. 1024) | 2 | S | Stops slow leak in value-heavy plugin UIs |
| 2 | Render glyphs directly into the returned buffer; drop scratch→bitmap copy | 3 | S | Removes per-glyph alloc + full copy |
| 3 | Cache CTFont-per-size and the two CGColorSpaces | 4 | S | Fewer CF object creations per glyph |
| 4 | `upgradable_read` in `AuTextSystem::layout_line` | 6 | S | Removes text-path serialization |
| 5 | Replace blocking `device.poll(Wait)` on resize with deferred texture retirement (gpui_wgpu) | 1 | M | Removes UI-thread GPU stall during host live-resize |
| 6 | `&'static str` named keys / avoid `format!` per keystroke | 7 | S | Minor alloc reduction |
| 7 | Upstream: `Arc<LineLayout>` from `PlatformTextSystem::layout_line` | 5 | L | Kills per-line clone on every text frame |

## Quick wins

- Cap `layout_cache` (finding 2): ~10 lines, plus a boundedness test next to
  `tests.rs:101`.
- Drop the double buffer in `rasterize_glyph` (finding 3) by drawing into the
  output `Vec` when sizes match — the existing `GLYPH_CONTEXT_CREATE_COUNT`
  test harness already covers regression shape.
- `static` lazy color spaces + a `(FontId, u32-size)` → CTFont map (finding 4).
- Switch `layout_line` to `upgradable_read` (finding 6).
- `&'static str` for named keys in `mac_key_code_to_key` (finding 7).

Finding 1 and finding 5 touch vendored `gpui_wgpu`/`gpui` and should be
scheduled with the upstream-sync process; findings 2-4 and 6-7 are local to
gpui-au and landable independently. All allocation findings beyond finding 2
are per-new-glyph / per-event rather than per-frame; only findings 1, 2 and 5
can become visible in a real host session (needs profiling with gpui-profiler
to rank 5 vs. the atlas upload cost).
