# ----------------------------------------------------------------------
# Linux host setup and native builds
# ----------------------------------------------------------------------

[group('install')]
install-linux-root:
	sudo apt update
	sudo DEBIAN_FRONTEND=noninteractive apt -y install perl curl git build-essential gcc g++ pkg-config cmake ninja-build ca-certificates libssl-dev libsqlite3-dev libglib2.0-dev libxkbcommon-x11-dev libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libclang-dev libv4l-dev patchelf

[group('install')]
install-linux: install-linux-root
	rustup target add x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

[group('build-linux')]
linux-x86:
	cargo build --profile dist --target x86_64-unknown-linux-gnu --all-targets {{features}} {{cross_packages}}

[group('build-linux')]
linux-arm64:
	cargo build --profile dist --target aarch64-unknown-linux-gnu --all-targets {{features}} {{cross_packages}}
