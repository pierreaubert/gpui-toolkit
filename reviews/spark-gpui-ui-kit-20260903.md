# Code Review: gpui-ui-kit — 2026-09-03

Date: 2026-09-03 | Reviewer: Muse Spark | Scope: `crates/gpui-ui-kit` (~187 files, ~42.4k LOC)

## 1. Purpose / role
`div()`-native widget kit: forms, navigation, data display, workflow canvas, plus animation, i18n, theme and ARIA integration. Largest areas: `workflow/canvas/workflow_canvas.rs`, `table.rs`, `input.rs`, `tabs.rs`, `select.rs`, `thinking_orb/engine/*`, `color_tokens/*`, `i18n/*`, `accessibility.rs`, `focus.rs`.

Public API (top): `Button/ButtonVariant/ButtonSize`, `Input/NumberInput/Slider/Select/Checkbox/Toggle`, `Dialog/ConfirmDialog/Popover/Toast/Tooltip`, `Tabs/Menu/MenuBar`, `Table/Column/SortState/PaginationState`, `TreeView/CommandPalette/DragList`, `WorkflowCanvas/WorkflowGraph/CanvasState/ViewportState`, `VStack/HStack/Divider/Spacer`, `Theme/ThemeState`, `I18nState/Language`, `AriaRole/AccessibilityTree/register_accessible`, `Animation/Easing/Spring/KeyframeAnimation`.

## 2. SOTA gap analysis (vs Radix / shadcn / MUI v6 / SwiftUI / Material 3 / ArkUI)
1. **No RTL/bidi mirroring.** Only `SplitDirection::Horizontal` in `split_pane.rs:9,187`; no mirrored spacing/corner tokens. SOTA requires logical properties + RTL golden tests.
2. **Virtualization is table-only** (`table.rs:46,136-154` `virtual_window`). `tree_view.rs`, `drag_list.rs` render full lists — need `LazyVStack`-style windowing for 10k+ rows.
3. **No focus-trap / modal inert.** No `focus_trap|aria_modal|inert` hits; `dialog.rs` does not trap Tab. Add Radix-Dialog parity: trap, restore focus, Escape, `aria-modal`.
4. **Ad-hoc form validation.** `input.rs:3` mentions validation but no schema (zod/RHF parity), no async errors, no error-message plumbing. Add `Input::validate` + schema derive.
5. **Missing Radix primitives.** No Combobox/Autocomplete, DatePicker/Calendar, RadioGroup, `Separator`/`ScrollArea`/`Resizable`. Each is a top docs-site request.
6. **CPU-only animation** (`animation.rs:198-201` keyframe eval). No compositor spring-interrupt model, no `prefers-reduced-motion` gate — SwiftUI/M3 motion parity missing.
7. **A11y is data-only.** `accessibility.rs:1054-1061` writes a per-render `AccessibilityTree` global with no OS screen-reader bridge. Needs platform bridge (cf. `gpui-ios/accessibility.rs`).
8. **No prop-docs / controls metadata.** Props are not introspectable for Storybook-style knobs (see `gpui-component-lab` review).

## 3. Performance evaluation
- **God builds, untested:** `table.rs:256` `build()` 444 lines/cyclo 61/fan-out 112; `tabs.rs:245` 384 lines/fan-out 116; `workflow_canvas.rs:981` `render()` 440 lines/fan-out 85; `input.rs:599` `handle_key_down()` 326 lines/cyclo 91/CRAP 8372. All `test_covered: false`. Coverage ~23% crate-wide.
- **Per-build clones:** 103 `.clone()` across `table.rs+input.rs+tabs.rs+select.rs`, e.g. `table.rs:264,280,296,300,311,334-335` clone id/columns/handlers per cell/row.
- **Thread-local input state on hot path:** `input.rs:80`, `number_input.rs:63-76` (4 maps); every keystroke does `NUMBER_INPUT_*.with()` + `HashMap` lookup (`number_input.rs:977-1018`). Replace with `Entity`/`Model` state.
- **Global per-render:** `accessibility.rs:1055-1056` `global_mut::<AccessibilityTree>()` + `theme_state.rs:47-48` `try_global` on every themed render.
- **Workflow drag churn:** `Arc::new(self.state.graph.clone())` (`workflow_canvas.rs:993`) + obstacle scan O(conns × nodes) (`:1053-1061`) per mousemove; bezier flatten + lyon tessellation per paint (`draw.rs:40-64`); hit-test re-flattens all curves (`hit_test.rs:92-96`). QR paints one `paint_quad` per dark module (`qr/paint.rs:35-53`) — up to ~15k primitives/frame.

## 4. Recommendations (prioritized)
| # | Action | Effort | Payoff |
|---|--------|--------|--------|
| 1 | Split `table/tabs/select` builds into header/body/cell helpers; add golden + allocation-contract tests | M | kills largest CRAP scores |
| 2 | Replace thread-local edit maps with GPUI `Entity` state | M | removes HashMap from keystroke path |
| 3 | Virtualize `TreeView`/`DragList`; cache workflow flattened polylines keyed on (from,to,zoom); AABB pre-reject in hit-test | M | O(n²)→O(visible) |
| 4 | Rasterize QR matrix to cached bitmap, paint single image quad; `Arc<[QrColor]>` in animated path | S | O(modules²)→O(1) |
| 5 | Add focus-trap + RTL mirroring + schema validation + Combobox/DatePicker (SOTA close) | M–L | docs-site parity |
| 6 | Diff (don't rebuild) `AccessibilityTree`; gate animations on reduced-motion | S | a11y/motion parity |

## 5. Verdict
Broadest kit in the workspace but render functions carry the highest complexity. SOTA path: virtualization + focus/RTL + validation + 4 missing primitives. Perf path: split god builds, drop per-frame clones/globals, cache curves/QR bitmaps.
