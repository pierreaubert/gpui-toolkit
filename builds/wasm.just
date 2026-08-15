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

# Screenshot a served wasm app and diff against qa/visual/wasm/baselines/<name>.png.
# Positional args: `just wasm-visual hello 8080 gpui-hello-web` (defaults: showcase).
# Append `record` as a 4th arg to (re)write the baseline instead of comparing.
[group('wasm')]
wasm-visual name="showcase" port="8081" dir="gpui-showcase" record="":
	#!/usr/bin/env bash
	set -euo pipefail
	record_flag=""
	[ -n "{{record}}" ] && record_flag="--record"
	cd crates/{{dir}}
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort env -u NO_COLOR trunk serve --port {{port}} --open=false &
	server=$!
	trap 'kill $server' EXIT
	cd ../..
	for i in $(seq 1 60); do curl -sf "http://127.0.0.1:{{port}}" > /dev/null && break; sleep 2; done
	uv run --with pillow --with playwright python3 scripts/qa_wasm_screenshot.py --url "http://127.0.0.1:{{port}}" --name {{name}} $record_flag

# Prereqs: `cargo install wasm-bindgen-cli` (provides wasm-bindgen-test-runner)
# and a chromedriver on PATH (`brew install chromedriver`); set the CHROMEDRIVER
# env var if the driver lives somewhere non-standard.
[group('wasm')]
wasm-test:
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target {{wasm_target}} -p gpui-hello-web
