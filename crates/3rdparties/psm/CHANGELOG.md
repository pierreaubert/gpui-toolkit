# 0.1.30 (sotf vendored fork)

This is a vendored fork of upstream [psm](https://github.com/rust-lang/stacker). Only changes specific to the SOTF workspace are tracked here; refer to upstream for the canonical history.

## Changes

- Patched the assembly files so they fall back to ELF directives instead of Mach-O when running on Apple Tier-3 targets (tvOS / watchOS / visionOS) the upstream build does not recognise.
- Re-arranged into the workspace `crates/3rdparties/` tree so the patches stay isolated from upstream master.
