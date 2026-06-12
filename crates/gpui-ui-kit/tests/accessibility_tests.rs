#![recursion_limit = "512"]
//! Accessibility tests for GPUI UI Kit
//!
//! ## Running
//!
//! ```bash
//! cargo test -p gpui-ui-kit --test accessibility_tests
//! ```

// Unit tests (no gpui::test macro, no SIGBUS risk)
#[path = "components/accessibility_test.rs"]
mod unit_tests;
