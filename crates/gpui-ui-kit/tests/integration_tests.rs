//! Integration tests for GPUI UI Kit
//!
//! These tests verify that components can be rendered in actual GPUI windows.
//! They use the `#[gpui::test]` macro to set up a proper test environment.
//!
//! ## Structure
//!
//! - `integration/mod.rs` - Shared test utilities and setup
//! - `integration/button_test.rs` - Button component rendering tests
//! - More component test files can be added as needed
//!
//! ## Running
//!
//! ```bash
//! cargo test --test integration_tests
//! ```

#![recursion_limit = "8192"]

mod integration;
