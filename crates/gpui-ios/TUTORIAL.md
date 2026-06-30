# gpui-ios Tutorial

`gpui-ios` is the iOS and tvOS platform backend for GPUI.

## 1. Add the crate

```toml
[dependencies]
gpui = { workspace = true }
gpui-ios = { workspace = true }
```

For a complete showcase, use the nested app crate:

```bash
just ios-sim
```

## 2. Understand the layers

- `IosPlatform` adapts GPUI to UIKit/Metal.
- `native` exposes bridge reports and scene metrics.
- `platform_view` lets Swift-provided views appear inside GPUI.
- `pencil`, `momentum`, and text input modules handle mobile interaction.
- `hot_reload` supports simulator reload workflows.

## 3. Build for Apple targets

```bash
cargo build -p gpui-ios
cargo build -p gpui-showcase-ios --target aarch64-apple-ios-sim --release
```

tvOS uses nightly with `-Zbuild-std`:

```bash
just tvos-sim
```

## 4. Use keyboard and text input hooks

`gpui-ios` exposes helpers such as `show_keyboard`,
`show_keyboard_with_type`, `hide_keyboard`, `keyboard_height`, and
`set_text_input_callback`.

```rust
use gpui_ios::{KeyboardType, show_keyboard_with_type};

show_keyboard_with_type(KeyboardType::Default);
```

## 5. Verify

```bash
cargo check -p gpui-ios
just ios-rust-sim
```

Use Xcode for final simulator/device validation.
