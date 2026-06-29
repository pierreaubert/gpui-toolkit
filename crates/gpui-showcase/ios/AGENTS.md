# gpui-showcase-ios

iOS showcase binary for gpui-ui-kit components.

## Architecture

Static library (`crate-type = ["staticlib"]`) compiled for iOS, linked into a Swift iOS app. Demonstrates gpui-ui-kit components running on iOS via gpui-ios.

- `src/lib.rs` — FFI entry point exposing the showcase UI to Swift

## Dependencies

- `gpui` — GPUI framework
- `gpui-ios` — iOS platform backend
- `gpui-ui-kit` — UI component library
- `log` / `oslog` — Logging to iOS system log

## Testing

This is a showcase binary — test by building and running on an iOS device/simulator.

```bash
cargo check -p gpui-showcase-ios
```

## Important Notes

- Produces a static library, not a standalone binary — must be linked into a Swift iOS project
- Uses `oslog` for iOS system log integration (visible in Console.app/Xcode)
- Published: false (internal showcase only)
