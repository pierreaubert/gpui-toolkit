# GPUI Toolkit specification: stable element-to-focus mapping

## Problem

GPUI exposes `Window::focused()` and `FocusHandle::is_focused`, but neither
gives automation tools a supported way to identify *which rendered element*
owns the focused handle. Applications therefore cannot make a sound assertion
such as `assert_focused playlist.name_input`: comparing accessibility nodes,
layout bounds, or application state would be indirect and can produce false
passes.

The toolkit needs a frame-scoped, stable mapping from a rendered `ElementId`
to its associated `FocusHandle`, plus a query for the focused element ID.
This is intended for accessibility inspection, UI automation, and test
diagnostics. It must not change normal event dispatch or tab traversal.

## Goals

- Let an application associate an existing `FocusHandle` with an `ElementId`.
- Resolve the currently focused element to that stable `ElementId`.
- Keep the mapping correct across rerenders and remove entries when an element
  is no longer rendered.
- Support nested focus owners without guessing from layout or accessibility.
- Make snapshots deterministic and suitable for a dev-only HTTP API.
- Preserve GPUI's existing focus ownership, containment, and tab-order rules.

## Non-goals

- Replacing `FocusHandle`, `FocusId`, `track_focus`, or `TabStop`.
- Inventing focus for elements that have not opted in.
- Exposing private `FocusId` values as a durable external protocol.
- Changing native accessibility adapters or requiring accessibility to be
  enabled.
- Defining cross-window focus: each query is scoped to one `Window`.

## Proposed public API

The API belongs beside `FocusHandle`/`Window` in GPUI, with thin toolkit
helpers permitted for components.

```rust
impl FocusHandle {
    /// Associate this handle with `element_id` for the current rendered frame.
    /// Calling this repeatedly for the same pair is idempotent.
    pub fn track_element(
        &self,
        element_id: ElementId,
        window: &mut Window,
        cx: &mut App,
    );
}

impl Window {
    /// The element that owns focus in this window, if that focus handle was
    /// associated with an element during the current rendered frame.
    pub fn focused_element_id(&self, cx: &App) -> Option<ElementId>;

    /// Whether a particular rendered element owns keyboard focus.
    pub fn is_element_focused(&self, element_id: &ElementId, cx: &App) -> bool;
}
```

For components that use `track_focus`, add an element-builder convenience
method that makes the association at the same call site:

```rust
div()
    .id("playlist-name")
    .track_focus(&name_focus)
    .track_focus_element(&name_focus)
```

`track_focus_element` is optional syntactic sugar. It must use the element's
actual resolved `ElementId`; callers must not supply a second string ID that
can drift from `.id(...)`.

## Semantics and invariants

1. **Frame-scoped membership.** The element-focus registry is cleared at the
   start of each render/prepaint frame, then rebuilt by rendered elements.
   `focused_element_id` must never return an element absent from the current
   frame.
2. **Stable identity.** Returned IDs are GPUI `ElementId`s, not position,
   accessibility labels, inspector IDs, or opaque focus IDs. The same logical
   control must retain its ID across rerenders.
3. **One focused owner.** A window has at most one focused element result. If
   its focused handle has no current mapping, `focused_element_id` returns
   `None` rather than a stale or approximate match.
4. **Duplicate registration.** Re-registering the same `ElementId` and focus
   handle in one frame is allowed. Registering one `ElementId` to two distinct
   handles in one frame is a programmer error: debug builds should emit a
   diagnostic and the last registration must not silently make focus checks
   nondeterministic.
5. **Nested controls.** Only the handle that equals the window's focused
   handle is returned. A parent that merely `contains_focused` is not reported
   as focused. A future `focused_element_path()` API may model containment;
   it is not required for this change.
6. **Multiple windows.** Mapping and lookup are isolated per `Window`; focus
   in one window cannot resolve to an element registered in another.
7. **Lifecycle safety.** Dropping a `FocusHandle`, closing a window, or
   omitting an element in a later frame must remove its mapping without panic,
   leaked IDs, or stale snapshots.
8. **No event behavior change.** Registration has no effect on focus,
   keyboard dispatch, mouse dispatch, tab traversal, accessibility, or paint
   ordering.

## Suggested implementation shape

Keep a private per-window `HashMap<FocusId, ElementId>` and its inverse for
duplicate validation. During the element's prepaint path, use the element's
resolved ID and its existing focus handle to register the pair. At frame start,
clear/rebuild the map in lockstep with GPUI's existing retained element state.

The public query compares the window's focused `FocusId` to that map. Do not
serialize or expose `FocusId`; it is deliberately opaque and may be reused.

If an element wrapper is used, its `prepaint` implementation must forward
layout/paint behavior unchanged and register only after the inner element has
received the resolved global/element identity.

## Accessibility and automation integration

Extend the bridge snapshot with an optional boolean:

```json
{
  "element": "playlist-name",
  "role": "textbox",
  "label": "Playlist name",
  "focused": true
}
```

`focused` is meaningful only for nodes whose element ID maps to a registered
focus handle; otherwise it is `false`. The snapshot must contain at most one
`focused: true` node per window.

For SOTF's dev API, this enables a direct selector assertion:

```text
click playlist.name_input
assert_focused playlist.name_input
key cmd-a
type "Renamed playlist"
```

The app should consume the GPUI API rather than retain a parallel global focus
registry. This keeps runtime automation evidence aligned with actual keyboard
dispatch.

## Error reporting and diagnostics

- In debug builds, duplicate element-to-distinct-handle registration should
  report the element ID and both registration locations when available.
- A missing focused mapping is normal and returns `None`; do not log it as an
  error.
- Add an inspector/debug view showing the current mapping and focused element
  to simplify application integration.

## Acceptance tests

### Core GPUI tests

1. Register one element/handle, focus it, and assert
   `focused_element_id() == Some(element_id)`.
2. Focus a registered second handle and assert the result changes in the same
   window.
3. Focus an unregistered handle and assert `None`, never the previous ID.
4. Rerender without the previously focused element and assert `None`.
5. Rerender the same stable element ID/handle and assert focus survives.
6. Register the same pair twice and assert no duplicate or instability.
7. Register one ID to different handles and assert a debug diagnostic plus a
   deterministic documented outcome.
8. Create two windows with overlapping element IDs; assert lookup remains
   window-local.
9. Verify tab navigation and key dispatch are unchanged by registration.
10. Verify a bridge snapshot has zero or one focused node and matches
    `focused_element_id`.

### Toolkit component tests

1. A `Button`, `Input`, `Tab`, and custom focusable `div` using the convenience
   method report their actual IDs after pointer focus and keyboard Tab focus.
2. A nested focusable control reports the child, not the containing card.
3. Conditional rendering removes the focused mapping after the next frame.

### SOTF integration proof

1. Click the playlist name input; `assert_focused playlist.name_input` passes.
2. Click a sidebar row; its selector becomes focused and Enter activates it.
3. Open and close a dialog; focus restoration is asserted against its invoking
   selector.
4. Run those checks after resize and rerender to prove IDs are stable.

## Compatibility and rollout

- Ship the core API as additive and feature-neutral.
- Mark component convenience methods as stable only after the core lifecycle
  tests pass on macOS, Linux, and Windows.
- Keep SOTF's `assert_focused` unavailable until its dev API verifies the
  feature at runtime; failing with “focus evidence unavailable” is preferable
  to a false assertion.
- Document that apps must opt in by registering the same focus handle they
  already use for keyboard interaction.
