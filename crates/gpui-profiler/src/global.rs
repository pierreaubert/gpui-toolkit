//! Counting global allocator used when the `global-allocator` feature is enabled.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Bytes allocated since the allocator was installed.
pub static ALLOC_BYTES: AtomicUsize = AtomicUsize::new(0);
/// Number of allocation calls since the allocator was installed.
pub static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        // SAFETY: `GlobalAlloc::alloc` is unsafe; we forward the layout
        // unchanged to the system allocator.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `GlobalAlloc::dealloc` is unsafe; we forward the same
        // pointer/layout to the system allocator.
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let delta = new_size.saturating_sub(layout.size());
        ALLOC_BYTES.fetch_add(delta, Ordering::Relaxed);
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        // SAFETY: `GlobalAlloc::realloc` is unsafe; we forward the same
        // pointer/layout and new size to the system allocator.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;
