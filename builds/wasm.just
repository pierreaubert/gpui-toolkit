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
# Append X Y as 5th/6th args to click viewport coordinates before capture
# (in-app navigation for canvas-rendered UI), e.g.
# `just wasm-visual px-scatter 8082 gpui-px '' 80 137`.
[group('wasm')]
wasm-visual name="showcase" port="8081" dir="gpui-showcase" record="" click_x="" click_y="":
	#!/usr/bin/env bash
	set -euo pipefail
	record_flag=""
	[ -n "{{record}}" ] && record_flag="--record"
	click_flags=()
	if [ -n "{{click_x}}" ] || [ -n "{{click_y}}" ]; then
		[ -n "{{click_x}}" ] && [ -n "{{click_y}}" ] || { echo "wasm-visual: click_x and click_y must be passed together" >&2; exit 1; }
		click_flags=(--click-xy "{{click_x}}" "{{click_y}}")
	fi
	cd crates/{{dir}}
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort env -u NO_COLOR trunk serve --port {{port}} --open=false &
	server=$!
	trap 'kill $server' EXIT
	cd ../..
	server_up=""
	for i in $(seq 1 60); do
		if curl -sf "http://127.0.0.1:{{port}}" > /dev/null; then server_up=1; break; fi
		sleep 2
	done
	if [ -z "$server_up" ]; then
		echo "error: trunk on port {{port}} did not become ready within ~120s" >&2
		exit 1
	fi
	uv run --with pillow --with playwright python3 scripts/qa_wasm_screenshot.py --url "http://127.0.0.1:{{port}}" --name {{name}} $record_flag ${click_flags[@]+"${click_flags[@]}"}

# Prereqs: `cargo install wasm-bindgen-cli` (provides wasm-bindgen-test-runner)
# and a chromedriver on PATH (`brew install chromedriver`); set the CHROMEDRIVER
# env var if the driver lives somewhere non-standard.
[group('wasm')]
wasm-test:
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target {{wasm_target}} -p gpui-hello-web
