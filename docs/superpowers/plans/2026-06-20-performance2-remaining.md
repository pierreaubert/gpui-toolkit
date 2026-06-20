# Performance2 Remaining Items Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the remaining performance2 action-plan item 3 gaps in `gpui-ui-kit` by persisting the `WorkflowCanvas` entity and removing the per-frame unit clone in the number-input format cache.

**Architecture:** Follow the existing example pattern in `crates/gpui-ui-kit/examples/showcase.rs` by storing `workflow_canvas` as a persistent `Entity<WorkflowCanvas>` field on `Showcase`. Update the entity through its existing public API (`add_node`, `clear`, `stats`). For `NumberInput`, keep the existing `(value, decimals, unit)` cache but avoid cloning the unit when the cache already matches.

**Tech Stack:** Rust, GPUI, `gpui-ui-kit` workspace crate.

---

## Task 1: Persist `WorkflowCanvas` in `Showcase`

**Files:**
- Modify: `crates/gpui-ui-kit/src/showcase.rs:126-131` (struct fields)
- Modify: `crates/gpui-ui-kit/src/showcase.rs:160-247` (`Showcase::new`)
- Modify: `crates/gpui-ui-kit/src/showcase.rs:646-667` (Clear button handler)
- Modify: `crates/gpui-ui-kit/src/showcase.rs:586-698` (`render_workflow_section`)
- Modify: `crates/gpui-ui-kit/src/showcase.rs:700-712` (`workflow_add_node`)

- [ ] **Step 1: Remove `workflow_graph` field and add persistent canvas entity**

In `crates/gpui-ui-kit/src/showcase.rs`, replace the workflow state fields:

```rust
    // Workflow state (simple graph, no persistent Entity)
    pub workflow_graph: WorkflowGraph,
    pub workflow_node_counter: usize,
```

with:

```rust
    // Persistent workflow canvas entity; graph state lives inside it.
    pub workflow_canvas: Entity<WorkflowCanvas>,
    pub workflow_node_counter: usize,
```

- [ ] **Step 2: Initialize the persistent canvas entity in `Showcase::new`**

In `Showcase::new`, replace:

```rust
        let workflow_graph = WorkflowGraph::new();
        let entity = cx.entity().clone();
```

with:

```rust
        let workflow_canvas = cx.new(|cx| WorkflowCanvas::with_graph(WorkflowGraph::new(), cx));
        let entity = cx.entity().clone();
```

And replace the struct initialization line:

```rust
            workflow_graph,
            workflow_node_counter: 0,
```

with:

```rust
            workflow_canvas,
            workflow_node_counter: 0,
```

- [ ] **Step 3: Update `workflow_add_node` to mutate the persistent canvas**

Replace the current implementation:

```rust
    fn workflow_add_node(&mut self, _cx: &mut Context<Self>) {
        self.workflow_node_counter += 1;
        let id = self.workflow_node_counter;

        let x = 100.0 + (id as f32 * 30.0) % 400.0;
        let y = 100.0 + (id as f32 * 20.0) % 300.0;

        let node = WorkflowNodeData::new(format!("Node {}", id), Position::new(x, y))
            .with_ports(1, 1)
            .with_size(160.0, 70.0);

        self.workflow_graph.add_node(node);
    }
```

with:

```rust
    fn workflow_add_node(&mut self, cx: &mut Context<Self>) {
        self.workflow_node_counter += 1;
        let id = self.workflow_node_counter;

        let x = 100.0 + (id as f32 * 30.0) % 400.0;
        let y = 100.0 + (id as f32 * 20.0) % 300.0;

        let node = WorkflowNodeData::new(format!("Node {}", id), Position::new(x, y))
            .with_ports(1, 1)
            .with_size(160.0, 70.0);

        self.workflow_canvas.update(cx, |canvas, _cx| {
            canvas.add_node(node);
        });
    }
```

- [ ] **Step 4: Update the Clear button handler to clear the canvas entity**

In `render_workflow_section`, replace the Clear button `on_click` closure:

```rust
                                                move |_, cx| {
                                                    entity.update(cx, |this, _cx| {
                                                        this.workflow_graph = WorkflowGraph::new();
                                                        this.workflow_node_counter = 0;
                                                    });
                                                }
```

with:

```rust
                                                move |_, cx| {
                                                    entity.update(cx, |this, cx| {
                                                        this.workflow_canvas.update(cx, |canvas, cx| {
                                                            canvas.clear(cx);
                                                        });
                                                        this.workflow_node_counter = 0;
                                                    });
                                                }
```

- [ ] **Step 5: Use the persistent canvas entity in `render_workflow_section`**

Replace the body of `render_workflow_section` so it no longer creates a new entity. Remove:

```rust
        // Create a WorkflowCanvas entity on-the-fly from the stored graph
        let graph = self.workflow_graph.clone();
        let workflow_canvas = cx.new(|cx| WorkflowCanvas::with_graph(graph, cx));

        // Get stats from canvas
        let (node_count, connection_count, _selected_count) = workflow_canvas.read(cx).stats();
```

and replace with:

```rust
        // Get stats from the persistent canvas entity
        let (node_count, connection_count, _selected_count) = self.workflow_canvas.read(cx).stats();
```

Then replace the canvas child render:

```rust
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .child(workflow_canvas)
                    )
```

with:

```rust
                    .child(
                        div()
                            .flex_1()
                            .relative()
                            .child(self.workflow_canvas.clone())
                    )
```

- [ ] **Step 6: Check compilation and fix any import/borrow issues**

Run:

```bash
cargo check -p gpui-ui-kit
```

Expected: no errors. If `WorkflowCanvas` or `WorkflowGraph` imports are unused or missing, adjust the `use` statements at the top of `showcase.rs` accordingly.

---

## Task 2: Avoid Unit Clone in `NumberEditState` Format Cache

**Files:**
- Modify: `crates/gpui-ui-kit/src/number_input/number_edit_state.rs:41-64`

- [ ] **Step 1: Rewrite `format_value_str` to compare by reference before building owned key**

Replace:

```rust
    pub(super) fn format_value_str(
        &mut self,
        value: f64,
        decimals: usize,
        unit: Option<&SharedString>,
    ) -> SharedString {
        let key = (value, decimals, unit.cloned());
        if self.last_format_key.as_ref() == Some(&key) {
            return self.last_format_value.clone();
        }

        let formatted = format!("{:.prec$}", value, prec = decimals);
        let result: SharedString = if let Some(unit) = unit {
            format!("{} {}", formatted, unit).into()
        } else {
            formatted.into()
        };

        self.last_format_key = Some(key);
        self.last_format_value = result.clone();
        result
    }
```

with:

```rust
    pub(super) fn format_value_str(
        &mut self,
        value: f64,
        decimals: usize,
        unit: Option<&SharedString>,
    ) -> SharedString {
        // Fast path: compare the incoming unit by reference against the stored
        // key so we do not clone the SharedString on every cache hit.
        let cache_hit = self
            .last_format_key
            .as_ref()
            .map(|(last_value, last_decimals, last_unit)| {
                *last_value == value
                    && *last_decimals == decimals
                    && last_unit.as_ref() == unit
            })
            .unwrap_or(false);

        if cache_hit {
            return self.last_format_value.clone();
        }

        let formatted = format!("{:.prec$}", value, prec = decimals);
        let result: SharedString = if let Some(unit) = unit {
            format!("{} {}", formatted, unit).into()
        } else {
            formatted.into()
        };

        self.last_format_key = Some((value, decimals, unit.cloned()));
        self.last_format_value = result.clone();
        result
    }
```

- [ ] **Step 2: Run existing number-input tests**

Run:

```bash
cargo test -p gpui-ui-kit number_input
```

Expected: all tests pass, including `format_value_str_caches_result`.

---

## Task 3: Verify the Full Crate

- [ ] **Step 1: Run the full test suite for `gpui-ui-kit`**

Run:

```bash
cargo test -p gpui-ui-kit
```

Expected: all tests pass.

- [ ] **Step 2: Run clippy (optional but recommended)**

Run:

```bash
cargo clippy -p gpui-ui-kit -- -D warnings
```

Expected: no warnings related to the changed code.

---

## Self-Review Checklist

1. **Spec coverage:**
   - Persist `WorkflowCanvas` in `Showcase` → Task 1.
   - Remove per-frame unit clone in number-input format cache → Task 2.
   - `Tabs` left as-is (already uses `cx.listener`) → documented in design spec.

2. **Placeholder scan:**
   - No TBD/TODO/fill-in-details patterns.
   - Exact file paths and line ranges are provided.
   - Code blocks contain complete replacement code.

3. **Type consistency:**
   - `workflow_canvas` type is `Entity<WorkflowCanvas>` in struct, initialization, and usage.
   - `workflow_add_node` signature remains `fn(&mut self, &mut Context<Self>)`.
   - `format_value_str` signature and return type unchanged.
