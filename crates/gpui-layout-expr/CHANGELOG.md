# Unreleased

# 0.10.33

## Added

- Extended the grammar Astryx-XLE-compatibly: `Hd` headings with
  `[level=1-4]`, text size modifiers (`Tx.lg`), text `weight`/`muted`
  attrs, and stack `spacing=`/`gN` gaps (shared `stack_spacing_of` /
  `heading_level_of` resolvers for both emitters). Custom-component
  modifier presets now cover text sizes as well as button variants.
- Initial release: compact layout-expression AST, parser, and registry
  validation (`V > (Tx"Hi" + B.primary"Save"#save)`), canonical
  re-serialization, and named custom-component aliases resolved before
  validation. Pure `std` with no GPUI dependencies, shared by the
  `gpui-toolkit` CLI expander and the `layout!` proc macro.
