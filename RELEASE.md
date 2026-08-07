# Release Policy

GPUI Toolkit uses three distribution lanes. A tag may contain all workspace
source while making narrower stability and registry claims.

## Distribution lanes

| Lane | Current scope | Promise |
| --- | --- | --- |
| crates.io wave 1 | `gpui-design`, `gpui-profiler`, `gpui-ui-kit-macros` | GPUI-free packages with locked dry-run, Rust 1.89 MSRV, docs, tests, and dependency gates. |
| crates.io deferred | `gpui-pretext`, then `gpui-builder` | Publish only after their registry predecessors exist and locked dry-runs pass. |
| source beta | GPUI-dependent UI, audio, theme, chart, showcase, and tooling crates | Available from the signed/tagged source archive; APIs and platform coverage remain pre-1.0 beta. |
| internal/experimental | aggregate, mobile backends, Audio Unit, scaffolder, and platform delivery artifacts | No registry or production-support claim without the target-specific gates in `qa.md`. |

The machine-readable authorities are
`gpui_toolkit::publish_plan()`, `crate_stability_manifest()`, and
`release_qa_matrix()`. If prose and executable evidence disagree, the release
must stop and the metadata must be corrected.

## Versioning and compatibility

Public crates follow Cargo SemVer conventions. Because versions are below 1.0,
minor releases may contain intentional breaking changes, but those changes
still require changelog entries and migration notes. Patch releases must not
intentionally break documented APIs. The declared MSRV is part of the public
contract and may only increase in a minor release with release-note notice.

## Candidate procedure

1. Start from a clean worktree and reviewed release issue.
2. Run narrow tests, then `just qa`; attach coverage, dependency, performance,
   snapshot/gallery, and platform reports.
   The final QA step writes `target/qa/release-evidence.{json,md}`, which binds
   the required reports and comparator inputs to their source revision, host,
   toolchains, sizes, and SHA-256 digests.
3. Run `just qa-release-contract` and the locked publish dry-runs in the
   machine-readable order. Never use `--allow-dirty` for final evidence.
4. Generate the reproducible RC bundle, SBOM/license inventory, provenance,
   and SHA-256 checksums with `just release-rc <version>`.
5. Review `CHANGELOG.md`, `WHATSNEW.md`, known limitations, support scope, and
   every accepted advisory or manual platform gate.
6. Run `just qa-release-evidence` from the final clean commit. Require any
   attached mobile lane with `scripts/qa_release_evidence.py --require-clean
   --require-platform <lane>` so stale or dirty platform captures are rejected.
7. Create and verify a signed tag. Publishing packages or uploading release
   assets is a separate, explicit maintainer action; the automation does not
   publish to crates.io or create a remote release by itself.

`release-rc` is deliberately offline and refuses a dirty worktree, an invalid
version, a version that differs from `[workspace.package]`, or an existing
output directory. It writes `target/release/gpui-toolkit-<version>-rc/` with:

- a deterministic source archive and a standalone visual gallery containing
  17 renderer contact sheets plus Android emulator and iOS/tvOS simulator captures;
- locked `.crate` archives for the three reviewed wave-1 packages;
- an SPDX 2.3 JSON SBOM and JSON/Markdown license inventory;
- path-free provenance recording the commit, lockfile digest, tool versions,
  and source epoch; and
- `SHA256SUMS` covering every other artifact.

For final evidence, run the command in two fresh worktrees at the same commit
with different output directories and compare every file byte-for-byte.

## Rollback and yanking

Prefer a fixed patch release. Yank a crate version only when it is unusable,
security-sensitive, or materially violates the package contract; record the
reason and replacement version. Never rewrite an existing tag or release
archive.
