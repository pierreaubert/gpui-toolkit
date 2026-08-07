# Unsafe Rust policy

GPUI Toolkit keeps portable first-party Rust free of `unsafe` code. Crates
that do not own a native ABI boundary should declare `#![forbid(unsafe_code)]`
and use safe standard-library or dependency APIs instead of local raw-pointer
or FFI wrappers.

This is an enforcement boundary, not a claim that the complete dependency
graph contains no unsafe code. Rust's standard library, GPUI, platform crates,
and other audited dependencies may use unsafe code internally.

## Audited boundaries

Unsafe Rust is permitted only where first-party code translates between Rust
and a native platform ABI. The current directory boundaries are:

- `crates/gpui-android/`
- `crates/gpui-au/`
- `crates/gpui-ios/`
- `crates/gpui-showcase/android/`
- `crates/gpui-showcase/ios/`
- `crates/gpui-showcase/tvos/`

The macOS camera authorization shim in
`crates/gpui-ui-kit/examples/qr_debug/misc.rs` is the only file-level
exception. `crates/gpui-scaffolder/src/lib.rs` may contain FFI attributes as
generated source text, but its executable code remains protected by
`#![forbid(unsafe_code)]`.

The executable allowlist in `scripts/qa_unsafe_policy.py` is authoritative.
Keeping it narrower than an entire product or utility crate makes additions
easy to identify and review.

## Portable code and safe wrappers

Portable crates should move native operations behind an existing safe crate
when possible. For example, the Python showcase uses `security-framework` for
macOS Keychain access instead of declaring Security.framework functions, and
`gpui-profiler` delegates its global allocator implementation to
`stats_alloc`. Both first-party crates forbid unsafe code.

## Vendored code

Sources under `crates/3rdparties/` are excluded from the first-party scanner
because they are imported projects with their own safety architecture. They
are pinned, patched, and reviewed through the vendored-dependency governance
checks. A local modification that introduces or changes an unsafe block must
receive the same safety review as first-party FFI code and be recorded with
the vendored patch.

## Adding or changing an exception

Prefer a maintained safe dependency or isolate the operation in an existing
platform backend. If first-party unsafe code is unavoidable, a change must:

1. Keep the exception to the smallest practical file or native-backend
   directory and update the policy allowlist explicitly.
2. Document every unsafe block's invariants, including pointer validity,
   ownership, lifetime, thread, callback, and ABI assumptions that apply.
3. Add focused tests for safe-call behavior, error paths, and repeated cleanup
   or destruction where relevant.
4. Include a reviewer-visible rationale explaining why a safe wrapper is not
   suitable and how the boundary was audited.

Do not add a broad crate exemption merely to make the policy check pass.

## Enforcement

Run the focused policy checks with:

```bash
PYTHONPATH=scripts python3 -m unittest scripts.tests.test_qa_unsafe_policy
python3 scripts/qa_unsafe_policy.py
```

`just qa-scripts` runs these checks in the standard QA lane. Crate-level
`#![forbid(unsafe_code)]` attributes provide a second compiler-enforced layer
for portable crates such as `gpui-profiler` and `gpui-python-runtime`.
