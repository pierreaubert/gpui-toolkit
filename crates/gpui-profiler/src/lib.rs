//! Lightweight allocation profiling utilities for GPUI applications.
//!
//! Enable the `global-allocator` feature to count every heap allocation during
//! a profiling session. When the feature is disabled, the snapshot/probe API is
//! still available but reports zeros, so instrumented code compiles and runs
//! with no overhead.
//!
//! # Example
//!
//! ```ignore
//! use gpui_profiler::AllocProbe;
//!
//! let mut probe = AllocProbe::new();
//! // ... work you want to measure ...
//! probe.sample("after-update");
//! ```

#[cfg(feature = "global-allocator")]
mod global;

mod alloc_count;

pub use alloc_count::{AllocProbe, AllocSnapshot};
