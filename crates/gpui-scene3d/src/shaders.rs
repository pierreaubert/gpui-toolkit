//! WGSL shader sources for GPU hosts.
//!
//! These strings are the shader contract shared by every GPU backend
//! (Metal on Apple Silicon, Vulkan/Direct3D elsewhere, WebGPU in the
//! browser). The `gpu` feature validates the shader with the real `wgpu`
//! compiler. Local clipping equations are also used by the software paths;
//! perspective-correct interpolation is shared with software. GPU LUT sampling
//! remains quantized, while software evaluates the prepared transfer function.

// Rust guideline compliant 2026-02-21

/// Maximum clipping planes in the shader uniform block.
pub const MAX_CLIP_PLANES: usize = 6;

/// Uniform ABI version: viewport dimensions occupy the final aligned eight bytes.
pub const SCENE_UNIFORM_VERSION: u32 = 4;
/// Maximum forward f32 depth steps for black surface edge overlays.
///
/// Four representable steps cover surface/line interpolation rounding without a
/// fixed normalized-depth offset. Overlay fragments never write depth.
pub const WIREFRAME_DEPTH_ULPS: u32 = 4;
/// Surface uniform block size, including 16-byte alignment padding.
pub const SCENE_UNIFORM_SIZE: usize = 192;

/// Surface shader: per-vertex scalar colouring through a LUT texture,
/// interpolated two-sided lighting, and up to [`MAX_CLIP_PLANES`] clipping planes.
///
/// Uniform layout (`SceneUniforms`):
/// - `view_proj`: `mat4x4<f32>`, column-major, matching [`crate::Camera`].
/// - `scalar_range`: `[min, max, opacity, wireframe_depth_ulps]`.
/// - `clip_equations[i]`: local `[normal.xyz, offset]` plane coefficients;
///   fragments with a negative distance to any active plane are discarded.
/// - `clip_count`: number of active planes.
/// - `viewport`: physical target dimensions, aligned at byte 184. Required by
///   `vs_point`; use [`scene_uniform_block_for_target`] for point-cell hosts.
pub const SURFACE_WGSL: &str = r"
struct SceneUniforms {
    view_proj: mat4x4<f32>,
    scalar_range: vec4<f32>,
    clip_equations: array<vec4<f32>, 6>,
    clip_count: u32,
    viewport: vec2<f32>,
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
    @location(0) local_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) scalar: f32,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = scene.view_proj * vec4<f32>(input.position, 1.0);
    out.local_position = input.position;
    out.normal = input.normal;
    out.scalar = input.scalar;
    return out;
}

struct PrimitiveInput {
    @location(0) position: vec3<f32>,
    @location(2) scalar: f32,
};

fn primitive_vertex(input: PrimitiveInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = scene.view_proj * vec4<f32>(input.position, 1.0);
    out.local_position = input.position;
    out.normal = vec3<f32>(0.0);
    out.scalar = input.scalar;
    return out;
}

@vertex
fn vs_primitive(input: PrimitiveInput) -> VertexOutput {
    return primitive_vertex(input);
}

@vertex
fn vs_point(input: PrimitiveInput) -> VertexOutput {
    var out = primitive_vertex(input);
    let clip = out.clip_position;
    if (clip.w <= 0.0 || clip.z < 0.0 || clip.z > clip.w || any(scene.viewport <= vec2<f32>(0.0))) {
        out.clip_position = vec4<f32>(0.0, 0.0, -1.0, 1.0);
        return out;
    }
    let ndc = clip.xy / clip.w;
    let pixel = floor(vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5) * scene.viewport);
    if (any(pixel < vec2<f32>(0.0)) || any(pixel >= scene.viewport)) {
        out.clip_position = vec4<f32>(0.0, 0.0, -1.0, 1.0);
        return out;
    }
    let center = (pixel + vec2<f32>(0.5)) / scene.viewport;
    out.clip_position = vec4<f32>(vec2<f32>(center.x * 2.0 - 1.0, 1.0 - center.y * 2.0) * clip.w, clip.z, clip.w);
    return out;
}

fn material_color(input: VertexOutput) -> vec4<f32> {
    // Invalid point/cell samples are encoded as NaN during preparation.
    if (input.scalar != input.scalar) {
        discard;
    }
    for (var i: u32 = 0u; i < scene.clip_count; i = i + 1u) {
        let plane = scene.clip_equations[i];
        if (dot(input.local_position, plane.xyz) - plane.w < 0.0) {
            discard;
        }
    }
    let range = scene.scalar_range.y - scene.scalar_range.x;
    var t = 0.0;
    if (range > 0.0) {
        if (input.scalar >= scene.scalar_range.y) { t = 1.0; }
        else if (input.scalar > scene.scalar_range.x) {
            t = (input.scalar - scene.scalar_range.x) / range;
        }
    }
    return textureSample(lut_texture, lut_sampler, t);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var colour = material_color(input);
    let light_dir = normalize(vec3<f32>(-0.4, 0.7, 0.6));
    // Opposing smooth normals can cancel. A zero normal has ambient light,
    // matching the software fallback without relying on normalize(0) behavior.
    let squared = dot(input.normal, input.normal);
    var diffuse = 0.0;
    if (squared > 0.0) {
        diffuse = clamp(abs(dot(input.normal, light_dir)) / sqrt(squared), 0.0, 1.0);
    }
    colour = vec4<f32>(colour.rgb * (0.35 + 0.65 * diffuse), colour.a * scene.scalar_range.z);
    return colour;
}

@fragment
fn fs_unlit(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = material_color(input);
    return vec4<f32>(color.rgb, color.a * scene.scalar_range.z);
}

struct WireOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fs_wire(input: VertexOutput) -> WireOutput {
    let color = material_color(input);
    var output: WireOutput;
    output.color = vec4<f32>(0.0, 0.0, 0.0, color.a * scene.scalar_range.z);
    let depth_bits = bitcast<u32>(input.clip_position.z) & 0x7fffffffu;
    let steps = min(depth_bits, u32(scene.scalar_range.w));
    output.depth = bitcast<f32>(depth_bits - steps);
    return output;
}
";

/// The gradient compute kernel lives with the flow diagnostics in
/// `gpui_dataset_transformation::gpu::GRADIENT_WGSL`, next to the CPU
/// reference it must match.
/// Builds the surface-shader uniform block bytes for a camera and actor.
///
/// Layout matches `SceneUniforms` in [`SURFACE_WGSL`]: view-projection
/// (64 bytes), scalar range (16), local plane equations (96), count (4),
/// alignment padding (4), and physical viewport dimensions (8).
/// The viewport remains zero here; point-cell hosts must instead use
/// [`scene_uniform_block_for_target`].
///
/// # Errors
///
/// Returns an error for invalid plane coefficients or too many planes.
pub fn scene_uniform_bytes(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: &[crate::scene::ClipPlane],
) -> Result<Vec<u8>, crate::SceneError> {
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
///
/// # Errors
///
/// Returns an error for nonfinite planes, unrepresentable local offsets, or
/// more than [`MAX_CLIP_PLANES`] planes.
pub fn scene_uniform_bytes_relative_to(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: &[crate::scene::ClipPlane],
    origin: [f64; 3],
) -> Result<Vec<u8>, crate::SceneError> {
    Ok(scene_uniform_block_relative_to(
        view_projection,
        scalar_range,
        opacity,
        clip_planes,
        origin,
    )?
    .to_vec())
}

/// Builds the fixed-size uniform block without a heap allocation.
///
/// The viewport remains zero; use [`scene_uniform_block_for_target`] when
/// rendering point cells with `vs_point`.
///
/// # Errors
///
/// Returns an error for invalid plane coefficients or too many planes.
pub fn scene_uniform_block_relative_to(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: &[crate::scene::ClipPlane],
    origin: [f64; 3],
) -> Result<[u8; SCENE_UNIFORM_SIZE], crate::SceneError> {
    let equations = packed_clip_planes(clip_planes, origin)?;
    Ok(scene_uniform_block_with_equations(
        view_projection,
        scalar_range,
        opacity,
        &equations,
    ))
}

/// Builds uniforms with physical viewport dimensions for one-pixel point cells.
///
/// # Errors
///
/// Returns an error for zero target dimensions, invalid plane coefficients or
/// more than [`MAX_CLIP_PLANES`] planes.
pub fn scene_uniform_block_for_target(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    clip_planes: &[crate::ClipPlane],
    origin: [f64; 3],
    size: [u32; 2],
) -> Result<[u8; SCENE_UNIFORM_SIZE], crate::SceneError> {
    let mut bytes = scene_uniform_block_relative_to(
        view_projection,
        scalar_range,
        opacity,
        clip_planes,
        origin,
    )?;
    set_viewport(&mut bytes, size)?;
    Ok(bytes)
}

pub(crate) fn set_viewport(
    bytes: &mut [u8; SCENE_UNIFORM_SIZE],
    size: [u32; 2],
) -> Result<(), crate::SceneError> {
    if size.contains(&0) {
        return Err(crate::SceneError::InvalidSize(size[0], size[1]));
    }
    bytes[184..188].copy_from_slice(&(size[0] as f32).to_le_bytes());
    bytes[188..192].copy_from_slice(&(size[1] as f32).to_le_bytes());
    Ok(())
}

pub(crate) type PackedClipPlanes = smallvec::SmallVec<[[f32; 4]; MAX_CLIP_PLANES]>;

pub(crate) fn packed_clip_planes(
    planes: &[crate::ClipPlane],
    origin: [f64; 3],
) -> Result<PackedClipPlanes, crate::SceneError> {
    if planes.len() > MAX_CLIP_PLANES {
        return Err(crate::SceneError::ResourceLimit {
            resource: "GPU clip plane count",
            requested: planes.len() as u64,
            limit: MAX_CLIP_PLANES as u64,
        });
    }
    crate::scene::local_planes(planes, origin)?
        .into_iter()
        .map(crate::scene::PlaneEquation::to_uniform)
        .collect()
}

pub(crate) fn scene_uniform_block_with_equations(
    view_projection: &[f32; 16],
    scalar_range: [f32; 2],
    opacity: f32,
    equations: &[[f32; 4]],
) -> [u8; SCENE_UNIFORM_SIZE] {
    let mut bytes = [0u8; SCENE_UNIFORM_SIZE];
    let mut offset = 0;
    let mut append = |value: [u8; 4]| {
        bytes[offset..offset + 4].copy_from_slice(&value);
        offset += 4;
    };
    for value in view_projection {
        append(value.to_le_bytes());
    }
    for value in [
        scalar_range[0],
        scalar_range[1],
        opacity,
        WIREFRAME_DEPTH_ULPS as f32,
    ] {
        append(value.to_le_bytes());
    }
    for index in 0..MAX_CLIP_PLANES {
        for value in equations.get(index).copied().unwrap_or([0.0; 4]) {
            append(value.to_le_bytes());
        }
    }
    append((equations.len().min(MAX_CLIP_PLANES) as u32).to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_uses_reserved_tail_without_changing_uniform_stride() {
        let bytes =
            scene_uniform_block_for_target(&[0.0; 16], [0.0, 1.0], 1.0, &[], [0.0; 3], [96, 80])
                .unwrap();
        assert_eq!(bytes.len(), 192);
        assert_eq!(
            f32::from_le_bytes(bytes[184..188].try_into().unwrap()),
            96.0
        );
        assert_eq!(
            f32::from_le_bytes(bytes[188..192].try_into().unwrap()),
            80.0
        );
        assert!(matches!(
            scene_uniform_block_for_target(&[0.0; 16], [0.0, 1.0], 1.0, &[], [0.0; 3], [0, 80]),
            Err(crate::SceneError::InvalidSize(0, 80))
        ));
    }

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
        )
        .unwrap();
        assert_eq!(bytes.len() % 16, 0);
        assert_eq!(bytes.len(), SCENE_UNIFORM_SIZE);
        assert_eq!(
            f32::from_le_bytes(bytes[76..80].try_into().unwrap()),
            WIREFRAME_DEPTH_ULPS as f32
        );
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
        let bytes =
            scene_uniform_bytes_relative_to(&[0.0; 16], [0.0, 1.0], 1.0, &[plane], origin).unwrap();
        let read = |offset| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        assert_eq!([read(80), read(84), read(88)], [1.0 / 3.0, 2.0 / 3.0, -1.0]);
        assert!((read(92) - 2.125 / 3.0).abs() < 1e-7);
        assert_eq!(u32::from_le_bytes(bytes[176..180].try_into().unwrap()), 1);
    }

    #[test]
    fn uniform_builders_reject_excess_and_nonfinite_planes() {
        let plane = crate::ClipPlane {
            origin: [0.0; 3],
            normal: [1.0, 0.0, 0.0],
        };
        assert!(matches!(
            scene_uniform_bytes(&[0.0; 16], [0.0, 1.0], 1.0, &[plane; 7]),
            Err(crate::SceneError::ResourceLimit { .. })
        ));
        assert!(matches!(
            scene_uniform_bytes(
                &[0.0; 16],
                [0.0, 1.0],
                1.0,
                &[crate::ClipPlane {
                    normal: [f64::INFINITY; 3],
                    ..plane
                }]
            ),
            Err(crate::SceneError::InvalidClipPlane(_))
        ));
        assert_eq!(SCENE_UNIFORM_VERSION, 4);
        assert_eq!(SCENE_UNIFORM_SIZE, 192);
    }
}
