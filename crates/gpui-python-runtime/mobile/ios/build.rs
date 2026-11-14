//! Links the vendored CPython XCframework slice matching the target.
//!
//! PyO3's own build configuration (headers, `sysconfig`) still comes from
//! the `PYO3_CROSS_LIB_DIR` environment variable, which `build-rust.sh`
//! sets per target. This script only adds the framework search path and
//! the `Python` framework link for the final staticlib consumer.

use std::path::PathBuf;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let slice = match target.as_str() {
        "aarch64-apple-ios" => "ios-arm64",
        "aarch64-apple-ios-sim" => "ios-arm64_x86_64-simulator",
        "aarch64-apple-tvos" => "tvos-arm64",
        "aarch64-apple-tvos-sim" => "tvos-arm64_x86_64-simulator",
        _ => return,
    };
    let xcframework = std::env::var_os("GPUI_PYTHON_XCFRAMEWORK")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/Python.xcframework")
        });
    println!("cargo:rerun-if-env-changed=GPUI_PYTHON_XCFRAMEWORK");
    println!(
        "cargo:rustc-link-search=framework={}",
        xcframework.join(slice).display()
    );
    println!("cargo:rustc-link-lib=framework=Python");
}
