# ----------------------------------------------------------------------
# Windows host setup and native MSVC builds
# ----------------------------------------------------------------------

[group('build-windows')]
windows-x86:
	rustup target add x86_64-pc-windows-msvc
	cargo build --profile dist --target x86_64-pc-windows-msvc --all-targets {{features}} {{cross_packages}}

[group('build-windows')]
windows-arm64:
	rustup target add aarch64-pc-windows-msvc
	cargo build --profile dist --target aarch64-pc-windows-msvc --all-targets {{features}} {{cross_packages}}

[group('build-windows')]
windows:
	just windows-x86
	just windows-arm64
