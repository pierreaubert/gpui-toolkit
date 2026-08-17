//! Shared WGSL source for the retained 2D mesh pipeline.
//!
//! The polynomial maps and derivative-based isoline are intentionally kept
//! here rather than duplicated in the backend. The 3D shader has a matching
//! source so a scalar field has the same visual contract in both views.

pub const MESH_WGSL: &str = r#"

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
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let t6 = t5 * t;
    return clamp(
        vec3<f32>(0.2777, 0.0054, 0.3340)
            + vec3<f32>(0.1050, 0.6387, 0.2383) * t
            + vec3<f32>(-0.3308, 0.3143, 0.5287) * t2
            + vec3<f32>(-4.6342, -5.7991, -19.3324) * t3
            + vec3<f32>(6.2282, 14.1799, 56.6905) * t4
            + vec3<f32>(4.7763, -13.7451, -65.3530) * t5
            + vec3<f32>(-5.4354, 4.6456, 26.3124) * t6,
        vec3<f32>(0.0), vec3<f32>(1.0));
}

fn plasma(t: f32) -> vec3<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let t6 = t5 * t;
    return clamp(
        vec3<f32>(0.0504, 0.0298, 0.5280)
            + vec3<f32>(2.0280, -0.3996, -0.1361) * t
            + vec3<f32>(-2.1285, 1.3971, -1.8103) * t2
            + vec3<f32>(-10.2107, 6.8536, 18.8406) * t3
            + vec3<f32>(33.6908, -21.2851, -41.8887) * t4
            + vec3<f32>(-38.8641, 25.8915, 35.6632) * t5
            + vec3<f32>(12.8861, -7.9772, -11.5408) * t6,
        vec3<f32>(0.0), vec3<f32>(1.0));
}

fn inferno(t: f32) -> vec3<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let t6 = t5 * t;
    return clamp(
        vec3<f32>(0.0002, 0.0016, 0.0139)
            + vec3<f32>(0.1260, 0.4023, 1.3241) * t
            + vec3<f32>(1.1661, 0.0868, -2.1073) * t2
            + vec3<f32>(-1.0127, 2.0841, 2.4048) * t3
            + vec3<f32>(-8.8174, 0.1567, -2.5439) * t4
            + vec3<f32>(17.5174, -4.5424, 0.8282) * t5
            + vec3<f32>(-9.5028, 3.3025, 0.0987) * t6,
        vec3<f32>(0.0), vec3<f32>(1.0));
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
"#;

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
