# gpui-au Tutorial

`gpui-au` is the macOS Audio Unit platform backend for embedding GPUI rendering
inside AUv3 view controllers.

## 1. Add the crate

Inside this workspace, depend on it through the root workspace table:

```toml
[dependencies]
gpui-au = { workspace = true }
gpui = { workspace = true }
```

## 2. Understand the role

Use `gpui-au` when a host environment owns the macOS view lifecycle and GPUI
must render into that host-provided surface. The crate exposes `AuPlatform` and
FFI support used by an AU shell.

## 3. Build the library

```bash
cargo build -p gpui-au
cargo build -p gpui-au --target aarch64-apple-darwin --release
```

## 4. Integrate with an AU host

1. Let the AU extension create or receive the native view controller.
2. Initialize the GPUI platform through `AuPlatform`.
3. Pass pending view information through the FFI boundary.
4. Keep audio processing outside the UI thread.
5. Use GPUI only for editor visualization and interaction.

## 5. Verify

```bash
cargo check -p gpui-au
```

If you add FFI entry points, also validate the consuming Swift or Objective-C
target because most integration failures happen at the host boundary.

Use `include/gpui_au.h` as the single ABI source for lifecycle, pointer,
keyboard, and `NSTextInputClient` forwarding. Key down/up callbacks are not a
substitute for `insertText`, `setMarkedText`, and `unmarkText`: AppKit sends
IME/dead-key composition through the text-input client, and forwarding both
paths is required for international text entry.
