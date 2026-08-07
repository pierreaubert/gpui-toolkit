# Support Policy

GPUI Toolkit is pre-1.0 software. The latest public tag receives bug fixes and
security updates; older tags receive best-effort help only.

Use GitHub Issues for reproducible defects and feature requests. Include the
crate and version, operating system and architecture, Rust version, enabled
features, a minimal reproduction, expected and actual behavior, and relevant
logs. Use the private process in [SECURITY.md](SECURITY.md) for vulnerabilities.

The crates.io lane, source-beta lane, and experimental platform backends have
different guarantees. Exact scope is recorded in [RELEASE.md](RELEASE.md),
`gpui_toolkit::crate_stability_manifest()`, and the release QA matrix. A build
on one desktop platform is not evidence of mobile, accessibility, installer,
Audio Unit, or app-store readiness.
