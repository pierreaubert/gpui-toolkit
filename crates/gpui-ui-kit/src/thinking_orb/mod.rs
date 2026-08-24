//! Dotted 3D "thinking orb" status animations.
//!
//! The geometry engine and presets in this module are a faithful Rust port of
//! the TypeScript `thinking-orbs` library (version 0.3.1, MIT © Jakub
//! Antalik). The engine is pure math — no gpui imports — and its output is
//! verified against the upstream golden vectors in
//! `tests/components/thinking_orb_parity_test.rs`.

pub mod engine;
pub mod presets;
