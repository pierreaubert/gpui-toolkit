# gpui-ui-kit (lib: `gpui_ui_kit`, version: 0.7)

Reusable UI component library for the GPUI framework.

Read `GPUI.md` at the project root before working on GPUI code.

## Key Components

- Button, Input, Slider, Dropdown, Modal, Tabs, Toggle, Select, NumberInput, ColorPicker
- Audio controls live in sibling crate `gpui-audio-kit`
- Data display: Table, Badge, Avatar, Progress, Spinner, QrCode, KeyboardShortcutLabel, EmptyState
- Layout: VStack, HStack, PaneDivider, Sidebar, StatusBar, Accordion, Breadcrumbs
- Navigation: Tabs, Menu, ContextMenu, Wizard
- Feedback: Alert, Toast, Tooltip, Dialog, ConfirmDialog, Popover, SearchBar
- Workflow: Node graph editor
- Theme system with 6 variants, i18n with 5 languages
- Accessibility: ARIA roles, labels, and runtime accessibility tree

## Accessibility

All components support ARIA roles and labels via the `accessibility` module.
GPUI has no native accessibility support; this is a UI-kit-level data layer.

### Adding accessibility to a component

Every component should have:
1. `aria_label: Option<SharedString>` and `aria_role: Option<AriaRole>` fields
2. `.aria_label()` and `.aria_role()` builder methods
3. `cx.register_accessible(...)` at the start of `render()` with a default role

```rust
use crate::accessibility::{AccessibilityExt, AccessibilityNode, AriaProps, AriaRole, AriaState};

// In render():
cx.register_accessible(AccessibilityNode {
    element_id: self.id.clone(),
    label: self.aria_label.clone().unwrap_or_else(|| self.label.clone()),
    props: AriaProps::with_role(self.aria_role.unwrap_or(AriaRole::Button))
        .maybe_state(self.disabled, AriaState::Disabled),
});
```

### Default role mapping

| Component type | Default AriaRole |
|---------------|-----------------|
| Buttons | `Button` |
| Checkboxes | `Checkbox` |
| Toggles | `Switch` |
| Text inputs | `Textbox` |
| Number inputs | `Spinbutton` |
| Sliders/knobs | `Slider` |
| Dropdowns | `Combobox` |
| Dialogs | `Dialog` / `Alertdialog` |
| Toasts (error) | `Alert` + `AriaLive::Assertive` |
| Toasts (other) | `Status` + `AriaLive::Polite` |

### Querying the tree

```rust
let tree = cx.global::<AccessibilityTree>();
for node in tree.nodes_in_order() {
    println!("{:?}: {} (role={:?})", node.element_id, node.label, node.props.role);
}
```

Note: `Button::build()` and similar `build_with_theme()` methods bypass
accessibility registration. Prefer using components via `RenderOnce` for
automatic registration.

## Adding a New Component

When adding a new component, you MUST complete ALL of the following steps. Do not skip any.

### Files to Create/Modify

| # | Action | File |
|---|--------|------|
| 1 | Create component source | `src/<component>.rs` |
| 2 | Register module + re-exports | `src/lib.rs` |
| 3 | Add i18n keys (ALL 5 languages) | `src/i18n.rs` |
| 4 | Create unit tests | `tests/components/<component>_test.rs` |
| 5 | Register unit test module | `tests/components/mod.rs` |
| 6 | Create integration tests | `tests/integration/<component>_test.rs` |
| 7 | Register integration test module | `tests/integration/mod.rs` |
| 8 | Create debug example | `examples/<component>_debug.rs` |
| 9 | Register example in manifest | `Cargo.toml` (`[[example]]`) |
| 10 | Create showcase include | `../gpui-showcase/src/showcase/sections/render_<component>.rs` |
| 11 | Register in showcase | `../gpui-showcase/src/showcase.rs` |
| 12 | Update README | `README.md` (component table + usage example) |

### Step 1: Component Source (`src/<component>.rs`)

Follow these conventions exactly:

- **Builder pattern**: All setters return `Self`
- **Enums for variants/sizes**: `#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]`
- **Event handlers**: `Option<Box<dyn Fn(...) + 'static>>`
- **Rendering**: Implement `RenderOnce` + `IntoElement`
- **Theming**: Use `cx.theme()` via `ThemeExt` trait for colors
- **FormField macro**: Use for form components to auto-generate constructor + setters

```rust
use gpui::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MyWidgetVariant {
    #[default]
    Default,
    Primary,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MyWidgetSize {
    Sm,
    #[default]
    Md,
    Lg,
}

pub struct MyWidget {
    id: ElementId,
    label: SharedString,
    variant: MyWidgetVariant,
    size: MyWidgetSize,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl MyWidget {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: MyWidgetVariant::Default,
            size: MyWidgetSize::Md,
            disabled: false,
            on_click: None,
        }
    }

    pub fn variant(mut self, variant: MyWidgetVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: MyWidgetSize) -> Self {
        self.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for MyWidget {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        use crate::theme::ThemeExt;
        let theme = cx.theme();
        div().id(self.id).child(self.label.clone())
    }
}

impl IntoElement for MyWidget {
    type Element = <Self as RenderOnce>::Element;
    fn into_element(self) -> Self::Element {
        self.into_any_element()
    }
}
```

### Step 2: Register in `src/lib.rs`

Add in the appropriate section:

```rust
// Under the matching category comment:
pub mod my_widget;

// Under the matching re-export section:
pub use my_widget::{MyWidget, MyWidgetSize, MyWidgetVariant};
```

### Step 3: i18n Keys in `src/i18n.rs`

You MUST add translations for ALL 5 languages. Missing any language will cause test failures.

1. Add `TranslationKey` variant:
   ```rust
   SectionMyWidget,  // under "Section titles"
   ```

2. Add to ALL language functions (`add_english`, `add_french`, `add_german`, `add_spanish`, `add_japanese`):
   ```rust
   t.insert(TranslationKey::SectionMyWidget, "My Widget");       // English
   t.insert(TranslationKey::SectionMyWidget, "Mon Widget");       // French
   t.insert(TranslationKey::SectionMyWidget, "Mein Widget");      // German
   t.insert(TranslationKey::SectionMyWidget, "Mi Widget");        // Spanish
   t.insert(TranslationKey::SectionMyWidget, "マイウィジェット");    // Japanese
   ```

### Step 4: Unit Tests (`tests/components/<component>_test.rs`)

Test API compilation and configuration. Pattern: create component, verify it compiles.

```rust
//! MyWidget component tests

use gpui_ui_kit::my_widget::{MyWidget, MyWidgetSize, MyWidgetVariant};

#[test]
fn test_my_widget_creation() {
    let widget = MyWidget::new("test", "Label");
    let _ = widget;
}

#[test]
fn test_my_widget_variants() {
    for variant in [MyWidgetVariant::Default, MyWidgetVariant::Primary] {
        let widget = MyWidget::new("test", "Label").variant(variant);
        let _ = widget;
    }
}

#[test]
fn test_my_widget_sizes() {
    for size in [MyWidgetSize::Sm, MyWidgetSize::Md, MyWidgetSize::Lg] {
        let widget = MyWidget::new("test", "Label").size(size);
        let _ = widget;
    }
}

#[test]
fn test_my_widget_disabled() {
    let widget = MyWidget::new("test", "Label").disabled(true);
    let _ = widget;
}

#[test]
fn test_my_widget_with_click_handler() {
    let widget = MyWidget::new("test", "Label")
        .on_click(|_window, _cx| {});
    let _ = widget;
}

#[test]
fn test_my_widget_full_configuration() {
    let widget = MyWidget::new("test", "Label")
        .variant(MyWidgetVariant::Primary)
        .size(MyWidgetSize::Lg)
        .disabled(false)
        .on_click(|_window, _cx| {});
    let _ = widget;
}
```

Register in `tests/components/mod.rs`:
```rust
mod my_widget_test;
```

### Step 5: Integration Tests (`tests/integration/<component>_test.rs`)

Test rendering in actual GPUI windows using `#[gpui::test]` and `TestAppContext`.

```rust
//! Integration tests for MyWidget component

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::my_widget::{MyWidget, MyWidgetSize, MyWidgetVariant};

struct MyWidgetTestView;

impl Render for MyWidgetTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(MyWidget::new("test", "Hello"))
    }
}

#[gpui::test]
async fn test_my_widget_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| MyWidgetTestView);
}

#[gpui::test]
async fn test_my_widget_all_variants(cx: &mut TestAppContext) {
    struct AllVariantsView;
    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(MyWidget::new("a", "Default").variant(MyWidgetVariant::Default))
                .child(MyWidget::new("b", "Primary").variant(MyWidgetVariant::Primary))
        }
    }
    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

#[gpui::test]
async fn test_my_widget_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;
    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(MyWidget::new("a", "Sm").size(MyWidgetSize::Sm))
                .child(MyWidget::new("b", "Md").size(MyWidgetSize::Md))
                .child(MyWidget::new("c", "Lg").size(MyWidgetSize::Lg))
        }
    }
    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

#[gpui::test]
async fn test_my_widget_disabled(cx: &mut TestAppContext) {
    struct DisabledView;
    impl Render for DisabledView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(MyWidget::new("test", "Disabled").disabled(true))
        }
    }
    let _window = cx.add_window(|_window, _cx| DisabledView);
}
```

Register in `tests/integration/mod.rs`:
```rust
mod my_widget_test;
```

### Step 6: Showcase Include (`../gpui-showcase/src/showcase/sections/render_<component>.rs`)

Add a new renderer module in the `gpui-showcase` crate:

```rust
use super::prelude::*;

impl Showcase {
    fn render_my_widget_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionMyWidget);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(MyWidget::new("default", "Default"))
                    .child(MyWidget::new("primary", "Primary").variant(MyWidgetVariant::Primary)),
            )
    }
}
```

Also register the module in `../gpui-showcase/src/showcase/sections/mod.rs`.

### Step 7: Register in Showcase (`../gpui-showcase/src/showcase.rs`)

Three changes needed:

```rust
// 1. Add to ShowcaseSection enum:
pub enum ShowcaseSection {
    // ...existing...
    MyWidget,
}

// 2. Add to ShowcaseGroup::sections() where the component belongs:
ShowcaseSection::MyWidget,

// 3. Add match arm in render_section_content():
ShowcaseSection::MyWidget => self.render_my_widget_section(cx).into_any_element(),
```

### Step 8: Verify Everything

```bash
cargo check -p gpui-ui-kit --all-targets
cargo clippy -p gpui-ui-kit --all-targets
cargo test -p gpui-ui-kit --lib --tests
cargo run -p gpui-showcase --release
cargo fmt -p gpui-ui-kit
```

All tests must pass. The showcase must display your component.

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

## NumberInput Text Editing in App Context

NumberInput uses `RenderOnce` with thread-local state for editing. Three mechanisms interact to make text editing work inside a full application:

### 1. Key Binding Bypass

GPUI dispatches action key bindings **before** `on_key_down` handlers. If the app binds `-`→VolumeDown or `enter`→Enter, those bindings consume keystrokes before NumberInput sees them.

The app uses `is_text_input_mode()` to switch the root element's `key_context` from `"PlayerView"` to `"TextInput"`, which prevents `PlayerView`-scoped bindings from matching. NumberInput exposes `is_number_input_editing()` (thread-local check) so the app can include NumberInput editing in this check:

```rust
pub(crate) fn is_text_input_mode(input_mode: InputMode) -> bool {
    input_mode.is_text_input() || gpui_ui_kit::is_number_input_editing()
}
```

### 2. Focus Handle Stability Across Re-renders

`focus_handle.is_focused(window)` returns **false** during `RenderOnce::render()` because the old element is destroyed before the new one calls `.track_focus()`. Never use `is_focused()` during render to gate editing state — it will always clear it. Instead, trust the thread-local `state.editing` flag and use `window.on_focus_out()` for actual blur detection.

### 3. Parent `on_key_down` Wrappers

Never wrap a NumberInput (or Input) in a parent `div().on_key_down(|..| cx.stop_propagation())`. In GPUI, `on_key_down` handlers on parent elements in the dispatch path fire **before** the focused child's handler (capture phase in `dispatch_key_down_up_event`). The parent's `stop_propagation` prevents the NumberInput from receiving any keystrokes. NumberInput already calls `cx.stop_propagation()` in its own handler, which prevents further bubbling to global handlers.

## Dependencies

- `gpui` - GPU-accelerated UI framework
- `gpui-ui-kit-macros` - Procedural macros for component definitions
- `serde`, `uuid`

## Examples

```bash
cargo run -p gpui-showcase --release                       # Component gallery
cargo run --release --example workflow_debug -p gpui-ui-kit # Workflow editor
```

## Testing

```bash
cargo test -p gpui-ui-kit --lib --tests   # All tests
cargo test --test component_tests         # Unit tests only
cargo test --test integration_tests       # Integration tests only
cargo test --test i18n_tests              # i18n tests only
cargo check -p gpui-ui-kit && cargo clippy -p gpui-ui-kit
```
