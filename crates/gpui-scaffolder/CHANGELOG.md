# Unreleased

## Added

- New `python-mobile` template (`--template python-mobile`): generates a
  Python-authored iOS/Android project (`app/main_app.py`,
  `gpui-mobile.toml`, Swift/Gradle shells, preflight/build Just recipes)
  that reuses the toolkit's embedded-CPython mobile hosts, with no
  generated Rust crate.
- Android scaffolds now copy the complete Java host source set
  (previously only two of six files, which no longer compiled).

# 0.11.0

# 0.10.4

## Added

- Reworked the scaffolder library surface and CLI with updated manifests
  and template coverage.

# 0.9.3

## Features

- Added non-mutating scaffold previews that validate options and return the exact desktop, iOS, and Android file plan without writing to disk.

# 0.7.1

## Maintenance

- Version bump; no user-facing changes.
