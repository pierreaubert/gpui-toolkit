# mach2

**Vendored 3rd-party crate** -- fork of [mach2](https://github.com/JohnTitor/mach2).

Rust interface to the user-space API of the Mach 3.0 kernel (macOS/iOS/tvOS). Provides FFI bindings to Mach kernel primitives (ports, tasks, threads, VM).

## Important Notes

- This is a vendored upstream crate -- minimize modifications
- macOS/iOS/tvOS only
- `no_std` compatible
- Used transitively by audio and system-level crates
