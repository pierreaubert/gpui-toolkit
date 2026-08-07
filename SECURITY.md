# Security Policy

## Supported versions

Security fixes are provided for the latest tagged release candidate or public
release. Older pre-release tags and untagged source snapshots are supported on
a best-effort basis. Platform backends marked experimental in the release
matrix do not carry a production-support claim.

## Reporting a vulnerability

Please use GitHub's private security-advisory reporting for this repository:
`Security` → `Advisories` → `Report a vulnerability`. Do not open a public
issue for a suspected vulnerability and do not include secrets, personal data,
or proprietary code in a report.

Include the affected crate/version, target platform, impact, reproduction or
proof of concept, and any known mitigation. Maintainers aim to acknowledge a
report within 5 business days and will coordinate validation, a fix, release
timing, and credit with the reporter. Timelines vary with severity and
platform access; no embargo date should be assumed until agreed in writing.

For dependency advisories, include the RustSec identifier and resolved
dependency path when available. The repository's accepted advisory exceptions
are documented in `deny.toml` and the dependency-hygiene report.
