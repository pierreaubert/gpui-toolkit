//! Thread-local scratch buffers for hot path-string construction.

use std::cell::RefCell;

thread_local! {
    static PATH_STRING_SCRATCH: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Run `f` with a cleared thread-local path-string scratch buffer.
///
/// The buffer is reused across calls to avoid repeated small allocations
/// when building SVG path data on hot rendering paths.
pub(crate) fn with_path_scratch<R>(f: impl FnOnce(&mut String) -> R) -> R {
    PATH_STRING_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        scratch.clear();
        f(&mut scratch)
    })
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
