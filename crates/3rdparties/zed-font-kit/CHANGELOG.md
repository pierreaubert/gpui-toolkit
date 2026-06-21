# 0.14.1-zed (sotf vendored fork)

This is a vendored fork of Zed's [font-kit](https://github.com/zed-industries/font-kit) fork. Only changes specific to the SOTF workspace are tracked here; refer to upstream for the canonical history.

## Changes

- Fixed broken `core-text` imports introduced by the upstream Zed git rev so the crate builds again on macOS.
- Extended the Apple-platform `cfg` gates to include tvOS / watchOS / visionOS so those targets keep the CoreText backend instead of pulling in fontconfig/freetype.
