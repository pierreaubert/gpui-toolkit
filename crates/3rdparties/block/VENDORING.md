# block Vendoring Notes

## Upstream

- Project: https://github.com/SSheldon/rust-block
- Base crate: `block` 0.1.6 from crates.io
- License: MIT as declared by the upstream crate manifest.
- Local status: active `[patch.crates-io]` dependency.

## Why This Is Vendored

`block` 0.1.6 declares the Objective-C runtime symbol
`_NSConcreteStackBlock` as a Rust static with an uninhabited `Class` type.
Current Rust accepts this with a future-incompatibility warning, but Cargo
reports that it will become a hard error in a future compiler.

The workspace uses `block` through Zed/GPUI's Objective-C and Cocoa dependency
chain, so the release cannot leave this warning invisible.

## Local Changes

- Keep the upstream `Class` marker for block layout pointers.
- Declare `_NSConcreteStackBlock` as an opaque `u8` extern static.
- Store the block `isa` pointer with `ptr::addr_of!(_NSConcreteStackBlock).cast::<Class>()`.
- Use explicit `extern "C"` ABI spellings for block invoke function pointers.
- Add a local `[workspace]` table so the vendored crate can be checked directly
  without being treated as an undeclared member of the repository workspace.
- Remove the packaged `objc_test_utils` dev-dependency entry because crates.io
  excludes the referenced `test_utils` helper crate.
- Make the upstream default Rust 2015 edition explicit so direct Cargo checks
  do not emit an edition-default warning.

## Upgrade Guidance

1. Check whether upstream `block` or GPUI's dependency stack has removed the
   `block` 0.1.x dependency.
2. If this crate is still required, reapply only the opaque-symbol change above.
3. Run:
   - `cargo check --manifest-path crates/3rdparties/block/Cargo.toml --lib`
   - a dependent GPUI/toolkit check
   - `cargo report future-incompatibilities --id <build-id>` if Cargo emits a
     future-incompatibility report.
4. Remove this vendored patch if the dependency is no longer present or the
   upstream crate carries an equivalent fix.
