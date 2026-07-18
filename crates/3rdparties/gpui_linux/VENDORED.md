# Vendored: gpui_linux

- Upstream: https://github.com/zed-industries/zed/tree/v1.9.0/crates/gpui_linux
- Base ref: v1.9.0
- Import: scripts/import_gpui_upstream.py (history-free snapshot)
- Excluded on import: examples/, benches/, deps on gpui_platform, gpui_web, reqwest_client, zlog, ztracing, ztracing_macro

## Local patches

- The standalone manifest sets `gpui` to `default-features = false`, matching
  Zed v1.9.0's workspace dependency policy. `gpui-miniapp` explicitly enables
  the Linux `wayland` and `x11` backend features.
- The `image` dependency is restricted to GPUI's advertised bitmap formats,
  avoiding codecs that the GPUI `ImageFormat` API cannot request.
