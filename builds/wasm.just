# ----------------------------------------------------------------------
# WASM / browser builds (WebGPU-only)
# Design: docs/superpowers/specs/2026-08-15-wasm-browser-target-design.md
# wasm builds need nightly (-Z build-std + parking_lot/nightly), injected via
# env so the stable host toolchain is untouched.
# ----------------------------------------------------------------------

wasm_target := "wasm32-unknown-unknown"
wasm_packages := "-p gpui_web -p gpui-miniapp -p gpui-hello-web -p gpui-showcase -p gpui-px -p gpui-d3rs"

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

[group('wasm')]
wasm-serve-showcase:
	cd crates/gpui-showcase && RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve

[group('wasm')]
wasm-serve-px:
	cd crates/gpui-px && RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve

# Prereqs: `cargo install wasm-bindgen-cli` (provides wasm-bindgen-test-runner)
# and a chromedriver on PATH (`brew install chromedriver`); set the CHROMEDRIVER
# env var if the driver lives somewhere non-standard.
[group('wasm')]
wasm-test:
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target {{wasm_target}} -p gpui-hello-web
