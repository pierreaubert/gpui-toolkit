//! Counting global allocator used when `global-allocator` is enabled.
//!
//! The unsafe `GlobalAlloc` implementation lives in the reviewed
//! `stats_alloc` dependency rather than in first-party portable code.

use stats_alloc::{INSTRUMENTED_SYSTEM, StatsAlloc};
use std::alloc::System;

#[global_allocator]
pub(crate) static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;
