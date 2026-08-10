//! MSL source for the retained 2D mesh backend.
//!
//! The scalar map and isoline AA mirror `shaders.rs`; values are already
//! expanded per triangle when a cell-associated field is uploaded.

pub const MESH_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;

struct Uniforms {
    float4x4 view_transform;
    float4 range;
    float4 style;
};

struct Vertex {
    float3 position [[attribute(0)]];
    float value [[attribute(1)]];
};

struct Out {
    float4 position [[position]];
    float value;
};

vertex Out mesh_vertex(Vertex input [[stage_in]], constant Uniforms& uniforms [[buffer(1)]]) {
    Out output;
    output.position = uniforms.view_transform * float4(input.position, 1.0);
    output.value = input.value;
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
float3 colormap(float t, float map_id) {
    t = clamp(t, 0.0, 1.0);
    if (map_id < 0.5) return viridis(t);
    if (map_id < 1.5) return plasma(t);
    if (map_id < 2.5) return inferno(t);
    if (map_id < 3.5) return turbo(t);
    return coolwarm(t);
}
float isoline_alpha(float value, float step, float width) {
    if (step <= 0.0 || width <= 0.0) return 0.0;
    float phase = value / step;
    float distance = abs(fract(phase + 0.5) - 0.5);
    float phase_per_pixel = clamp(fwidth(phase), 0.0001, 0.5);
    float half_width = max(0.5 * width * phase_per_pixel, 0.35 * phase_per_pixel);
    return 1.0 - smoothstep(half_width, half_width + phase_per_pixel, distance);
}

fragment float4 mesh_fragment(Out input [[stage_in]], constant Uniforms& uniforms [[buffer(1)]]) {
    if (isnan(input.value) || isinf(input.value)) discard_fragment();
    float t = clamp((input.value - uniforms.range.x) / max(uniforms.range.y - uniforms.range.x, 1e-6), 0.0, 1.0);
    float line = isoline_alpha(t, uniforms.style.x, uniforms.style.y);
    return float4(mix(colormap(t, uniforms.range.z), float3(0.08,0.10,0.14), line), 1.0);
}

fragment float4 mesh_line_fragment(Out _input [[stage_in]], constant Uniforms& uniforms [[buffer(1)]]) {
    return float4(0.08, 0.10, 0.14, 0.9);
}
"#;
