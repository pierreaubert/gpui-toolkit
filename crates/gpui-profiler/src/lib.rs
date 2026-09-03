#![forbid(unsafe_code)]

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
//!
//! Collect [`LabeledAllocSample`]s with [`AllocProbe::sample_labeled`] (or
//! share a [`ThreadAllocProbe`] across threads) and export the named series
//! with [`samples_to_csv`] or [`samples_to_chrome_trace`] for Perfetto.

#[cfg(feature = "global-allocator")]
mod global;

mod alloc_count;

pub use alloc_count::{
    AllocProbe, AllocSnapshot, AllocationBudget, LabeledAllocSample, ThreadAllocProbe,
    samples_to_chrome_trace, samples_to_csv,
};
