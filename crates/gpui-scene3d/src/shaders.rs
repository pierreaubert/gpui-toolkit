//! WGSL shader sources for GPU hosts.
//!
//! These strings are the shader contract shared by every GPU backend
//! (Metal on Apple Silicon, Vulkan/Direct3D elsewhere, WebGPU in the
//! browser). Keeping them as data in this crate means the software
//! rasterizer, the volume raycaster, and GPU hosts all implement the
//! same clipping and colouring semantics; the `gpu` feature validates
//! them with the real `wgpu` compiler.

// Rust guideline compliant 2026-02-21

/// Maximum clipping planes in the shader uniform block.
pub const MAX_CLIP_PLANES: usize = 6;

/// Surface shader: per-vertex scalar colouring through a LUT texture,
/// flat lighting, and up to [`MAX_CLIP_PLANES`] clipping planes.
///
/// Uniform layout (`SceneUniforms`):
/// - `view_proj`: `mat4x4<f32>`, column-major, matching [`crate::Camera`].
/// - `scalar_range`: `[min, max, opacity, wireframe_mix]`.
/// - `clip_origins[i]` / `clip_normals[i]`: world-space plane data;
///   fragments with a negative distance to any active plane are discarded.
/// - `clip_count`: number of active planes.
pub const SURFACE_WGSL: &str = r"
struct SceneUniforms {
    view_proj: mat4x4<f32>,
    scalar_range: vec4<f32>,
    clip_origins: array<vec4<f32>, 6>,
    clip_normals: array<vec4<f32>, 6>,
    clip_count: u32,
};

@group(0) @binding(0) var<uniform> scene: SceneUniforms;
@group(0) @binding(1) var lut_texture: texture_1d<f32>;
@group(0) @binding(2) var lut_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) scalar: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) scalar: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = scene.view_proj * vec4<f32>(input.position, 1.0);
    out.world_position = input.position;
    out.normal = input.normal;
    out.scalar = input.scalar;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Invalid point/cell samples are encoded as NaN during preparation.
    if (input.scalar != input.scalar) {
        discard;
    }
    for (var i: u32 = 0u; i < scene.clip_count; i = i + 1u) {
        let origin = scene.clip_origins[i].xyz;
        let normal = scene.clip_normals[i].xyz;
        if (dot(input.world_position - origin, normal) < 0.0) {
            discard;
        }
    }
    let range = scene.scalar_range.y - scene.scalar_range.x;
    var t = 0.0;
    if (range > 0.0) {
        t = clamp((input.scalar - scene.scalar_range.x) / range, 0.0, 1.0);
    }
    var colour = textureSample(lut_texture, lut_sampler, t);
    let light_dir = normalize(vec3<f32>(-0.4, 0.7, 0.6));
    let diffuse = abs(dot(normalize(input.normal), light_dir));
    colour = vec4<f32>(colour.rgb * (0.35 + 0.65 * diffuse), colour.a * scene.scalar_range.z);
    return colour;
}
";

/// The gradient compute kernel lives with the flow diagnostics in
/// `gpui_dataset_transformation::gpu::GRADIENT_WGSL`, next to the CPU
/// reference it must match.
/// Builds the surface-shader uniform block bytes for a camera and actor.
///
/// Layout matches `SceneUniforms` in [`SURFACE_WGSL`]: view-projection
/// (64 bytes), scalar range (16), origins (96), normals (96), count (4),
/// padded to 16-byte alignment.
#[must_use]
pub fn scene_uniform_bytes(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: &[crate::scene::ClipPlane],
) -> Vec<u8> {
    scene_uniform_bytes_relative_to(
        view_projection,
        scalar_range,
        opacity,
        clip_planes,
        [0.0; 3],
    )
}

/// Builds uniforms for vertices rebased on `origin`.
///
/// Plane origins are converted in `f64` before upload, matching vertex
/// preparation and preserving small half-space offsets at large coordinates.
#[must_use]
pub fn scene_uniform_bytes_relative_to(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: &[crate::scene::ClipPlane],
    origin: [f64; 3],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(288);
    for value in view_projection {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for value in [scalar_range[0], scalar_range[1], opacity, 0.0] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    let mut origins = [[0.0f32; 4]; MAX_CLIP_PLANES];
    let mut normals = [[0.0f32; 4]; MAX_CLIP_PLANES];
    for (i, plane) in clip_planes.iter().take(MAX_CLIP_PLANES).enumerate() {
        let mut local = *plane;
        local.origin = std::array::from_fn(|axis| plane.origin[axis] - origin[axis]);
        let uniform = local.to_uniform();
        origins[i] = [uniform[0], uniform[1], uniform[2], 0.0];
        normals[i] = [uniform[4], uniform[5], uniform[6], 0.0];
    }
    for plane in origins.iter().chain(normals.iter()) {
        for value in plane {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes.extend_from_slice(&(clip_planes.len().min(MAX_CLIP_PLANES) as u32).to_le_bytes());
    bytes.extend_from_slice(&[0u8; 12]);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_block_is_aligned_and_counts_planes() {
        let bytes = scene_uniform_bytes(
            &[0.0; 16],
            [0.0, 10.0],
            0.5,
            &[crate::scene::ClipPlane {
                origin: [1.0, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            }],
        );
        assert_eq!(bytes.len() % 16, 0);
        assert_eq!(bytes.len(), 288);
    }

    #[test]
    fn shaders_declare_the_clipping_contract() {
        assert!(SURFACE_WGSL.contains("clip_count"));
        assert!(SURFACE_WGSL.contains("discard"));
    }

    #[test]
    fn planes_are_rebased_before_float_conversion() {
        let origin = [1e12, -1e12, 1e12];
        let plane = crate::ClipPlane {
            origin: [origin[0] + 0.125, origin[1] + 0.25, origin[2] - 0.5],
            normal: [1.0, 2.0, -3.0],
        };
        let bytes = scene_uniform_bytes_relative_to(&[0.0; 16], [0.0, 1.0], 1.0, &[plane], origin);
        let read = |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        assert_eq!([read(80), read(84), read(88)], [0.125, 0.25, -0.5]);
        assert_eq!([read(176), read(180), read(184)], [1.0, 2.0, -3.0]);
    }
}
