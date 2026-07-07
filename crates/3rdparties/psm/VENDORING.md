# Vendoring Notes: psm

## Upstream

- Source: `https://github.com/rust-lang/stacker`
- Upstream crate: `psm`
- Local package: `psm 0.1.30`
- Last reviewed: 2026-07-07

## Build Status

This directory is a local snapshot. The current lockfile resolves `psm 0.1.31`
from crates.io, not from this directory.

If this local copy is intended to affect builds, add a `[patch.crates-io]` entry
and confirm with:

```sh
cargo tree -i psm
```

## Why Vendored

This is a snapshot of Portable Stack Manipulation, used for stack manipulation
and introspection in recursive or stack-sensitive code. It is useful when
platform-specific stack fixes need to be staged locally.

## Local Changes

- No intentional local changes are currently documented.
- This snapshot is older than the registry version currently used by the
  lockfile.

## Upgrade Procedure

1. Check whether this snapshot is still needed.
2. If needed, replace it with the target upstream release or commit.
3. Reapply any local platform patches and document them here.
4. If not needed, consider removing the directory or leaving it explicitly marked
   as inactive.

## Verification

Recommended checks:

```sh
cargo test --manifest-path crates/3rdparties/psm/Cargo.toml
cargo check --manifest-path crates/3rdparties/psm/Cargo.toml
```

Also run any downstream crate that depends on stacker/psm behavior.

## Upstreaming Status

No local changes are documented, so there is nothing currently identified for
upstreaming.
