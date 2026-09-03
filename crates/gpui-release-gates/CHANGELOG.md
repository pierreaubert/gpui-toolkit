# Unreleased

## Added

- Added the `gate_execution` module: allowlisted `cargo publish --dry-run`
  gate commands now parse to shell-free argv and execute with captured
  output, so CI refreshes packaging evidence instead of trusting stale
  strings. Manual-action rows fail closed to `None`.
