# Unreleased

# 0.8.1

## New

- Included the canonical `gpui-android` Java activity source in the Gradle
  build and updated the manifest to launch `dev.gpui.mobile.GpuiActivity`.
- Added Java host compilation coverage to CI.
- Added an Android NativeActivity/Gradle host for the GPUI showcase, including
  a Rust `cdylib` entry point and Justfile recipes for target checks, native
  library builds, APK builds, installs, and launches.

## Fixed

- Verified the showcase on an Android 36 ARM64 AVD with `-gpu host`, where wgpu
  can select the emulator's Vulkan adapter instead of the GLES translator.
- Avoid a MoltenVK shader translation failure on the Android emulator by not
  passing fixed-size gradient color-stop arrays through the shared GPUI wgpu
  shader helper.
