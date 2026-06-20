# Performance2 Remaining Items — Design Spec

## Goal
Complete the remaining performance work surfaced in `performance2.md` so that the high/medium-impact items in the UI crate are no longer rebuilding per frame.

## Background
`performance2.md` audits the workspace after the first performance pass. Static analysis and recent commits show that 13 of the 14 action-plan items are already implemented. The remaining gaps are concentrated in **item 3** (stateful `Render` entity refactor) of `gpui-ui-kit`:

| Area | Status | Impact |
|------|--------|--------|
| `Showcase::render_workflow_section` creates a new `WorkflowCanvas` entity every render | Not done | High |
| `NumberInputEntity::render` calls `format_value_str` every frame (cache exists but key comparison still clones unit) | Partial | Medium-High |
| `TabsEntity::render` builds fresh `on_mouse_down` closures per tab each render | Partial | Medium |

## Design

### 1. Persist `WorkflowCanvas` in `Showcase`
`crates/gpui-ui-kit/src/showcase.rs` currently clones `self.workflow_graph` and creates `cx.new(|cx| WorkflowCanvas::with_graph(graph, cx))` on every render. This rebuilds the entire canvas entity and its handlers every frame.

The standalone example (`crates/gpui-ui-kit/examples/showcase.rs`) already demonstrates the target pattern: store the canvas as a persistent `Entity<WorkflowCanvas>` field and mutate it through its public API.

Changes to `src/showcase.rs`:
- Remove `pub workflow_graph: WorkflowGraph`.
- Add `workflow_canvas: Entity<WorkflowCanvas>`.
- Initialize the entity in `Showcase::new` with an empty graph.
- Change `workflow_add_node` to increment the counter and call `self.workflow_canvas.update(|canvas, _cx| canvas.add_node(node))`.
- Change the Clear button handler to call `self.workflow_canvas.update(|canvas, cx| canvas.clear(cx))`.
- In `render_workflow_section`, read stats from `self.workflow_canvas.read(cx).stats()` and render `self.workflow_canvas.clone()` as the child.

This keeps the public `workflow_add_node` behavior unchanged from the caller’s point of view and avoids the per-frame `cx.new` allocation.

### 2. Remove unit clone from `NumberEditState` format cache key
`NumberEditState::format_value_str` already caches the formatted string keyed by `(value, decimals, unit)`. On a cache hit it still performs `unit.cloned()` to build the comparison key. We can avoid that clone by comparing the incoming `Option<&SharedString>` against the stored `Option<SharedString>` by reference before constructing the owned key.

Changes to `crates/gpui-ui-kit/src/number_input/number_edit_state.rs`:
- Restructure the fast path so the owned key is only built on a cache miss.
- Keep the cache fields and public signature unchanged.
- Existing tests for caching continue to pass.

### 3. `Tabs` — keep current listener pattern
`TabsEntity::render` uses `cx.listener(move |this, event, window, cx| this.handle_tab_click(index, event, window, cx))` for each tab. `cx.listener` already binds to the entity rather than capturing per-render closure state, so the allocation is a thin wrapper. The marginal win from a dedicated `Tab` sub-entity is small relative to the refactor cost and the risk of breaking tab keyboard navigation / dynamic tab lists. We leave `Tabs` as-is and document that it follows the recommended `cx.listener` pattern.

## Testing
- `cargo check -p gpui-ui-kit` must pass.
- `cargo test -p gpui-ui-kit` must pass, including existing `number_input` cache tests.
- The `gpui-ui-kit` showcase example already uses the persistent canvas pattern and can be used as a behavioral reference.

## Files changed
- `crates/gpui-ui-kit/src/showcase.rs`
- `crates/gpui-ui-kit/src/number_input/number_edit_state.rs`
