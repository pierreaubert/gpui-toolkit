# gpui-ui-kit-ios-showcase

iOS showcase for the gpui-ui-kit component library.

## What It Does

Demonstrates all gpui-ui-kit UI components running natively on iOS. Compiled as a static library and linked into a Swift iOS app, providing a visual gallery of buttons, inputs, sliders, and other components rendered via Metal on iOS devices.

## Features

- **Native iOS rendering**: Components rendered via GPUI's Metal backend on iOS
- **Full component gallery**: All gpui-ui-kit components demonstrated
- **System log integration**: Logging visible in Xcode/Console.app via oslog

## Building

This crate produces a static library that must be linked into a Swift iOS project:

```bash
# Check compilation
cargo check -p gpui-ui-kit-ios-showcase

# Build for iOS target
cargo build -p gpui-ui-kit-ios-showcase --target aarch64-apple-ios
```

## Architecture

```
ios/
├── Cargo.toml  # Static library configuration
└── src/
    └── lib.rs  # FFI entry point for Swift
```

The Swift iOS app links the static library and calls the FFI entry point to initialize the GPUI rendering surface and display the showcase.

## License

Part of the SOTF (Sound of the Future) project.
