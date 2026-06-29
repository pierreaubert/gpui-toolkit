//! Core 2D chart renderer

use bytemuck::{Pod, Zeroable};

mod chart2_drenderer;
mod misc;

pub use chart2_drenderer::*;

/// Uniform buffer data
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    viewport_size: [f32; 2],
    _padding: [f32; 2],
}
