# Vendoring Notes: mach2

## Upstream

- Source: `https://github.com/JohnTitor/mach2`
- Local package: `mach2 0.5.0`
- Last reviewed: 2026-07-07

## Build Status

This directory is a local snapshot. The current lockfile resolves `mach2` from
crates.io, not from this directory.

If this local copy is intended to affect builds, add a `[patch.crates-io]` entry
and confirm with:

```sh
cargo tree -i mach2
```

## Why Vendored

This is a snapshot of Mach kernel FFI bindings for macOS/iOS/tvOS platform work.
It is useful when platform fixes need to be staged locally before relying on a
published crate.

## Local Changes

- No intentional local changes are currently documented.
- `AGENTS.md` notes that modifications should be minimized.

## Upgrade Procedure

1. Check whether this snapshot is still needed.
2. If needed, replace it with the target upstream release or commit.
3. Reapply any local patches and document them here.
4. If not needed, consider removing the directory or leaving it explicitly marked
   as inactive.

## Verification

Recommended checks:

```sh
cargo check --manifest-path crates/3rdparties/mach2/Cargo.toml --target aarch64-apple-darwin
cargo check --manifest-path crates/3rdparties/mach2/Cargo.toml --target aarch64-apple-ios
```

## Upstreaming Status

No local changes are documented, so there is nothing currently identified for
upstreaming.
