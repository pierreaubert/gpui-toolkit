# --------------------------------------------------------- -*- just -*-
# How to install Just?
#	  cargo install just
# ----------------------------------------------------------------------

default:
	just --list

download-once:
	wget -q -O gpui-d3rs/bin/showcase/data/land-50m.json https://cdn.jsdelivr.net/npm/world-atlas@2/land-50m.json

# ----------------------------------------------------------------------
# TEST
# ----------------------------------------------------------------------

test:
	# Exclude GPUI crates from check - they cause stack overflow in syn during test/example mode compilation
	RUST_MIN_STACK=16777216 cargo check --workspace --all-targets

# Build gpui-ui-kit examples to verify they compile (doesn't run them)
test-examples:
	@echo "Building gpui-ui-kit examples..."
	RUST_MIN_STACK=16777216 cargo build --examples -p gpui-ui-kit
	@echo "✓ All gpui-ui-kit examples compiled successfully"

test-negative:
	cargo test -p sotf-gpui --test negative

test-proptest:
	PROPTEST_CASES=10000 cargo test -p sotf-gpui --test proptest_tests

# Note: --lib is intentionally omitted to respect `test = false` in crates like sotf-gpui
# which have deeply nested GPUI macros that cause stack overflow in syn
ntest:
    RUST_MIN_STACK=16777216 cargo nextest run --release --no-fail-fast --workspace

# ----------------------------------------------------------------------
# FORMAT
# ----------------------------------------------------------------------

alias format := fmt

fmt:
	cargo fmt --all

# ----------------------------------------------------------------------
# PROD
# ----------------------------------------------------------------------

alias build := prod

prod: prod-workspace

prod-workspace:
	cargo build --release --workspace

prod-sotf-gpui:
	cargo build --release --bin SotF -p sotf-gpui

# ----------------------------------------------------------------------
# AUDIO UNIT (macOS only)
# ----------------------------------------------------------------------

# Build Rust FFI library for Audio Units
build-au-rust:
	#!/usr/bin/env bash
	set -euxo pipefail
	# Build for both architectures
	cargo build --release -p sotf-audio-plugins-ffi --target x86_64-apple-darwin
	cargo build --release -p sotf-audio-plugins-ffi --target aarch64-apple-darwin
	cargo build --release -p gpui-au --target x86_64-apple-darwin
	cargo build --release -p gpui-au --target aarch64-apple-darwin
	# Create universal binaries
	mkdir -p sotf-audio-plugins/src-au/Resources
	lipo -create \
		target/x86_64-apple-darwin/release/libsotf_audio_plugins_ffi.a \
		target/aarch64-apple-darwin/release/libsotf_audio_plugins_ffi.a \
		-output sotf-audio-plugins/src-au/Resources/libsotf_audio_plugins_ffi.a
	lipo -create \
		target/x86_64-apple-darwin/release/libgpui_au.a \
		target/aarch64-apple-darwin/release/libgpui_au.a \
		-output sotf-audio-plugins/src-au/Resources/libgpui_au.a
	# Copy header files
	cp sotf-audio-plugins/src-ffi/sotf_audio_plugin_ffi.h sotf-audio-plugins/src-au/Shared/
	cp gpui-au/GPUIBridge.h sotf-audio-plugins/src-au/Shared/
	echo "✅ Universal Rust FFI libraries created"

# Build Audio Unit plugins in Xcode
build-au-swift: build-au-rust
	#!/usr/bin/env bash
	set -euxo pipefail
	cd sotf-audio-plugins/src-au
	# Generate Xcode project with XcodeGen
	if [ ! -d "SOTFAudioUnits.xcodeproj" ] || [ "project.yml" -nt "SOTFAudioUnits.xcodeproj/project.pbxproj" ]; then
		echo "🔨 Generating Xcode project with XcodeGen..."
		xcodegen generate
	fi
	# Build the Audio Unit
	xcodebuild -project SOTFAudioUnits.xcodeproj \
		-scheme EQAudioUnit \
		-configuration Release \
		build
	echo "✅ Audio Unit built successfully"

# Install Audio Units to system
install-au: build-au-rust build-au-swift
	#!/usr/bin/env bash
	set -euxo pipefail
	# Find the Xcode DerivedData build output - need the container .app
	XCODE_APP=$(find ~/Library/Developer/Xcode/DerivedData/SOTFAudioUnits-*/Build/Products/Release/SOTFAudioUnits.app -maxdepth 0 2>/dev/null | head -1)
	if [ -n "$XCODE_APP" ] && [ -d "$XCODE_APP" ]; then
		# Copy the entire app to Applications (AUv3 extensions require this)
		rm -rf ~/Applications/SOTFAudioUnits.app
		mkdir -p ~/Applications
		cp -r "$XCODE_APP" ~/Applications/
		echo "✅ SOTF Audio Units app installed to ~/Applications/"
		echo ""
		echo "IMPORTANT: You must launch ~/Applications/SOTFAudioUnits.app once to register the AU"
		echo "           Then it will be available in DAWs as 'SOTF: Parametric EQ'"
		echo ""
	else
		echo "⚠️  No Audio Unit build found in Xcode DerivedData"
		echo "    Run 'just build-au-swift' first"
		exit 1
	fi
	# Restart Audio Component registration
	killall -9 AudioComponentRegistrar coreaudiod 2>/dev/null || true

# Validate Audio Unit
validate-au:
	#!/usr/bin/env bash
	set -euxo pipefail
	echo "Validating SOTF EQ Audio Unit..."
	auval -v aufx SOEQ SOTF

# Complete AU build pipeline
build-au: build-au-rust build-au-swift
	echo "✅ Complete Audio Unit build finished"

# ----------------------------------------------------------------------
# CLEAN
# ----------------------------------------------------------------------

clean:
	cargo clean
	find . -name '*~' -exec rm {} \; -print
	find . -name 'Cargo.lock' -exec rm {} \; -print

# ----------------------------------------------------------------------
# DEV
# ----------------------------------------------------------------------

dev:
	cargo build --workspace

# ----------------------------------------------------------------------
# UPDATE
# ----------------------------------------------------------------------

update: update-rust update-pre-commit

update-rust:
	rustup update
	cargo update

update-pre-commit:
	pre-commit autoupdate

# ----------------------------------------------------------------------
# DEMO
# ----------------------------------------------------------------------

demo: demo-d3rs demo-px demo-ui-kit

demo-ui-kit:
	cargo build --release --example showcase -p gpui-ui-kit

demo-d3rs:
	cargo build --release --bin d3rs-showcase --features="gpui"
	cargo build --release --bin d3rs-spinorama --features="spinorama, gpu-3d"

demo-px:
	cargo build --release --bin px-showcase -p gpui-px
	cargo build --release --bin px-spinorama -p gpui-px --features="autoeq,tokio,reqwest,urlencoding"

# ----------------------------------------------------------------------
# Install rustup
# ----------------------------------------------------------------------

install-rustup:
	curl https://sh.rustup.rs -sSf > ./scripts/install-rustup
	chmod +x ./scripts/install-rustup
	./scripts/install-rustup -y
	~/.cargo/bin/rustup default stable
	~/.cargo/bin/cargo install just

# ----------------------------------------------------------------------
# Install macos
# ----------------------------------------------------------------------

install-macos-brew:
	curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh > ./scripts/install-brew
	chmod +x ./scripts/install-brew
	NONINTERACTIVE=1 ./scripts/install-brew

install-macos: install-macos-brew install-rustup
	# need xcode
	xcode-select --install
	# need metal
	xcodebuild -downloadComponent MetalToolchain
	# chromedriver sheanigans
	brew install chromedriver
	xattr -d com.apple.quarantine $(which chromedriver)
	# optimisation library
	brew install nlopt cmake netcdf opencv chafa


# ----------------------------------------------------------------------
# Install linux
# ----------------------------------------------------------------------

install-linux-root:
	sudo apt update && sudo apt -y install \
	   perl curl build-essential gcc g++ pkg-config cmake ninja-build gfortran \
	   libssl-dev \
	   ca-certificates \
	   patchelf libopenblas-dev gfortran \
	   chromium-browser chromium-chromedriver

install-linux: install-linux-root install-rustup

install-ubuntu-common:
		sudo apt install -y \
			 curl \
			 build-essential gcc g++ \
			 pkg-config \
			 libssl-dev \
			 ca-certificates \
			 cmake \
			 ninja-build \
			 perl \
			 libglib2.0-dev \
			 libxkbcommon-x11-dev \
			 libgtk-3-dev \
			 libwebkit2gtk-4.1-dev \
			 libayatana-appindicator3-dev \
			 librsvg2-dev \
			 patchelf \
			 libopenblas-dev \
			 gfortran \
			 libasound2-dev \
			 libnetcdf-dev \
			 libopencv-dev \
			 libclang-dev \
			 webkit2gtk-driver

install-ubuntu-x86-driver :
		sudo apt install -y \
			 chromium-browser \
			 chromium-chromedriver

install-ubuntu-arm64-driver :
		sudo apt install -y firefox
		# where is the geckodriver ?

install-ubuntu-x86: install-ubuntu-common install-ubuntu-x86-driver

install-ubuntu-arm64: install-ubuntu-common install-ubuntu-arm64-driver

# ----------------------------------------------------------------------
# POST
# ----------------------------------------------------------------------

post-install:
	$HOME/.cargo/bin/rustup default stable
	$HOME/.cargo/bin/cargo install just
	$HOME/.cargo/bin/cargo install cargo-wizard
	$HOME/.cargo/bin/cargo install cargo-vcpkg
	$HOME/.cargo/bin/cargo install cargo-llvm-cov
	$HOME/.cargo/bin/cargo install cross
	$HOME/.cargo/bin/cargo install cargo-binstall
	$HOME/.cargo/bin/cargo binstall cargo-nextest --secure
	$HOME/.cargo/bin/cargo check

# ----------------------------------------------------------------------
# SIGNING
# ----------------------------------------------------------------------



