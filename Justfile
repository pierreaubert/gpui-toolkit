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

features := "--features autoeq,camera,gpu-2d,gpu-3d,reqwest,spinorama,tokio,urlencoding"
cross_packages := "-p gpui-audio-kit -p gpui-builder -p gpui-component-lab -p gpui-d3rs -p gpui-design -p gpui-design-tools -p gpui-keybinding -p gpui-miniapp -p gpui-pretext -p gpui-px -p gpui-python-runtime -p gpui-scaffolder -p gpui-themes -p gpui-ui-kit -p gpui-ui-kit-macros"
public_core_packages := "-p gpui-audio-kit -p gpui-builder -p gpui-d3rs -p gpui-design -p gpui-keybinding -p gpui-pretext -p gpui-px -p gpui-themes -p gpui-ui-kit -p gpui-ui-kit-macros"

android_sdk_root := env_var_or_default("ANDROID_HOME", env_var_or_default("ANDROID_SDK_ROOT", "/opt/homebrew/share/android-commandlinetools"))
android_ndk_version := env_var_or_default("ANDROID_NDK_VERSION", "27.2.12479018")
android_java_home := env_var_or_default("JAVA_HOME", "/Applications/Android Studio.app/Contents/jbr/Contents/Home")

# QA / coverage settings
# The gate is a ratchet, not an aspirational value that leaves `just qa` red.
# Raise this floor whenever the measured portable-core coverage improves; the
# release target remains 90% (documented in qa.md).
cov_threshold := "73.5"
cov_ignore_regex := '.*/(tests|benches|examples|target|crates/3rdparties|crates/gpui-au|crates/gpui-android|crates/gpui-ios|crates/gpui-miniapp|crates/.*/bin)/.*'
cov_summary := "target/qa/cov/summary.json"
cov_report := "target/qa/cov/report.md"
perf_baseline := "qa/perf/baseline.json"
perf_current := "target/qa/perf/current.json"
perf_report := "target/qa/perf/report.md"
# Cross-process Criterion runs on shared developer/CI hosts show up to ~18%
# paired variance even after warm-up. Keep this a hard gate, but reserve failure
# for a material slowdown above the observed envelope.
perf_threshold := "20"
perf_noise_floor_ns := "150"

# ----------------------------------------------------------------------
# TEST / QA
# ----------------------------------------------------------------------

[group('check')]
check:
	cargo check --workspace --all-targets {{features}}

[group('lint')]
lint: lint-host

[group('lint')]
lint-host:
	cargo clippy --workspace --all-targets {{features}} -- -D warnings

[group('lint')]
lint-all: lint-host lint-ios-rust

[group('lint')]
lint-ios-rust:
	RUSTFLAGS="-D warnings" cargo build -p gpui-showcase-ios --target aarch64-apple-ios-sim --release {{features}}

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
	cargo test -p gpui-ui-kit {{features}} --features bench --test allocation_contracts -- --test-threads=1
	cargo test -p gpui-ui-kit {{features}} --features bench --test text_input_corpus
	cargo test -p gpui-d3rs {{features}}
	cargo test -p gpui-px {{features}}
	cargo tree -p gpui-design-tools {{features}}

# ----------------------------------------------------------------------
# QA SUITE
# ----------------------------------------------------------------------

# Full QA aggregator. Runs non-coverage checks first; coverage gate is last so a
# sub-90% report still lets the other suites exercise the code.
[group('qa')]
qa: lint-host qa-scripts qa-api qa-prop qa-visual qa-perf qa-gpui-obvious qa-cov-check qa-deps
	@echo "Full QA suite passed"

[group('qa')]
qa-scripts:
	PYTHONPATH=scripts python3 -m unittest discover -s scripts/tests -p 'test_*.py'

[group('qa')]
qa-api:
	python3 scripts/qa_docs_policy.py
	cargo check {{public_core_packages}} --lib --no-default-features
	RUSTDOCFLAGS="-D warnings" cargo doc {{public_core_packages}} --lib --no-deps --no-default-features
	cargo test -p gpui-scaffolder scaffolded_project_passes_cargo_check

# Dependency, advisory, license, and source-origin policy. cargo-deny is the
# canonical release check; keeping it in `qa` prevents the policy from becoming
# report-only metadata.
[group('qa')]
qa-deps:
	cargo deny check

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
	cargo llvm-cov --workspace --exclude gpui-scaffolder --all-targets --html --output-dir target/qa/cov/html {{features}} --ignore-filename-regex '{{cov_ignore_regex}}'
	cargo llvm-cov --workspace --exclude gpui-scaffolder --all-targets --json --summary-only --output-path {{cov_summary}} {{features}} --ignore-filename-regex '{{cov_ignore_regex}}'
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
	cargo llvm-cov --workspace --exclude gpui-scaffolder --all-targets --json --summary-only --output-path {{cov_summary}} {{features}} --ignore-filename-regex '{{cov_ignore_regex}}'
	python3 scripts/qa_cov_check.py --summary {{cov_summary}} --threshold {{cov_threshold}} --output {{cov_report}} --ignore-regex '{{cov_ignore_regex}}'

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
	python3 scripts/qa_perf_check.py --baseline {{perf_baseline}} --current {{perf_current}} --threshold {{perf_threshold}} --noise-floor-ns {{perf_noise_floor_ns}} --output {{perf_report}}

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

alias demos := demo

[group('demo')]
demo: demo-audio-kit demo-builder demo-component-lab demo-d3rs demo-px demo-python demo-showcase demo-themes demo-ui-kit

[group('demo')]
demo-audio-kit:
	cargo build --release --examples -p gpui-audio-kit {{features}}

[group('demo')]
demo-ui-kit:
	cargo build --release --examples -p gpui-ui-kit {{features}}

[group('demo')]
demo-builder:
	cargo build --release --bin layout-showcase -p gpui-builder {{features}} --features showcase

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
	cargo build --release --bin gpui-python-showcase -p gpui-python-runtime {{features}} --features showcase

[group('demo')]
demo-showcase:
	cargo build --release --bin gpui-showcase -p gpui-showcase

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
	cargo build -p gpui-showcase-ios --target aarch64-apple-ios-sim --release {{features}}

# Build Showcase iOS Rust static library for device.
[group('ios')]
showcase-rust-device:
	cargo build -p gpui-showcase-ios --target aarch64-apple-ios --release {{features}}

# Build Showcase iOS Rust lib and copy it to the Xcode project.
[group('ios')]
showcase-build-rust-sim: showcase-rust-sim
	#!/usr/bin/env bash
	set -euo pipefail
	IOS_DIR="crates/gpui-showcase/ios"
	mkdir -p "$IOS_DIR/lib"
	cp target/aarch64-apple-ios-sim/release/libshowcase_ios.a "$IOS_DIR/lib/"
	echo "Copied libshowcase_ios.a to $IOS_DIR/lib/"

# Build Showcase iOS Rust lib for device and copy it to the Xcode project.
[group('ios')]
showcase-build-rust-device: showcase-rust-device
	#!/usr/bin/env bash
	set -euo pipefail
	IOS_DIR="crates/gpui-showcase/ios"
	mkdir -p "$IOS_DIR/lib"
	cp target/aarch64-apple-ios/release/libshowcase_ios.a "$IOS_DIR/lib/"
	echo "Copied libshowcase_ios.a to $IOS_DIR/lib/"

# Generate the Showcase iOS Xcode project with XcodeGen.
[group('ios')]
showcase-xcodegen:
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-showcase/ios
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
	cd crates/gpui-showcase/ios
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
	cd crates/gpui-showcase/ios
	xcodebuild -project GPUIShowcase.xcodeproj \
		-scheme GPUIShowcase \
		-configuration Release \
		-sdk iphoneos \
		-arch arm64 \
		build

# Build the simulator hot-reload dylib and manifest.
[group('ios')]
showcase-hot-reload:
	crates/gpui-showcase/ios/hot-reload-showcase.sh

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
alias tvos-xcodegen := showcase-tvos-xcodegen
alias tvos-build-sim := showcase-tvos-build-sim
alias tvos-build-device := showcase-tvos-build-device

# Build Showcase tvOS Rust static library for simulator.
[group('tvos')]
showcase-tvos-rust-sim:
	TVOS_DEPLOYMENT_TARGET=15.0 cargo +nightly build -p gpui-showcase-tvos --target aarch64-apple-tvos-sim --release {{features}} -Zbuild-std

# Build Showcase tvOS Rust static library for device.
[group('tvos')]
showcase-tvos-rust-device:
	TVOS_DEPLOYMENT_TARGET=15.0 cargo +nightly build -p gpui-showcase-tvos --target aarch64-apple-tvos --release {{features}} -Zbuild-std

# Build Showcase tvOS Rust lib and copy it next to the mobile Xcode assets.
[group('tvos')]
showcase-tvos-build-rust-sim: showcase-tvos-rust-sim
	#!/usr/bin/env bash
	set -euo pipefail
	TVOS_DIR="crates/gpui-showcase/tvos"
	mkdir -p "$TVOS_DIR/lib"
	cp target/aarch64-apple-tvos-sim/release/libshowcase_tvos.a "$TVOS_DIR/lib/"
	echo "Copied libshowcase_tvos.a to $TVOS_DIR/lib/"

# Build Showcase tvOS Rust lib for device and copy it next to the mobile Xcode assets.
[group('tvos')]
showcase-tvos-build-rust-device: showcase-tvos-rust-device
	#!/usr/bin/env bash
	set -euo pipefail
	TVOS_DIR="crates/gpui-showcase/tvos"
	mkdir -p "$TVOS_DIR/lib"
	cp target/aarch64-apple-tvos/release/libshowcase_tvos.a "$TVOS_DIR/lib/"
	echo "Copied libshowcase_tvos.a to $TVOS_DIR/lib/"

# Generate the Showcase tvOS Xcode project with XcodeGen.
[group('tvos')]
showcase-tvos-xcodegen:
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-showcase/tvos
	if [ ! -d "GPUIShowcaseTV.xcodeproj" ] || [ "project.yml" -nt "GPUIShowcaseTV.xcodeproj/project.pbxproj" ]; then
		echo "Generating tvOS Xcode project..."
		xcodegen generate
	else
		echo "tvOS Xcode project is up to date"
	fi

# Build Showcase tvOS app for simulator.
[group('tvos')]
showcase-tvos-build-sim: showcase-tvos-build-rust-sim showcase-tvos-xcodegen
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-showcase/tvos
	xcodebuild -project GPUIShowcaseTV.xcodeproj \
		-scheme GPUIShowcaseTV \
		-configuration Release \
		-sdk appletvsimulator \
		-destination 'generic/platform=tvOS Simulator' \
		-derivedDataPath build/DerivedData-simulator \
		build

# Build Showcase tvOS app for device.
[group('tvos')]
showcase-tvos-build-device: showcase-tvos-build-rust-device showcase-tvos-xcodegen
	#!/usr/bin/env bash
	set -euo pipefail
	cd crates/gpui-showcase/tvos
	xcodebuild -project GPUIShowcaseTV.xcodeproj \
		-scheme GPUIShowcaseTV \
		-configuration Release \
		-sdk appletvos \
		-destination 'generic/platform=tvOS' \
		-derivedDataPath build/DerivedData-device \
		CODE_SIGNING_ALLOWED=NO \
		build

# Build the Showcase tvOS app for simulator.
[group('tvos')]
tvos-sim: showcase-tvos-build-sim
	@echo "tvOS simulator build complete"

# Build the Showcase tvOS app for device.
[group('tvos')]
tvos-device: showcase-tvos-build-device
	@echo "tvOS device build complete"

# ----------------------------------------------------------------------
# ANDROID
# ----------------------------------------------------------------------
#
# Android requires the Rust Android target plus cargo-ndk:
#   rustup target add aarch64-linux-android
#   cargo install cargo-ndk
#   sdkmanager --install "platform-tools" "platforms;android-35" "build-tools;35.0.0" "ndk;27.2.12479018"
# Build an APK with:
#   just android-apk

alias android-rust := showcase-android-rust
alias android-check := showcase-android-check
alias android-build-rust := showcase-android-build-rust
alias android-apk := showcase-android-apk
alias android-install := showcase-android-install
alias android-run := showcase-android-run

# Check the Showcase Android Rust crate for the arm64 Android target.
[group('android')]
showcase-android-check:
	CC_aarch64_linux_android="{{android_sdk_root}}/ndk/{{android_ndk_version}}/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android35-clang" \
		CXX_aarch64_linux_android="{{android_sdk_root}}/ndk/{{android_ndk_version}}/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android35-clang++" \
		AR_aarch64_linux_android="{{android_sdk_root}}/ndk/{{android_ndk_version}}/toolchains/llvm/prebuilt/darwin-x86_64/bin/llvm-ar" \
		cargo check -p gpui-showcase-android --target aarch64-linux-android {{features}}

# Build Showcase Android Rust shared library for arm64 devices/emulators.
[group('android')]
showcase-android-rust:
		ANDROID_HOME="{{android_sdk_root}}" \
		ANDROID_SDK_ROOT="{{android_sdk_root}}" \
		ANDROID_NDK_HOME="{{android_sdk_root}}/ndk/{{android_ndk_version}}" \
		cargo ndk -t arm64-v8a -P 26 -o crates/gpui-showcase/android/gradle/app/src/main/jniLibs build -p gpui-showcase-android --release {{features}}

# Build Showcase Android Rust shared library and copy it into Gradle jniLibs.
[group('android')]
showcase-android-build-rust: showcase-android-rust
	@echo "Copied libshowcase_android.so to crates/gpui-showcase/android/gradle/app/src/main/jniLibs/arm64-v8a/"

# Build the Showcase Android APK.
[group('android')]
showcase-android-apk: showcase-android-build-rust
	#!/usr/bin/env bash
	set -euo pipefail
	export ANDROID_HOME="{{android_sdk_root}}"
	export ANDROID_SDK_ROOT="{{android_sdk_root}}"
	export JAVA_HOME="{{android_java_home}}"
	cd crates/gpui-showcase/android/gradle
	./gradlew assembleDebug

# Install the Showcase Android APK on the connected device/emulator.
[group('android')]
showcase-android-install: showcase-android-apk
	"{{android_sdk_root}}/platform-tools/adb" install -r crates/gpui-showcase/android/gradle/app/build/outputs/apk/debug/app-debug.apk

# Launch the Showcase Android APK on the connected device/emulator.
[group('android')]
showcase-android-run: showcase-android-install
	"{{android_sdk_root}}/platform-tools/adb" shell am start -n org.spinorama.gpui.showcase/android.app.NativeActivity

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

[group('maintenance')]
xcode:
        xcodebuild -downloadComponent MetalToolchain

# ----------------------------------------------------------------------
# DOWNLOAD
# ----------------------------------------------------------------------

[group('download')]
download-once:
	wget -q -O crates/gpui-d3rs/bin/showcase/data/land-50m.json https://cdn.jsdelivr.net/npm/world-atlas@2/land-50m.json
