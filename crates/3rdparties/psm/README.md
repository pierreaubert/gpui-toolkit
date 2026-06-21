# psm (sotf vendored fork)

Vendored fork of [psm](https://github.com/rust-lang/stacker) — portable stack manipulation primitives used transitively via stacker.

This crate is excluded from the SOTF workspace (`exclude` in the root `Cargo.toml`); it ships only as a `[patch.crates-io]` override so the assembly directives compile on Apple Tier-3 platforms (tvOS / watchOS / visionOS).

The full upstream documentation is in [README.mkd](README.mkd). See also [AGENTS.md](AGENTS.md) and [CHANGELOG.md](CHANGELOG.md) for the SOTF-specific patch notes.
