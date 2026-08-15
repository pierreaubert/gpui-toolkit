# ----------------------------------------------------------------------
# WASM / browser builds (WebGPU-only)
# Design: docs/superpowers/specs/2026-08-15-wasm-browser-target-design.md
# wasm builds need nightly (-Z build-std + parking_lot/nightly), injected via
# env so the stable host toolchain is untouched.
# ----------------------------------------------------------------------

wasm_target := "wasm32-unknown-unknown"
wasm_packages := "-p gpui_web -p gpui-miniapp -p gpui-hello-web"

[group('wasm')]
wasm-setup:
	rustup toolchain install nightly --component rust-src --target {{wasm_target}}
	rustup target add {{wasm_target}}

[group('wasm')]
wasm-check:
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort cargo check --target {{wasm_target}} {{wasm_packages}}

[group('wasm')]
wasm-serve-hello:
	cd crates/gpui-hello-web && RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve
