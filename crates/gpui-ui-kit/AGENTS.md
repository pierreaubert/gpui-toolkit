# gpui-ui-kit (lib: `gpui_ui_kit`, version: 0.6.0)

Reusable UI component library for the GPUI framework.

## Key Components

- Button, Input, Slider, Dropdown, Modal, Tabs, Toggle
- Theme system integration
- Node graph editor
- Workflow builder
- Read `GPUI.md` at the project root before working on GPUI code

## FormField Macro

The `FormField` derive macro reduces boilerplate for form component structs by generating:

- `new()` constructor
- Builder pattern setters for each field

### Usage

```rust
use gpui_ui_kit::FormField;

#[derive(FormField)]
pub struct MyInput {
    #[field(required)]           // Required in constructor
    id: ElementId,
    
    #[field(optional, into)]     // Optional field, accepts impl Into<T>
    value: Option<SharedString>,
    
    #[field(optional, into)]
    label: Option<SharedString>,
    
    #[field(default = "false")]  // Custom default value
    disabled: bool,
    
    #[field(builder = false)]    // Skip builder method
    on_change: Option<Box<dyn Fn(&str, &mut Window, &mut App) + 'static>>,
}

// Generated API:
let input = MyInput::new("my-id")
    .value("Hello")
    .label("Name")
    .disabled(true);
```

### Attributes

- `#[field(required)]` - Required field (must be provided in `new()`)
- `#[field(optional)]` - Optional field (wraps in `Some()`)
- `#[field(into)]` - Use `impl Into<T>` for the setter
- `#[field(builder = false)]` - Skip generating builder method
- `#[field(default = "expr")]` - Custom default value expression
- `#[field(skip)]` - Skip field entirely

## Dependencies

- `gpui` - GPU-accelerated UI framework
- `gpui-ui-kit-macros` - Procedural macros for component definitions
- `serde`, `uuid`

## Examples

```bash
cargo run --release --example showcase -p gpui-ui-kit      # Component gallery
cargo run --release --example workflow_debug -p gpui-ui-kit # Workflow editor
```

## Testing

```bash
cargo test -p gpui-ui-kit --lib
cargo check -p gpui-ui-kit && cargo clippy -p gpui-ui-kit
```
