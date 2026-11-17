//! Links the vendored `CPython` Android build's `libpython`.
//!
//! `PyO3`'s own build configuration (headers, `sysconfig`) comes from the
//! `PYO3_CROSS_LIB_DIR` environment variable, which `build-rust.sh` sets
//! to the same vendor prefix used here for the link search path.

use std::path::PathBuf;

fn main() {
    if std::env::var("TARGET").map_or(true, |target| !target.contains("android")) {
        return;
    }
    let prefix = std::env::var_os("GPUI_PYTHON_ANDROID_PREFIX").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/prefix"),
        PathBuf::from,
    );
    println!("cargo:rerun-if-env-changed=GPUI_PYTHON_ANDROID_PREFIX");
    println!(
        "cargo:rustc-link-search=native={}",
        prefix.join("lib").display()
    );
    println!("cargo:rustc-link-lib=dylib=python3.15");
}
