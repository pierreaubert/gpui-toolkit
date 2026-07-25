# Unreleased

# 0.1.1

## Fixed

- Replaced zero-length vector truncation with `Vec::clear` for compatibility
  with Rust 1.97's `clippy::manual_clear` lint under `-D warnings`.
