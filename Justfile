# --------------------------------------------------------- -*- just -*-
# GPUI Toolkit workspace tasks
# ----------------------------------------------------------------------

set dotenv-load := true

default:
	just --list

import 'builds/linux.just'
import 'builds/windows.just'
import 'builds/cross.just'

# ----------------------------------------------------------------------
# VARIABLES
# ----------------------------------------------------------------------

features := "--features autoeq,camera,gpu-2d,gpu-3d,reqwest,showcase,spinorama,tokio,urlencoding"
cross_packages := "-p gpui-audio-kit -p gpui-builder -p gpui-component-lab -p gpui-d3rs -p gpui-design -p gpui-design-tools -p gpui-keybinding -p gpui-miniapp -p gpui-pretext -p gpui-px -p gpui-python-runtime -p gpui-scaffolder -p gpui-themes -p gpui-ui-kit -p gpui-ui-kit-macros"

# QA / coverage settings
cov_threshold := "90"
cov_ignore_regex := '.*/(tests|benches|examples|target|crates/3rdparties|crates/gpui-au|crates/gpui-ios|crates/gpui-miniapp|crates/.*/bin)/.*'
cov_summary := "target/qa/cov/summary.json"
cov_report := "target/qa/cov/report.md"
perf_baseline := "qa/perf/baseline.json"
perf_current := "target/qa/perf/current.json"
perf_report := "target/qa/perf/report.md"
perf_threshold := "10"
perf_noise_floor_ns := "150"

# ----------------------------------------------------------------------
# TEST / QA
# ----------------------------------------------------------------------

[group('check')]
check:
	cargo check --workspace --all-targets {{features}}

[group('lint')]
lint: lint-host lint-ios-rust

[group('lint')]
lint-host:
	cargo clippy --workspace --all-targets {{features}} -- -D warnings

[group('lint')]
lint-ios-rust:
	RUSTFLAGS="-D warnings" cargo build -p gpui-ui-kit-ios-showcase --target aarch64-apple-ios-sim --release {{features}}

alias clippy := lint

[group('test')]
test-examples:
	@echo "Building gpui-ui-kit examples..."
	cargo build --examples -p gpui-ui-kit {{features}}
	@echo "All gpui-ui-kit examples compiled successfully"

[group('test')]
ntest:
	cargo nextest run --release --no-fail-fast --workspace {{features}}

[group('qa')]
qa-gpui-conformance:
	mkdir -p target/gpui-conformance
	cargo test -p gpui-design-tools {{features}}
	cargo test -p gpui-component-lab {{features}}
	cargo run -p gpui-design-tools --bin gpui-validate-design-tokens {{features}} -- --report-json target/gpui-conformance/design-tokens.json --report-markdown target/gpui-conformance/design-tokens.md
	cargo run -p gpui-component-lab --bin gpui-component-lab {{features}} -- --conformance --report-json target/gpui-conformance/component-lab.json --report-markdown target/gpui-conformance/component-lab.md

[group('qa')]
qa-gpui-obvious: qa-gpui-conformance
	cargo test -p gpui-ui-kit-macros {{features}}
	cargo test -p gpui-builder {{features}}
	cargo check -p gpui-builder {{features}}
	cargo test -p gpui-audio-kit {{features}}
	cargo test -p gpui-ui-kit {{features}}
	cargo test -p gpui-d3rs {{features}}
	cargo test -p gpui-px {{features}}
	cargo tree -p gpui-design-tools {{features}}

# ----------------------------------------------------------------------
# QA SUITE
# ----------------------------------------------------------------------

# Full QA aggregator. Runs non-coverage checks first; coverage gate is last so a
# sub-90% report still lets the other suites exercise the code.
[group('qa')]
qa: qa-prop qa-visual qa-perf qa-gpui-obvious qa-cov-check
	@echo "Full QA suite passed"

# Property-based non-regression. If no proptest tests exist, this exits cleanly.
[group('qa')]
qa-prop:
	@echo "Running property-based tests..."
	bash scripts/qa_prop_check.sh

# Visual non-regression (manifest/golden/conformance; pixel diff is a Phase 1+ stub).
[group('qa')]
qa-visual:
	@echo "Running visual non-regression checks..."
	bash scripts/qa_visual_capture.sh

# Generate a full workspace coverage report.
[group('qa')]
qa-cov:
	mkdir -p target/qa/cov
	cargo llvm-cov --workspace --all-targets --html --output-dir target/qa/cov/html {{features}} --ignore-filename-regex '{{cov_ignore_regex}}'
	cargo llvm-cov --workspace --all-targets --json --summary-only --output-path {{cov_summary}} {{features}} --ignore-filename-regex '{{cov_ignore_regex}}'
	@echo "Coverage report: target/qa/cov/html/index.html"

# Open the HTML coverage report (macOS).
[group('qa')]
qa-cov-html: qa-cov
	open target/qa/cov/html/index.html

# Coverage gate: fails if aggregate coverage is below {{cov_threshold}}%.
[group('qa')]
qa-cov-check:
	@echo "Running coverage gate (threshold {{cov_threshold}}%)..."
	mkdir -p target/qa/cov
	cargo llvm-cov --workspace --all-targets --json --summary-only --output-path {{cov_summary}} {{features}} --ignore-filename-regex '{{cov_ignore_regex}}'
	python3 scripts/qa_cov_check.py --summary {{cov_summary}} --threshold {{cov_threshold}} --output {{cov_report}}

# Update the committed performance baseline. Run intentionally after benchmarking.
[group('qa')]
qa-perf-update:
	@echo "Updating performance baseline..."
	python3 scripts/qa_perf_baseline.py --output {{perf_baseline}}

# Performance non-regression against the committed baseline.
[group('qa')]
qa-perf:
	@echo "Running performance non-regression checks..."
	python3 scripts/qa_perf_baseline.py --output {{perf_current}}
	# Phase 0: --warn-only keeps the gate informational while the baseline stabilizes.
	python3 scripts/qa_perf_check.py --baseline {{perf_baseline}} --current {{perf_current}} --threshold {{perf_threshold}} --noise-floor-ns {{perf_noise_floor_ns}} --warn-only --output {{perf_report}}

# ----------------------------------------------------------------------
# FORMAT / BUILD
# ----------------------------------------------------------------------

alias format := fmt
alias build := prod

[group('format')]
fmt:
	cargo fmt --all

[group('build')]
dev:
	cargo build --workspace {{features}}

[group('build')]
prod: prod-workspace

[group('build')]
prod-workspace:
	cargo build --release --workspace {{features}}

# ----------------------------------------------------------------------
# DEMOS
# ----------------------------------------------------------------------

[group('demo')]
demo: demo-audio-kit demo-builder demo-component-lab demo-d3rs demo-px demo-python demo-themes demo-ui-kit

[group('demo')]
demo-audio-kit:
	cargo build --release --examples -p gpui-audio-kit {{features}}

[group('demo')]
demo-ui-kit:
	cargo build --release --examples -p gpui-ui-kit {{features}}

[group('demo')]
demo-builder:
	cargo build --release --bin layout-showcase -p gpui-builder {{features}}

[group('demo')]
demo-component-lab:
	cargo build --release --bin gpui-component-lab -p gpui-component-lab {{features}}

[group('demo')]
demo-d3rs:
	cargo build --release --bin d3rs-showcase -p gpui-d3rs {{features}}
	cargo build --release --bin d3rs-spinorama -p gpui-d3rs {{features}}
	cargo build --release --examples -p gpui-d3rs {{features}}

[group('demo')]
demo-px:
	cargo build --release --bin px-showcase -p gpui-px {{features}}
	cargo build --release --bin px-spinorama -p gpui-px {{features}}
	cargo build --release --examples -p gpui-px {{features}}

[group('demo')]
demo-python:
	cargo build --release --bin gpui-python-showcase -p gpui-python-runtime {{features}}

[group('demo')]
demo-themes:
	cargo build --release --bin theme-showcase -p gpui-themes {{features}}

# Build all maintained examples.
[group('examples')]
examples: examples-audio-kit examples-builder examples-d3rs examples-px examples-ui-kit
	@echo "All examples compiled successfully"

[group('examples')]
examples-audio-kit:
	@echo "Building gpui-audio-kit examples..."
	cargo build --examples -p gpui-audio-kit {{features}}
	@echo "gpui-audio-kit examples compiled successfully"

[group('examples')]
examples-builder:
	@echo "Building gpui-builder examples..."
	cargo build --examples -p gpui-builder {{features}}
	@echo "gpui-builder examples compiled successfully"

[group('examples')]
examples-d3rs:
	@echo "Building gpui-d3rs examples..."
	cargo build --examples -p gpui-d3rs {{features}}
	@echo "gpui-d3rs examples compiled successfully"

[group('examples')]
examples-px:
	@echo "Building gpui-px examples..."
	cargo build --examples -p gpui-px {{features}}
	@echo "gpui-px examples compiled successfully"

[group('examples')]
examples-ui-kit:
	@echo "Building all gpui-ui-kit examples..."
	cargo build --examples -p gpui-ui-kit {{features}}
	@echo "All gpui-ui-kit examples compiled successfully"

# Run QR debug example. On macOS this builds a small app bundle so the camera
# permission prompt has Info.plist metadata.
[group('examples')]
[macos]
run-qr-debug:
	cargo build -p gpui-ui-kit --example qr_debug {{features}}
	rm -rf target/debug/examples/QrDebug.app
	mkdir -p target/debug/examples/QrDebug.app/Contents/MacOS
	cp target/debug/examples/qr_debug target/debug/examples/QrDebug.app/Contents/MacOS/qr_debug
	cp crates/gpui-ui-kit/examples/qr_debug.plist target/debug/examples/QrDebug.app/Contents/Info.plist
	echo -n "APPL????" > target/debug/examples/QrDebug.app/Contents/PkgInfo
	codesign --force --deep --sign - target/debug/examples/QrDebug.app
	open target/debug/examples/QrDebug.app

[group('examples')]
[linux]
[windows]
run-qr-debug:
	cargo run -p gpui-ui-kit --example qr_debug {{features}}

# ----------------------------------------------------------------------
# IOS
# ----------------------------------------------------------------------

alias ios-rust-sim := showcase-rust-sim
alias ios-rust-device := showcase-rust-device
alias ios-build-rust-sim := showcase-build-rust-sim
alias ios-build-rust-device := showcase-build-rust-device
alias ios-xcodegen := showcase-xcodegen
alias ios-build-sim := showcase-build-sim
alias ios-build-device := showcase-build-device
alias ios-hot-reload := showcase-hot-reload

# Build Showcase iOS Rust static library for simulator.
[group('ios')]
showcase-rust-sim:
	cargo build -p gpui-ui-kit-ios-showcase --target aarch64-apple-ios-sim --release {{features}}

# Build Showcase iOS Rust static library for device.
[group('ios')]
showcase-rust-device:
	cargo build -p gpui-ui-kit-ios-showcase --target aarch64-apple-ios --release {{features}}

# Build Showcase iOS Rust lib and copy it to the Xcode project.
[group('ios')]
showcase-build-rust-sim: showcase-rust-sim
	#!/usr/bin/env bash
	set -euo pipefail
	IOS_DIR="crates/gpui-ui-kit/ios"
	mkdir -p "$IOS_DIR/lib"
	cp target/aarch64-apple-ios-sim/release/libshowcase_ios.a "$IOS_DIR/lib/"
	echo "Copied libshowcase_ios.a to $IOS_DIR/lib/"

# Build Showcase iOS Rust lib for device and copy it to the Xcode project.
[group('ios')]
showcase-build-rust-device: showcase-rust-device
	#!/usr/bin/env bash
	set -euo pipefail
	IOS_DIR="crates/gpui-ui-kit/ios"
	mkdir -p "$IOS_DIR/lib"
	cp target/aarch64-apple-ios/release/libshowcase_ios.a "$IOS_DIR/lib/"
	echo "Copied libshowcase_ios.a to $IOS_DIR/lib/"

# Generate the Showcase iOS Xcode project with XcodeGen.
[group('ios')]
showcase-xcodegen:
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-ui-kit/ios
	if [ ! -d "GPUIShowcase.xcodeproj" ] || [ "project.yml" -nt "GPUIShowcase.xcodeproj/project.pbxproj" ]; then
		echo "Generating Xcode project..."
		xcodegen generate
	else
		echo "Xcode project is up to date"
	fi

# Build Showcase iOS app for simulator.
[group('ios')]
showcase-build-sim: showcase-build-rust-sim showcase-xcodegen
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-ui-kit/ios
	xcodebuild -project GPUIShowcase.xcodeproj \
		-scheme GPUIShowcase \
		-configuration Release \
		-sdk iphonesimulator \
		-arch arm64 \
		build

# Build Showcase iOS app for device.
[group('ios')]
showcase-build-device: showcase-build-rust-device showcase-xcodegen
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-ui-kit/ios
	xcodebuild -project GPUIShowcase.xcodeproj \
		-scheme GPUIShowcase \
		-configuration Release \
		-sdk iphoneos \
		-arch arm64 \
		build

# Build the simulator hot-reload dylib and manifest.
[group('ios')]
showcase-hot-reload:
	crates/gpui-ui-kit/ios/hot-reload-showcase.sh

# Build the Showcase iOS app for simulator.
[group('ios')]
ios-sim: showcase-build-sim
	@echo "iOS simulator build complete"

# Build the Showcase iOS app for device.
[group('ios')]
ios-device: showcase-build-device
	@echo "iOS device build complete"

# ----------------------------------------------------------------------
# TVOS
# ----------------------------------------------------------------------
#
# tvOS is a Tier 3 Rust target and requires nightly with rust-src:
#   rustup toolchain install nightly
#   rustup component add rust-src --toolchain nightly

alias tvos-rust-sim := showcase-tvos-rust-sim
alias tvos-rust-device := showcase-tvos-rust-device
alias tvos-build-rust-sim := showcase-tvos-build-rust-sim
alias tvos-build-rust-device := showcase-tvos-build-rust-device

# Build Showcase tvOS Rust static library for simulator.
[group('tvos')]
showcase-tvos-rust-sim:
	cargo +nightly build -p gpui-ui-kit-ios-showcase --target aarch64-apple-tvos-sim --release {{features}} -Zbuild-std

# Build Showcase tvOS Rust static library for device.
[group('tvos')]
showcase-tvos-rust-device:
	cargo +nightly build -p gpui-ui-kit-ios-showcase --target aarch64-apple-tvos --release {{features}} -Zbuild-std

# Build Showcase tvOS Rust lib and copy it next to the mobile Xcode assets.
[group('tvos')]
showcase-tvos-build-rust-sim: showcase-tvos-rust-sim
	#!/usr/bin/env bash
	set -euo pipefail
	TVOS_DIR="crates/gpui-ui-kit/ios"
	mkdir -p "$TVOS_DIR/lib"
	cp target/aarch64-apple-tvos-sim/release/libshowcase_ios.a "$TVOS_DIR/lib/libshowcase_ios_tvos_sim.a"
	echo "Copied libshowcase_ios_tvos_sim.a to $TVOS_DIR/lib/"

# Build Showcase tvOS Rust lib for device and copy it next to the mobile Xcode assets.
[group('tvos')]
showcase-tvos-build-rust-device: showcase-tvos-rust-device
	#!/usr/bin/env bash
	set -euo pipefail
	TVOS_DIR="crates/gpui-ui-kit/ios"
	mkdir -p "$TVOS_DIR/lib"
	cp target/aarch64-apple-tvos/release/libshowcase_ios.a "$TVOS_DIR/lib/libshowcase_ios_tvos.a"
	echo "Copied libshowcase_ios_tvos.a to $TVOS_DIR/lib/"

# Build the Showcase tvOS Rust library for simulator.
[group('tvos')]
tvos-sim: showcase-tvos-build-rust-sim
	@echo "tvOS simulator Rust library build complete"

# Build the Showcase tvOS Rust library for device.
[group('tvos')]
tvos-device: showcase-tvos-build-rust-device
	@echo "tvOS device Rust library build complete"

# ----------------------------------------------------------------------
# MAINTENANCE
# ----------------------------------------------------------------------

[group('maintenance')]
clean:
	cargo clean
	find . -name '*~' -exec rm {} \; -print

[group('maintenance')]
update: update-rust update-pre-commit

[group('maintenance')]
update-rust:
	rustup update
	cargo update

[group('maintenance')]
update-pre-commit:
	pre-commit autoupdate

download-once:
	wget -q -O crates/gpui-d3rs/bin/showcase/data/land-50m.json https://cdn.jsdelivr.net/npm/world-atlas@2/land-50m.json
