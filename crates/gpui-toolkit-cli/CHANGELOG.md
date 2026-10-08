# Unreleased

## Added

- Initial release: unified agent-ready `gpui-toolkit` CLI. Commands
  cover the component catalog (`component` with batch reads, `search`),
  page/block/theme templates, theme token export and freshness checks,
  `doctor` health checks as a CI gate, `upgrade` migration notes plus
  deprecated-pattern detection, `toolkit.toml` project configuration,
  layout expression check/expand, and a self-describing `manifest`.
  Every command honors `--json` with typed `{apiVersion, type, data}`
  envelopes and stable append-only `ERR_*` error codes.
