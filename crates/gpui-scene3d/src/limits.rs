//! Checked render dimensions and frame working-memory budgets.

// Rust guideline compliant 2026-02-21

use crate::SceneError;

/// Limits applied before allocating render targets or CPU frame buffers.
///
/// The frame budget covers simultaneously live color/depth/staging storage
/// requested by the operation and software transparent-triangle storage,
/// including old/new triangle allocations during growth. It excludes retained
/// scene geometry and scene-order metadata. Hosts should
/// divide their total memory allowance among these separate working sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderLimits {
    /// Largest permitted width or height; GPU paths also apply device limits.
    pub max_dimension: u32,
    /// Maximum frame working-memory bytes.
    pub max_frame_bytes: u64,
}

impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            // Fits a 4K RGBA8/depth32 frame while rejecting accidental giant
            // targets. Constrained hosts can lower either limit explicitly.
            max_dimension: 16_384,
            max_frame_bytes: 64 * 1024 * 1024,
        }
    }
}

impl RenderLimits {
    /// Checks dimensions and total live storage without allocating.
    ///
    /// # Errors
    ///
    /// Returns an error for zero dimensions, overflow, or exceeded limits.
    pub fn pixel_count(
        self,
        width: u32,
        height: u32,
        bytes_per_pixel: u64,
    ) -> Result<usize, SceneError> {
        if width == 0 || height == 0 || bytes_per_pixel == 0 {
            return Err(SceneError::InvalidSize(width, height));
        }
        if width > self.max_dimension || height > self.max_dimension {
            return Err(SceneError::ResourceLimit {
                resource: "render dimension",
                requested: u64::from(width.max(height)),
                limit: u64::from(self.max_dimension),
            });
        }
        let pixels = u64::from(width) * u64::from(height);
        let bytes = pixels
            .checked_mul(bytes_per_pixel)
            .ok_or(SceneError::ResourceLimit {
                resource: "frame bytes",
                requested: u64::MAX,
                limit: self.max_frame_bytes,
            })?;
        let limit = self.max_frame_bytes.min(usize::MAX as u64);
        if bytes > limit {
            return Err(SceneError::ResourceLimit {
                resource: "frame bytes",
                requested: bytes,
                limit,
            });
        }
        Ok(pixels as usize)
    }
}

pub(crate) fn frame_buffer<T: Clone>(length: usize, value: T) -> Result<Vec<T>, SceneError> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(length)
        .map_err(|_| SceneError::Allocation {
            bytes: (length as u64).saturating_mul(std::mem::size_of::<T>() as u64),
        })?;
    buffer.resize(length, value);
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions_and_working_set_are_checked_before_allocation() {
        let limits = RenderLimits {
            max_dimension: 100_000,
            max_frame_bytes: 1024,
        };
        assert_eq!(limits.pixel_count(8, 16, 8).unwrap(), 128);
        assert!(limits.pixel_count(8, 17, 8).is_err());
        assert!(limits.pixel_count(65_536, 65_536, 8).is_err());
        assert!(limits.pixel_count(0, 1, 4).is_err());
        assert!(limits.pixel_count(1, 100_001, 4).is_err());
        let unlimited = RenderLimits {
            max_dimension: u32::MAX,
            max_frame_bytes: u64::MAX,
        };
        assert!(unlimited.pixel_count(u32::MAX, u32::MAX, 8).is_err());
    }
}
