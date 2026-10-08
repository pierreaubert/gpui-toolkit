# Unreleased

## Added

- Initial release: compact layout-expression AST, parser, and registry
  validation (`V > (Tx"Hi" + B.primary"Save"#save)`), canonical
  re-serialization, and named custom-component aliases resolved before
  validation. Pure `std` with no GPUI dependencies, shared by the
  `gpui-toolkit` CLI expander and the `layout!` proc macro.
