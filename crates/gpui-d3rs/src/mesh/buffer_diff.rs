//! Diff-gated GPU buffer uploads.
//!
//! The wgpu renderers in this crate own persistent buffers sized for the
//! current geometry revision and refill them with `queue.write_buffer`
//! instead of allocating per frame (see `crate::vello2d::wgpu_draw`,
//! `crate::mesh::gpu::wgpu_backend`, and `crate::mesh::gpu::renderer3d`). Uniform and
//! overlay buffers are additionally rewritten on frames where nothing
//! changed — a camera at rest, a static composite rect — so every per-frame
//! `write_buffer` goes through the check here first.
//!
//! This module owns the CPU-side "should I write?" decision as pure logic so
//! it stays testable without a graphics adapter: the GPU call sites only
//! forward their serialized bytes. Skipping a write is never observable —
//! the buffer already holds byte-identical content — which the golden
//! equivalence test (`tests/buffer_diff_golden.rs`) pins against the
//! full-upload path.

/// Returns `true` when `next` must be uploaded because no previous upload
/// exists or the bytes differ.
///
/// A `None` previous revision always requires a write (first frame, or the
/// cache was invalidated after a buffer was recreated). NaN payloads compare
/// by bits, so a NaN-containing uniform deterministically reports "changed"
/// only when its bits actually change.
pub fn upload_differs(previous: Option<&[u8]>, next: &[u8]) -> bool {
    previous.is_none_or(|prev| prev != next)
}

/// Remembers the last bytes handed to `queue.write_buffer` for one GPU
/// buffer and reports whether a new payload needs a queue write.
///
/// Backed by a single small allocation reused across frames; uniform blocks
/// here are tens of bytes (48 B composite uniforms, under 512 B mesh
/// uniforms).
#[derive(Debug, Default, Clone)]
pub struct BufferUploadCache {
    last: Option<Vec<u8>>,
}

impl BufferUploadCache {
    /// Create an empty cache. The first [`Self::needs_write`] call always
    /// returns `true`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` when `next` must be written, and records it as the new
    /// baseline. Returns `false` when the buffer already holds `next`,
    /// letting the caller skip the `queue.write_buffer` call entirely.
    pub fn needs_write(&mut self, next: &[u8]) -> bool {
        if upload_differs(self.last.as_deref(), next) {
            let slot = self
                .last
                .get_or_insert_with(|| Vec::with_capacity(next.len()));
            slot.clear();
            slot.extend_from_slice(next);
            true
        } else {
            false
        }
    }

    /// Forget the baseline, forcing the next [`Self::needs_write`] to return
    /// `true`. Call after the underlying GPU buffer is recreated (geometry
    /// revision change, resize) so the fresh allocation is always filled.
    pub fn invalidate(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{BufferUploadCache, upload_differs};

    #[test]
    fn first_write_is_always_required() {
        assert!(upload_differs(None, &[1, 2, 3]));
        assert!(upload_differs(None, &[]));
    }

    #[test]
    fn identical_bytes_skip_the_write() {
        let prev = [9u8, 0, 7, 255];
        assert!(!upload_differs(Some(&prev), &[9, 0, 7, 255]));
    }

    #[test]
    fn single_changed_byte_requires_a_write() {
        let prev = [9u8, 0, 7, 255];
        assert!(upload_differs(Some(&prev), &[9, 0, 7, 254]));
    }

    #[test]
    fn length_change_requires_a_write() {
        assert!(upload_differs(Some(&[1, 2]), &[1, 2, 3]));
        assert!(upload_differs(Some(&[1, 2, 3]), &[1, 2]));
    }

    #[test]
    fn cache_skips_repeated_frames_and_fires_on_change() {
        let mut cache = BufferUploadCache::new();
        let frame_a = [1u8, 2, 3, 4];
        let frame_b = [1u8, 2, 3, 5];

        assert!(cache.needs_write(&frame_a));
        assert!(!cache.needs_write(&frame_a));
        assert!(!cache.needs_write(&frame_a));
        assert!(cache.needs_write(&frame_b));
        assert!(!cache.needs_write(&frame_b));
    }

    #[test]
    fn invalidate_forces_a_rewrite() {
        let mut cache = BufferUploadCache::new();
        let frame = [4u8, 3, 2, 1];
        assert!(cache.needs_write(&frame));
        assert!(!cache.needs_write(&frame));
        cache.invalidate();
        assert!(cache.needs_write(&frame));
    }

    #[test]
    fn nan_payloads_compare_by_bits() {
        let nan_a = f32::NAN.to_le_bytes();
        let nan_b = (-f32::NAN).to_le_bytes();
        // Same NaN bits: no write needed.
        assert!(!upload_differs(Some(&nan_a), &nan_a));
        // Different NaN bits: conservative rewrite.
        assert!(upload_differs(Some(&nan_a), &nan_b));
    }
}
