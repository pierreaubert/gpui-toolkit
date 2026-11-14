//! Thread-local scratch buffers for hot path-string construction.

use std::cell::RefCell;

thread_local! {
    static PATH_STRING_SCRATCH: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Capacity retained after a call; one huge path must not pin its
/// allocation in the thread-local for the rest of the process.
const MAX_RETAINED_SCRATCH_CAPACITY: usize = 64 * 1024;

/// Run `f` with a cleared thread-local path-string scratch buffer.
///
/// The buffer is reused across calls to avoid repeated small allocations
/// when building SVG path data on hot rendering paths. After `f`
/// returns the contents are dropped and capacity beyond
/// `MAX_RETAINED_SCRATCH_CAPACITY` is released.
pub(crate) fn with_path_scratch<R>(f: impl FnOnce(&mut String) -> R) -> R {
    PATH_STRING_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        scratch.clear();
        let result = f(&mut scratch);
        scratch.clear();
        if scratch.capacity() > MAX_RETAINED_SCRATCH_CAPACITY {
            scratch.shrink_to(MAX_RETAINED_SCRATCH_CAPACITY);
        }
        result
    })
}

/// Release the thread-local scratch allocation (memory-pressure hook).
#[cfg(test)]
pub(crate) fn trim_path_scratch() {
    PATH_STRING_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        scratch.clear();
        scratch.shrink_to_fit();
    });
}

/// Currently retained scratch capacity, for tests and diagnostics.
#[cfg(test)]
fn path_scratch_capacity() -> usize {
    PATH_STRING_SCRATCH.with(|scratch| scratch.borrow().capacity())
}

/// Build a `String` from a path by writing into the thread-local scratch
/// buffer and cloning the result. The scratch capacity is retained for the
/// next call.
pub(crate) fn path_to_string(path: &crate::shape::path::Path) -> String {
    with_path_scratch(|scratch| {
        path.write_svg_string(scratch);
        scratch.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn huge_paths_do_not_pin_scratch_capacity() {
        let rendered = with_path_scratch(|scratch| {
            scratch.push_str(&"M0,0".repeat(300_000));
            scratch.len()
        });
        assert_eq!(rendered, 1_200_000);
        assert!(
            path_scratch_capacity() <= MAX_RETAINED_SCRATCH_CAPACITY,
            "capacity {} retained after huge path",
            path_scratch_capacity()
        );
        let small = with_path_scratch(|scratch| {
            scratch.push_str("M0,0L1,1");
            scratch.clone()
        });
        assert_eq!(small, "M0,0L1,1");
        trim_path_scratch();
        assert_eq!(path_scratch_capacity(), 0);
    }
}
