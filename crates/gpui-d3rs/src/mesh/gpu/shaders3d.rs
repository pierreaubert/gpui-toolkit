//! WGSL and MSL sources for the retained unstructured 3D mesh pass.
//!
//! The two sources deliberately keep scalar mapping, masking, and derivative
//! isoline behavior aligned with the 2D retained mesh shader.

/// WGSL surface, wireframe, and orientation-triad shader.
pub const MESH_3D_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    light_dir: vec4<f32>,
    params: vec4<f32>,
    value_range: vec4<f32>,
    isoline: vec4<f32>,
    isoline_color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read> values: array<f32>;

struct VertexIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) value: f32,
};

@vertex
fn vs_main(input: VertexIn, @builtin(vertex_index) vertex_index: u32) -> VertexOut {
    var output: VertexOut;
    let world = uniforms.model * vec4<f32>(input.position, 1.0);
    output.clip_position = uniforms.view_proj * world;
    output.normal = normalize((uniforms.model * vec4<f32>(input.normal, 0.0)).xyz);
    if (uniforms.value_range.z > 0.5) {
        output.value = values[vertex_index];
    } else {
        output.value = 0.0;
    }
    return output;
}

// Triad positions are already in NDC. Its pipeline has no depth attachment,
// so the axes remain visible over the surface and track camera orientation
// through the small CPU-updated vertex buffer.
@vertex
fn vs_triad(input: VertexIn) -> VertexOut {
    var output: VertexOut;
    output.clip_position = vec4<f32>(input.position.xy, 0.0, 1.0);
    output.normal = input.normal;
    output.value = 1.0;
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
    return vec3<f32>(
        clamp(0.13572 + t * (4.6153 + t * (-42.6592 + t * (138.5676 + t * (-152.3494 + t * 59.2859)))), 0.0, 1.0),
        clamp(0.09140 + t * (2.2537 + t * (0.6487 + t * (-23.3910 + t * (38.3522 - t * 18.0858)))), 0.0, 1.0),
        clamp(0.10667 + t * (12.5925 + t * (-60.5820 + t * (109.7316 + t * (-88.2949 + t * 26.7236)))), 0.0, 1.0),
    );
}

fn coolwarm(t: f32) -> vec3<f32> {
    if (t < 0.5) {
        return mix(vec3<f32>(0.23, 0.30, 0.75), vec3<f32>(0.87, 0.87, 0.87), t * 2.0);
    }
    return mix(vec3<f32>(0.87, 0.87, 0.87), vec3<f32>(0.71, 0.02, 0.15), (t - 0.5) * 2.0);
}

fn colormap(value: f32, index: f32) -> vec3<f32> {
    let t = clamp(value, 0.0, 1.0);
    if (index < 0.5) { return viridis(t); }
    if (index < 1.5) { return plasma(t); }
    if (index < 2.5) { return inferno(t); }
    if (index < 3.5) { return turbo(t); }
    return coolwarm(t);
}

fn isoline_alpha(value: f32, step: f32, width: f32) -> f32 {
    if (step <= 0.0 || width <= 0.0) { return 0.0; }
    let phase = value / step;
    let distance = abs(fract(phase + 0.5) - 0.5);
    // Derivative implementations can differ by a few ulps between Metal and
    // WGPU even when they target the same adapter. Quantize the pixel phase
    // width before the smoothstep so antialiased isolines have stable 8-bit
    // output at adapter boundaries.
    let phase_per_pixel = clamp(round(clamp(fwidth(phase), 0.0001, 0.5) * 64.0) / 64.0, 0.0001, 0.5);
    let half_width = max(0.5 * width * phase_per_pixel, 0.35 * phase_per_pixel);
    return 1.0 - smoothstep(half_width, half_width + phase_per_pixel, distance);
}

// Keep this portable across the Naga revisions used by GPUI. Some supported
// WGPU backends do not expose WGSL's isNan/isInf builtins yet.
fn finite(value: f32) -> bool {
    return value == value && abs(value) <= 3.402823466e38;
}

@fragment
fn fs_main(input: VertexOut) -> @location(0) vec4<f32> {
    if (!finite(input.value)) { discard; }
    var lambert = 1.0;
    if (uniforms.params.y <= 0.5) {
        lambert = uniforms.params.z + uniforms.params.w * abs(dot(normalize(input.normal), normalize(uniforms.light_dir.xyz)));
    }
    let normalized = clamp(
        (input.value - uniforms.value_range.x) /
            max(uniforms.value_range.y - uniforms.value_range.x, 1e-6),
        0.0, 1.0);
    let base = colormap(normalized, uniforms.params.x);
    let line = isoline_alpha(normalized, uniforms.isoline.x, uniforms.isoline.y) * uniforms.isoline.z;
    return vec4<f32>(mix(base * lambert, uniforms.isoline_color.rgb, line), uniforms.isoline_color.a);
}

@fragment
fn fs_wireframe(_input: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(uniforms.isoline_color.rgb, uniforms.isoline_color.a);
}

@fragment
fn fs_triad(input: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(input.normal, 1.0);
}
"#;

/// MSL twin kept beside the WGSL source for Metal-backed hosts.
pub const MESH_3D_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Uniforms {
    float4x4 view_proj;
    float4x4 model;
    float4 light_dir;
    float4 params;
    float4 value_range;
    float4 isoline;
    float4 isoline_color;
};

struct VertexIn {
    float4 position [[attribute(0)]];
    float4 normal [[attribute(1)]];
    // Keep the raw Metal buffer stride identical to Rust's `MetalVertex`:
    // position (16) + normal (16) + value (4) + padding (12).  The 3D
    // scalar is read from the separate values buffer in `vs_main`, but it
    // still occupies space in the interleaved upload and must be represented
    // here so vertex_id advances by 48 bytes rather than 32. Keep the
    // trailing padding as scalar members: Metal gives a float3 16-byte
    // alignment, which would incorrectly make this struct 64 bytes.
    float value [[attribute(2)]];
    float _padding[3];
};
static_assert(sizeof(VertexIn) == 48, "MetalVertex ABI must remain 48 bytes");

struct VertexOut {
    float4 position [[position]];
    float3 normal;
    float value;
};

vertex VertexOut vs_main(
    const device VertexIn* vertices [[buffer(0)]],
    uint vertex_id [[vertex_id]],
    constant Uniforms& uniforms [[buffer(1)]],
    const device float* values [[buffer(2)]]) {
  VertexIn input = vertices[vertex_id];
    VertexOut output;
    float4 world = uniforms.model * float4(input.position.xyz, 1.0);
    output.position = uniforms.view_proj * world;
    output.normal = normalize((uniforms.model * float4(input.normal.xyz, 0.0)).xyz);
    output.value = uniforms.value_range.z > 0.5 ? values[vertex_id] : 0.0;
    return output;
}

vertex VertexOut vs_triad(
    const device VertexIn* vertices [[buffer(0)]],
    uint vertex_id [[vertex_id]]) {
  VertexIn input = vertices[vertex_id];
    VertexOut output;
    output.position = float4(input.position.xy, 0.0, 1.0);
    output.normal = input.normal.xyz;
    output.value = 1.0;
    return output;
}

float3 viridis(float t) {
    float t2=t*t, t3=t2*t, t4=t3*t, t5=t4*t, t6=t5*t;
    return clamp(float3(0.2777,0.0054,0.3340) + float3(0.1050,0.6387,0.2383)*t
        + float3(-0.3308,0.3143,0.5287)*t2 + float3(-4.6342,-5.7991,-19.3324)*t3
        + float3(6.2282,14.1799,56.6905)*t4 + float3(4.7763,-13.7451,-65.3530)*t5
        + float3(-5.4354,4.6456,26.3124)*t6, 0.0, 1.0);
}
float3 plasma(float t) {
    float t2=t*t, t3=t2*t, t4=t3*t, t5=t4*t, t6=t5*t;
    return clamp(float3(0.0504,0.0298,0.5280) + float3(2.0280,-0.3996,-0.1361)*t
        + float3(-2.1285,1.3971,-1.8103)*t2 + float3(-10.2107,6.8536,18.8406)*t3
        + float3(33.6908,-21.2851,-41.8887)*t4 + float3(-38.8641,25.8915,35.6632)*t5
        + float3(12.8861,-7.9772,-11.5408)*t6, 0.0, 1.0);
}
float3 inferno(float t) {
    float t2=t*t, t3=t2*t, t4=t3*t, t5=t4*t, t6=t5*t;
    return clamp(float3(0.0002,0.0016,0.0139) + float3(0.1260,0.4023,1.3241)*t
        + float3(1.1661,0.0868,-2.1073)*t2 + float3(-1.0127,2.0841,2.4048)*t3
        + float3(-8.8174,0.1567,-2.5439)*t4 + float3(17.5174,-4.5424,0.8282)*t5
        + float3(-9.5028,3.3025,0.0987)*t6, 0.0, 1.0);
}
float3 turbo(float t) {
    return float3(clamp(0.13572+t*(4.6153+t*(-42.6592+t*(138.5676+t*(-152.3494+t*59.2859)))),0.0,1.0),
        clamp(0.09140+t*(2.2537+t*(0.6487+t*(-23.3910+t*(38.3522-t*18.0858)))),0.0,1.0),
        clamp(0.10667+t*(12.5925+t*(-60.5820+t*(109.7316+t*(-88.2949+t*26.7236)))),0.0,1.0));
}
float3 coolwarm(float t) {
    return t < 0.5 ? mix(float3(0.23,0.30,0.75), float3(0.87), t*2.0)
                   : mix(float3(0.87), float3(0.71,0.02,0.15), (t-0.5)*2.0);
}
float3 colormap(float t, float index) {
    t = clamp(t, 0.0, 1.0);
    if (index < 0.5) return viridis(t);
    if (index < 1.5) return plasma(t);
    if (index < 2.5) return inferno(t);
    if (index < 3.5) return turbo(t);
    return coolwarm(t);
}
float isoline_alpha(float value, float step, float width) {
    if (step <= 0.0 || width <= 0.0) return 0.0;
    float phase = value / step;
    float distance = abs(fract(phase + 0.5) - 0.5);
    // Keep antialiased isoline output stable across Metal and WGPU derivative
    // implementations by using the same fixed phase-width quantization.
    float phase_per_pixel = clamp(round(clamp(fwidth(phase), 0.0001, 0.5) * 64.0) / 64.0, 0.0001, 0.5);
    float half_width = max(0.5 * width * phase_per_pixel, 0.35 * phase_per_pixel);
    return 1.0 - smoothstep(half_width, half_width + phase_per_pixel, distance);
}

fragment float4 fs_main(VertexOut input [[stage_in]], constant Uniforms& uniforms [[buffer(0)]]) {
    if (isnan(input.value) || isinf(input.value)) discard_fragment();
    float lighting = 1.0;
    if (uniforms.params.y <= 0.5) {
        lighting = uniforms.params.z + uniforms.params.w * abs(dot(normalize(input.normal), normalize(uniforms.light_dir.xyz)));
    }
    float t = clamp((input.value - uniforms.value_range.x) / max(uniforms.value_range.y - uniforms.value_range.x, 1e-6), 0.0, 1.0);
    float line = isoline_alpha(t, uniforms.isoline.x, uniforms.isoline.y) * uniforms.isoline.z;
    return float4(mix(colormap(t, uniforms.params.x) * lighting, uniforms.isoline_color.rgb, line), uniforms.isoline_color.a);
}
fragment float4 fs_wireframe(VertexOut _input [[stage_in]], constant Uniforms& uniforms [[buffer(0)]]) {
    return uniforms.isoline_color;
}
fragment float4 fs_triad(VertexOut input [[stage_in]]) {
    return float4(input.normal, 1.0);
}
"#;

pub const fn wgsl() -> &'static str {
    MESH_3D_WGSL
}
pub const fn msl() -> &'static str {
    MESH_3D_MSL
}
