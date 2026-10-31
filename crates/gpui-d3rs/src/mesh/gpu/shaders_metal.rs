//! MSL source for the retained 2D mesh backend.
//!
//! The scalar map and isoline AA mirror `shaders.rs`; values are already
//! expanded per triangle when a cell-associated field is uploaded.

pub const MESH_MSL: &str = r"
#include <metal_stdlib>
using namespace metal;

struct Uniforms {
    float4x4 view_transform;
    float4 range;
    float4 style;
};

struct Vertex {
    float4 position;
    float4 normal;
    float value;
};

struct Out {
    float4 position [[position]];
    float value;
};

vertex Out mesh_vertex(
    const device Vertex* vertices [[buffer(0)]],
    uint vertex_id [[vertex_id]],
    constant Uniforms& uniforms [[buffer(1)]]) {
    Vertex input = vertices[vertex_id];
    Out output;
    output.position = uniforms.view_transform * float4(input.position.xyz, 1.0);
    output.value = input.value;
    return output;
}

float3 viridis(float t) {
    // Match the CPU ColorScale palette and its piecewise RGB interpolation.
    const float3 colors[10] = {
        float3(68.0, 1.0, 84.0) / 255.0,
        float3(72.0, 40.0, 120.0) / 255.0,
        float3(62.0, 74.0, 137.0) / 255.0,
        float3(49.0, 104.0, 142.0) / 255.0,
        float3(38.0, 131.0, 143.0) / 255.0,
        float3(31.0, 158.0, 137.0) / 255.0,
        float3(53.0, 183.0, 121.0) / 255.0,
        float3(110.0, 206.0, 88.0) / 255.0,
        float3(181.0, 222.0, 43.0) / 255.0,
        float3(253.0, 231.0, 37.0) / 255.0
    };
    float position = clamp(t, 0.0, 1.0) * 9.0;
    uint index = min(uint(position), 8u);
    return mix(colors[index], colors[index + 1], position - float(index));
}
float3 plasma(float t) {
    // Match the CPU ColorScale palette and its piecewise RGB interpolation.
    const float3 colors[10] = {
        float3(13.0, 8.0, 135.0) / 255.0,
        float3(70.0, 3.0, 159.0) / 255.0,
        float3(114.0, 1.0, 168.0) / 255.0,
        float3(156.0, 23.0, 158.0) / 255.0,
        float3(189.0, 55.0, 134.0) / 255.0,
        float3(216.0, 87.0, 107.0) / 255.0,
        float3(237.0, 121.0, 83.0) / 255.0,
        float3(251.0, 159.0, 58.0) / 255.0,
        float3(253.0, 202.0, 38.0) / 255.0,
        float3(240.0, 249.0, 33.0) / 255.0
    };
    float position = clamp(t, 0.0, 1.0) * 9.0;
    uint index = min(uint(position), 8u);
    return mix(colors[index], colors[index + 1], position - float(index));
}
float3 inferno(float t) {
    // Match the CPU ColorScale palette and its piecewise RGB interpolation.
    const float3 colors[10] = {
        float3(0.0, 0.0, 4.0) / 255.0,
        float3(27.0, 12.0, 65.0) / 255.0,
        float3(74.0, 12.0, 107.0) / 255.0,
        float3(120.0, 28.0, 109.0) / 255.0,
        float3(165.0, 44.0, 96.0) / 255.0,
        float3(207.0, 68.0, 70.0) / 255.0,
        float3(237.0, 105.0, 37.0) / 255.0,
        float3(251.0, 155.0, 6.0) / 255.0,
        float3(247.0, 209.0, 61.0) / 255.0,
        float3(252.0, 255.0, 164.0) / 255.0
    };
    float position = clamp(t, 0.0, 1.0) * 9.0;
    uint index = min(uint(position), 8u);
    return mix(colors[index], colors[index + 1], position - float(index));
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
    float line = isoline_alpha(t, uniforms.range.w, uniforms.style.x);
    return float4(mix(colormap(t, uniforms.range.z), float3(0.08,0.10,0.14), line), 1.0);
}

fragment float4 mesh_line_fragment(Out _input [[stage_in]], constant Uniforms& uniforms [[buffer(1)]]) {
    return float4(0.08, 0.10, 0.14, 0.9);
}
";
