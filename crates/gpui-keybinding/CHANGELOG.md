# Unreleased

## New

- Added an opaque VSCode-style `when`-clause `context` on `DocumentedKeybinding`
  (`with_context` / `normalized_context`); conflict detection groups by key and
  context, and command-palette search indexes the context text.
- Added `keybindings.json`-style user overrides: `parse_user_overrides`,
  `serialize_user_overrides`, and description-matched `apply_user_overrides`.

## Performance

- `CommandPaletteEntry` lowercases each field once into `Rc<str>` instead of
  twice into `String`, and clones share the lowered buffers.
- Uncached/cached palette search share one matching implementation; per-preset
  query caches evict a single entry instead of clearing all entries at the cap.
- Chord-prefix parsing splits the prefix once per call instead of per binding.

# 0.9.6 - 2026-08-23

## Performance

- Bounded key-discovery cache growth and reduced keybinding lookup/hash hot-path work.

# 0.7.3

## Maintenance

- Version bump; no user-facing changes.

# 0.6.0

## New

- Added a markdown editor as a demo for gpui-toolkit
