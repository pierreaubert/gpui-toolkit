//! Shared WGSL source for the retained 2D mesh pipeline.
//!
//! The polynomial maps and derivative-based isoline are intentionally kept
//! here rather than duplicated in the backend. The 3D shader has a matching
//! source so a scalar field has the same visual contract in both views.

pub const MESH_WGSL: &str = r"

struct Uniforms {
    view_transform: mat4x4<f32>,
    range: vec4<f32>,
    style: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> values: array<f32>;

struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) value: f32,
};

@vertex
fn vertex(@location(0) point: vec3<f32>, @builtin(vertex_index) index: u32) -> VertexOut {
    var output: VertexOut;
    output.position = uniforms.view_transform * vec4<f32>(point, 1.0);
    // Cell-associated uploads are expanded to triangle-local vertices before
    // they reach this pipeline. Both vertex- and cell-associated fields can
    // therefore use the same portable interpolated value path.
    output.value = select(0.5, values[index], uniforms.range.w > 0.5);
    return output;
}

fn viridis(t: f32) -> vec3<f32> {
    // Match the CPU ColorScale palette and its piecewise RGB interpolation.
    let colors = array<vec3<f32>, 10>(
        vec3<f32>(68.0, 1.0, 84.0) / 255.0,
        vec3<f32>(72.0, 40.0, 120.0) / 255.0,
        vec3<f32>(62.0, 74.0, 137.0) / 255.0,
        vec3<f32>(49.0, 104.0, 142.0) / 255.0,
        vec3<f32>(38.0, 131.0, 143.0) / 255.0,
        vec3<f32>(31.0, 158.0, 137.0) / 255.0,
        vec3<f32>(53.0, 183.0, 121.0) / 255.0,
        vec3<f32>(110.0, 206.0, 88.0) / 255.0,
        vec3<f32>(181.0, 222.0, 43.0) / 255.0,
        vec3<f32>(253.0, 231.0, 37.0) / 255.0
    );
    let position = clamp(t, 0.0, 1.0) * 9.0;
    let index = min(u32(position), 8u);
    return mix(colors[index], colors[index + 1u], position - f32(index));
}

fn plasma(t: f32) -> vec3<f32> {
    // Match the CPU ColorScale palette and its piecewise RGB interpolation.
    let colors = array<vec3<f32>, 10>(
        vec3<f32>(13.0, 8.0, 135.0) / 255.0,
        vec3<f32>(70.0, 3.0, 159.0) / 255.0,
        vec3<f32>(114.0, 1.0, 168.0) / 255.0,
        vec3<f32>(156.0, 23.0, 158.0) / 255.0,
        vec3<f32>(189.0, 55.0, 134.0) / 255.0,
        vec3<f32>(216.0, 87.0, 107.0) / 255.0,
        vec3<f32>(237.0, 121.0, 83.0) / 255.0,
        vec3<f32>(251.0, 159.0, 58.0) / 255.0,
        vec3<f32>(253.0, 202.0, 38.0) / 255.0,
        vec3<f32>(240.0, 249.0, 33.0) / 255.0
    );
    let position = clamp(t, 0.0, 1.0) * 9.0;
    let index = min(u32(position), 8u);
    return mix(colors[index], colors[index + 1u], position - f32(index));
}

fn inferno(t: f32) -> vec3<f32> {
    // Match the CPU ColorScale palette and its piecewise RGB interpolation.
    let colors = array<vec3<f32>, 10>(
        vec3<f32>(0.0, 0.0, 4.0) / 255.0,
        vec3<f32>(27.0, 12.0, 65.0) / 255.0,
        vec3<f32>(74.0, 12.0, 107.0) / 255.0,
        vec3<f32>(120.0, 28.0, 109.0) / 255.0,
        vec3<f32>(165.0, 44.0, 96.0) / 255.0,
        vec3<f32>(207.0, 68.0, 70.0) / 255.0,
        vec3<f32>(237.0, 105.0, 37.0) / 255.0,
        vec3<f32>(251.0, 155.0, 6.0) / 255.0,
        vec3<f32>(247.0, 209.0, 61.0) / 255.0,
        vec3<f32>(252.0, 255.0, 164.0) / 255.0
    );
    let position = clamp(t, 0.0, 1.0) * 9.0;
    let index = min(u32(position), 8u);
    return mix(colors[index], colors[index + 1u], position - f32(index));
}

fn turbo(t: f32) -> vec3<f32> {
    let r = clamp(0.13572 + t * (4.6153 + t * (-42.6592 + t * (138.5676 + t * (-152.3494 + t * 59.2859)))), 0.0, 1.0);
    let g = clamp(0.09140 + t * (2.2537 + t * (0.6487 + t * (-23.3910 + t * (38.3522 - t * 18.0858)))), 0.0, 1.0);
    let b = clamp(0.10667 + t * (12.5925 + t * (-60.5820 + t * (109.7316 + t * (-88.2949 + t * 26.7236)))), 0.0, 1.0);
    return vec3<f32>(r, g, b);
}

fn coolwarm(t: f32) -> vec3<f32> {
    if (t < 0.5) {
        return mix(vec3<f32>(0.23, 0.30, 0.75), vec3<f32>(0.87, 0.87, 0.87), t * 2.0);
    }
    return mix(vec3<f32>(0.87, 0.87, 0.87), vec3<f32>(0.71, 0.02, 0.15), (t - 0.5) * 2.0);
}

fn get_color(t: f32, map_id: f32) -> vec3<f32> {
    if (map_id < 0.5) { return viridis(t); }
    if (map_id < 1.5) { return plasma(t); }
    if (map_id < 2.5) { return inferno(t); }
    if (map_id < 3.5) { return turbo(t); }
    return coolwarm(t);
}

fn isoline_alpha(value: f32) -> f32 {
    let step = uniforms.style.x;
    if (step <= 0.0 || uniforms.style.y <= 0.0) { return 0.0; }
    let phase = value / step;
    let distance = abs(fract(phase + 0.5) - 0.5);
    let phase_per_pixel = clamp(fwidth(phase), 0.0001, 0.5);
    let half_width = max(0.5 * uniforms.style.y * phase_per_pixel, 0.35 * phase_per_pixel);
    return 1.0 - smoothstep(half_width, half_width + phase_per_pixel, distance);
}

// Keep this portable across the Naga revisions used by GPUI. Some supported
// WGPU backends do not expose WGSL's isNan/isInf builtins yet.
fn finite(value: f32) -> bool {
    return value == value && abs(value) <= 3.402823466e38;
}

@fragment
fn fragment(input: VertexOut) -> @location(0) vec4<f32> {
    let value = input.value;
    if (!finite(value)) { discard; }
    let span = max(uniforms.range.y - uniforms.range.x, 1e-6);
    let normalized = clamp((value - uniforms.range.x) / span, 0.0, 1.0);
    let base = get_color(normalized, uniforms.range.z);
    let line = isoline_alpha(normalized);
    return vec4<f32>(mix(base, vec3<f32>(0.08, 0.10, 0.14), line), 1.0);
}

@fragment
fn line_fragment(_input: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(0.08, 0.10, 0.14, 0.9);
}
";

#[cfg(test)]
mod tests {
    use super::MESH_WGSL;

    #[test]
    fn cell_field_shader_uses_interpolated_values() {
        assert!(!MESH_WGSL.contains("primitive_index"));
        assert!(!MESH_WGSL.contains("isNan"));
        assert!(!MESH_WGSL.contains("isInf"));
        assert!(MESH_WGSL.contains("values[index]"));
        assert!(MESH_WGSL.contains("let value = input.value;"));
    }
}
