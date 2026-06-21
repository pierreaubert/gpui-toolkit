# 0.5.0 (sotf vendored fork)

This is a vendored fork of upstream [mach2](https://github.com/JohnTitor/mach2). Only changes specific to the SOTF workspace are tracked here; refer to upstream for the canonical history.

## Changes

- Loosened the `compile_error!` target gate so tvOS / watchOS / visionOS targets compile alongside iOS.
- Re-arranged into the workspace `crates/3rdparties/` tree so the platform-specific patches stay isolated from upstream master.
