# zed-font-kit (lib: `zed_font_kit`)

**Vendored 3rd-party crate** -- Zed's fork of [font-kit](https://github.com/servo/font-kit).

Cross-platform font loading library. Provides font discovery, loading, and rasterization using platform-native backends (CoreText on macOS/iOS, DirectWrite on Windows, FreeType on Linux).

## Important Notes

- This is a vendored upstream crate -- minimize modifications
- Platform backends: CoreText (macOS/iOS/tvOS), DirectWrite (Windows), FreeType (Linux)
- Used by GPUI for font rendering
