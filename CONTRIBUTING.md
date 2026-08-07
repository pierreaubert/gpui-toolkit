# Contributing to GPUI Toolkit

Thank you for improving GPUI Toolkit. Bug reports, focused fixes, tests,
documentation, examples, and accessibility or platform evidence are welcome.

## Before opening a change

Open an issue for behavior changes, public API additions, new dependencies,
vendored-code changes, or work spanning several crates. Small documentation
and test fixes may go directly to a pull request. Never include credentials,
private data, proprietary source, or licensed assets you cannot redistribute.

## Development workflow

1. Use a focused branch and keep unrelated changes out of the patch.
2. Add or update tests with behavior changes. UI changes should add a component
   story and renderer-backed snapshot case when practical.
3. Run the narrow crate tests first, then `just qa-api` and the relevant QA
   recipe from [qa.md](qa.md). Maintainers run the complete `just qa` release
   gate before merging or tagging.
4. Run `cargo fmt` only on files you changed and keep Clippy warning-free.
5. Update `CHANGELOG.md` for user-visible behavior, compatibility, or security
   changes.

## Public API and compatibility

The project follows Cargo SemVer conventions. A breaking API change requires
an issue, migration notes, and an intentional version decision. The registry
release set is narrower than the source tree; see [RELEASE.md](RELEASE.md).
Do not make a crate publishable by hiding unresolved platform or dependency
constraints.

## Vendored code

Changes below `crates/3rdparties/` must update that crate's `VENDORED.md` or
`VENDORING.md`, the vendored-patch manifest, its owner/removal condition, and
the recorded verification gate. Prefer changes that can be upstreamed.

## Review expectations

Pull requests should explain the problem, the chosen tradeoff, validation
performed, platform limitations, and any visual changes. By contributing, you
agree that your contribution is licensed under this repository's ISC license.
