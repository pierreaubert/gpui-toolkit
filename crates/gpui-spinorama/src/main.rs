//! Full D3rs Spinorama viewer with live speaker measurements, polar and 3D views.
//!
//! Build/serve the wasm version via `just wasm-serve-spinorama` (Trunk, port 8084).
#![cfg_attr(target_family = "wasm", no_main)]

include!("entry.rs");
