# ----------------------------------------------------------------------
# CROSS PLATFORM BUILDS
#
# Linux and Windows targets are built in Docker images adapted from SotF.
# The recipes compile the portable toolkit crates, demos, examples, and tests
# with the shared feature set from the root Justfile.
# ----------------------------------------------------------------------

linux_arm_target := "aarch64-unknown-linux-gnu"
linux_x86_target := "x86_64-unknown-linux-gnu"
windows_arm_target := "aarch64-pc-windows-gnullvm"
windows_x86_target := "x86_64-pc-windows-gnullvm"

linux_arm_image := "gpui-toolkit-base-linux-arm64"
linux_x86_image := "gpui-toolkit-base-linux-x86"
windows_arm_image := "gpui-toolkit-base-windows-arm64"
windows_x86_image := "gpui-toolkit-base-windows-x86"

# Docker Desktop on macOS has a tight default memory limit. The `dist` profile
# (fat LTO, codegen-units=1) OOM-kills the container on large crates, so
# cross-docker builds default to `release` (thin LTO, 14 codegen units). Set
# CROSS_DOCKER_PROFILE=dist to opt back into the shipping profile if you have
# enough memory/swap allocated to Docker.
cross_docker_profile := env_var_or_default("CROSS_DOCKER_PROFILE", "release")

alias docker-linux-arm64 := cross-linux-arm64
alias docker-linux-x86 := cross-linux-x86
alias docker-windows-arm64 := cross-windows-arm64
alias docker-windows-x86 := cross-windows-x86
alias cross-windows-86 := cross-windows-x86

[group('cross')]
cross: cross-linux cross-windows

[group('cross')]
cross-arm: cross-linux-arm64 cross-windows-arm64

[group('cross')]
cross-x86: cross-linux-x86 cross-windows-x86

[group('cross')]
cross-linux: cross-linux-arm64 cross-linux-x86

[group('cross')]
cross-windows: cross-windows-arm64 cross-windows-x86

[group('cross')]
cross-linux-arm64: docker-base-linux-arm64
	just _cross-docker-build {{linux_arm_image}} linux/arm64 {{linux_arm_target}}

[group('cross')]
cross-linux-x86: docker-base-linux-x86
	just _cross-docker-build {{linux_x86_image}} linux/amd64 {{linux_x86_target}}

[group('cross')]
cross-windows-arm64: docker-base-windows-arm64
	just _cross-docker-build {{windows_arm_image}} linux/arm64 {{windows_arm_target}}

[group('cross')]
cross-windows-x86: docker-base-windows-x86
	just _cross-docker-build {{windows_x86_image}} linux/amd64 {{windows_x86_target}}

[group('cross')]
cross-native-linux-x86:
	rustup target add {{linux_x86_target}}
	cargo build --profile dist --target {{linux_x86_target}} --all-targets {{features}} {{cross_packages}}

[group('cross')]
cross-native-linux-arm64:
	rustup target add {{linux_arm_target}}
	cargo build --profile dist --target {{linux_arm_target}} --all-targets {{features}} {{cross_packages}}

[group('cross')]
cross-native-windows-x86:
	rustup target add x86_64-pc-windows-msvc
	cargo build --profile dist --target x86_64-pc-windows-msvc --all-targets {{features}} {{cross_packages}}

[group('cross')]
cross-native-windows-arm64:
	rustup target add aarch64-pc-windows-msvc
	cargo build --profile dist --target aarch64-pc-windows-msvc --all-targets {{features}} {{cross_packages}}

[group('docker')]
docker-base-linux-arm64:
	docker build --platform linux/arm64 -t {{linux_arm_image}} -f builds/from_macos_arm/Dockerfile.base.aarch64-unknown-linux-gnu .

[group('docker')]
docker-base-linux-x86:
	docker build --platform linux/amd64 -t {{linux_x86_image}} -f builds/from_macos_arm/Dockerfile.base.x86_64-unknown-linux-gnu .

[group('docker')]
docker-base-windows-arm64:
	docker build --platform linux/arm64 -t {{windows_arm_image}} -f builds/from_macos_arm/Dockerfile.base.aarch64-pc-windows-gnullvm .

[group('docker')]
docker-base-windows-x86:
	docker build --platform linux/amd64 -t {{windows_x86_image}} -f builds/from_macos_arm/Dockerfile.base.x86_64-pc-windows-gnullvm .

[private]
_cross-docker-build image platform target:
	#!/usr/bin/env bash
	set -euo pipefail
	mkdir -p target/cross
	# The target is already baked into the base image, and we pin the toolchain
	# via RUSTUP_TOOLCHAIN=stable to avoid rustup syncing/updating the toolchain
	# on OverlayFS, which fails with "Invalid cross-device link (os error 18)"
	# (see rust-lang/rustup#1239).
	# We default to the `release` profile (thin LTO, 14 codegen units) because
	# the `dist` profile's fat LTO + codegen-units=1 is too memory-hungry for
	# Docker Desktop's default memory limit and gets OOM-killed on large crates.
	# Use CROSS_DOCKER_PROFILE=dist if you have allocated enough Docker memory.
	docker run --rm --platform {{platform}} \
		-v "$(pwd)":/project \
		-v "gpui-toolkit-target-{{target}}:/build-target" \
		-w /project \
		-e CARGO_TARGET_DIR=/build-target \
		-e RUSTUP_TOOLCHAIN=stable \
		-e CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2} \
		{{image}} \
		bash -lc 'cargo build --profile {{cross_docker_profile}} --target {{target}} --all-targets {{features}} {{cross_packages}}'
