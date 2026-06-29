//! Golden file tests for D3.js compatibility
//!
//! These tests compare d3rs output against golden files generated from D3.js.
//! To regenerate golden files, run: `cd golden && npm run generate`

#[path = "golden_tests/misc.rs"]
mod misc;
#[path = "golden_tests/ord_f64.rs"]
mod ord_f64;
#[path = "golden_tests/test.rs"]
mod test;
#[cfg(test)]
#[path = "golden_tests/tests.rs"]
mod tests;
#[path = "golden_tests/types.rs"]
mod types;
