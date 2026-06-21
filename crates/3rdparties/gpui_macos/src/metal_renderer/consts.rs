
#[cfg(not(feature = "runtime_shaders"))]
pub(super) const SHADERS_METALLIB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/shaders.metallib"));

#[cfg(feature = "runtime_shaders")]
pub(super) const SHADERS_SOURCE_FILE: &str = include_str!(concat!(env!("OUT_DIR"), "/stitched_shaders.metal"));

pub(super) const PATH_SAMPLE_COUNT: u32 = 4;

