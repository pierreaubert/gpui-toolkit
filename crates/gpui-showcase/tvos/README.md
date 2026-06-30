# gpui-showcase-tvos

tvOS showcase host for the GPUI component gallery.

This crate builds `libshowcase_tvos.a`, which the XcodeGen project in this
directory links into a tvOS app target.

```bash
just tvos-sim
just tvos-device
```

`just tvos-sim` builds the Rust static library for `aarch64-apple-tvos-sim`,
copies it to `crates/gpui-showcase/tvos/lib/libshowcase_tvos.a`, generates the
Xcode project with XcodeGen, and builds the `GPUIShowcaseTV` app for the tvOS
simulator.

`just tvos-device` does the same for `aarch64-apple-tvos` and compiles the tvOS
device app without code signing. To install on Apple TV hardware, open
`GPUIShowcaseTV.xcodeproj`, select the `GPUIShowcaseTV` target, set your
development team in Signing & Capabilities, then build/run from Xcode.

To use Xcode directly after generating the project:

```bash
just tvos-build-rust-sim
just tvos-xcodegen
open crates/gpui-showcase/tvos/GPUIShowcaseTV.xcodeproj
```

For device builds in Xcode, run `just tvos-build-rust-device` first so the
linked archive has the device architecture.
