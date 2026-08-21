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
	cd crates/gpui-hello-web && env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve

[group('wasm')]
wasm-serve-showcase:
	cd crates/gpui-showcase && env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve

[group('wasm')]
wasm-serve-px:
	cd crates/gpui-px && env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve

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
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort env -u NO_COLOR trunk serve --port {{port}} --open=false --no-autoreload &
	server=$!
	trap 'kill $server' EXIT
	cd ../..
	server_up=""
	for i in $(seq 1 60); do
		if curl -fsS --max-time 5 "http://127.0.0.1:{{port}}" > /dev/null; then server_up=1; break; fi
		sleep 2
	done
	if [ -z "$server_up" ]; then
		echo "error: trunk on port {{port}} did not become ready within ~120s" >&2
		exit 1
	fi
	uv run --with pillow --with playwright python3 scripts/qa_wasm_screenshot.py --url "http://127.0.0.1:{{port}}" --name {{name}} $record_flag ${click_flags[@]+"${click_flags[@]}"}

# Build the three browser demos beneath target/demo-site so the generated
# gallery can publish both snapshots and live WASM routes.
[group('wasm')]
wasm-build-site:
	#!/usr/bin/env bash
	set -euo pipefail
	mkdir -p target/demo-site/hello target/demo-site/showcase target/demo-site/px
	cd crates/gpui-hello-web
	env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk build --release --dist ../../target/demo-site/hello
	cd ../gpui-showcase
	env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk build --release --dist ../../target/demo-site/showcase
	cd ../gpui-px
	env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk build --release --dist ../../target/demo-site/px

# Capture every query-addressable showcase section, then generate thumbnails,
# contact sheets, manifest.json, and the static gallery index.
[group('wasm')]
wasm-gallery only="":
	#!/usr/bin/env bash
	set -euo pipefail
	pids=()
	cleanup() {
		trap - EXIT INT TERM
		for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
		for pid in "${pids[@]}"; do wait "$pid" 2>/dev/null || true; done
	}
	trap cleanup EXIT INT TERM
	(
		cd crates/gpui-showcase
		env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve --port 8081 --open=false --no-autoreload
	) & pids+=("$!")
	(
		cd crates/gpui-px
		env -u NO_COLOR RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort trunk serve --port 8082 --open=false --no-autoreload
	) & pids+=("$!")
	for port in 8081 8082; do
		server_up=""
		for _ in $(seq 1 60); do
			page="$(curl -fsS --max-time 5 "http://127.0.0.1:$port" || true)"
			# Trunk serves the source index even when its build pipeline failed;
			# a successful build replaces data-trunk links with generated scripts.
			if [ -n "$page" ] && ! printf '%s' "$page" | grep -q 'data-trunk'; then
				server_up=1
				break
			fi
			sleep 2
		done
		if [ -z "$server_up" ]; then
			echo "wasm-gallery: Trunk on port $port did not produce a built page" >&2
			exit 1
		fi
	done
	only_flags=()
	if [ -n "{{only}}" ]; then only_flags+=(--only "{{only}}"); fi
	uv run --with pillow --with playwright python3 scripts/build_wasm_demo_gallery.py \
		--capture \
		--url showcase=http://127.0.0.1:8081 \
		--url px=http://127.0.0.1:8082 \
		${only_flags[@]+"${only_flags[@]}"} \
		--readme-snippet target/demo-site/README-snippet.md

# Generate the static site from the Rust manifests without requiring a browser.
[group('wasm')]
demo-site: wasm-build-site
	uv run --with pillow python3 scripts/build_wasm_demo_gallery.py \
		--readme-snippet target/demo-site/README-snippet.md

# Prereqs: `cargo install wasm-bindgen-cli` (provides wasm-bindgen-test-runner)
# and a chromedriver on PATH (`brew install chromedriver`); set the CHROMEDRIVER
# env var if the driver lives somewhere non-standard.
[group('wasm')]
wasm-test:
	RUSTUP_TOOLCHAIN=nightly CARGO_UNSTABLE_BUILD_STD=std,panic_abort CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner cargo test --target {{wasm_target}} -p gpui-hello-web
