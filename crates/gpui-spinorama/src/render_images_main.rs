//! Command-line entry point for offline Spinorama image rendering.
#![cfg_attr(target_family = "wasm", no_main)]

// Share mode dispatch and module declarations with the viewer binary.
include!("entry.rs");
