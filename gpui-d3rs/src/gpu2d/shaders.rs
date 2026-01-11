//! WGSL shaders for 2D chart rendering

/// Common shader code shared by all 2D primitives
pub fn common_shader() -> &'static str {
    r#"
// Common uniforms for all 2D shaders
struct Uniforms {
    viewport_size: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

// Convert pixel coordinates to NDC (normalized device coordinates)
fn pixel_to_ndc(pos: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (pos.x / uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / uniforms.viewport_size.y) * 2.0
    );
}
"#
}

/// Line shader - renders line segments as quads expanded by perpendicular normals
pub fn line_shader() -> String {
    format!(
        r#"
{common}

struct LineVertex {{
    @location(0) position: vec2<f32>,
    @location(1) normal: vec2<f32>,
    @location(2) color: vec4<f32>,
}}

struct LineOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}}

@vertex
fn vs_line(in: LineVertex) -> LineOutput {{
    let expanded = in.position + in.normal;
    var out: LineOutput;
    out.position = vec4<f32>(pixel_to_ndc(expanded), 0.0, 1.0);
    out.color = in.color;
    return out;
}}

@fragment
fn fs_line(in: LineOutput) -> @location(0) vec4<f32> {{
    return in.color;
}}
"#,
        common = common_shader()
    )
}

/// Rectangle shader - renders rectangles with optional rounded corners using SDF
pub fn rect_shader() -> String {
    format!(
        r#"
{common}

struct RectVertex {{
    @location(0) position: vec2<f32>,
    @location(1) rect_min: vec2<f32>,
    @location(2) rect_max: vec2<f32>,
    @location(3) corner_radius: f32,
    @location(4) color: vec4<f32>,
}}

struct RectOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) rect_min: vec2<f32>,
    @location(2) rect_max: vec2<f32>,
    @location(3) corner_radius: f32,
    @location(4) color: vec4<f32>,
}}

@vertex
fn vs_rect(in: RectVertex) -> RectOutput {{
    var out: RectOutput;
    out.position = vec4<f32>(pixel_to_ndc(in.position), 0.0, 1.0);
    out.local_pos = in.position;
    out.rect_min = in.rect_min;
    out.rect_max = in.rect_max;
    out.corner_radius = in.corner_radius;
    out.color = in.color;
    return out;
}}

@fragment
fn fs_rect(in: RectOutput) -> @location(0) vec4<f32> {{
    let half_size = (in.rect_max - in.rect_min) * 0.5;
    let center = (in.rect_min + in.rect_max) * 0.5;
    let p = abs(in.local_pos - center) - half_size + in.corner_radius;
    let d = length(max(p, vec2<f32>(0.0))) - in.corner_radius;

    // Anti-aliased edge
    let alpha = 1.0 - smoothstep(-1.0, 1.0, d);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}}
"#,
        common = common_shader()
    )
}

/// Circle shader - renders circles/points using SDF with anti-aliasing
pub fn circle_shader() -> String {
    format!(
        r#"
{common}

struct CircleVertex {{
    @location(0) position: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) radius: f32,
    @location(3) color: vec4<f32>,
}}

struct CircleOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,
    @location(1) center: vec2<f32>,
    @location(2) radius: f32,
    @location(3) color: vec4<f32>,
}}

@vertex
fn vs_circle(in: CircleVertex) -> CircleOutput {{
    var out: CircleOutput;
    out.position = vec4<f32>(pixel_to_ndc(in.position), 0.0, 1.0);
    out.local_pos = in.position;
    out.center = in.center;
    out.radius = in.radius;
    out.color = in.color;
    return out;
}}

@fragment
fn fs_circle(in: CircleOutput) -> @location(0) vec4<f32> {{
    let dist = length(in.local_pos - in.center);
    // Anti-aliased edge with 1px smoothing
    let alpha = 1.0 - smoothstep(in.radius - 1.0, in.radius + 1.0, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}}
"#,
        common = common_shader()
    )
}

/// Triangle shader - renders filled triangles for polygon fills
pub fn triangle_shader() -> String {
    format!(
        r#"
{common}

struct TriangleVertex {{
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}}

struct TriangleOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}}

@vertex
fn vs_triangle(in: TriangleVertex) -> TriangleOutput {{
    var out: TriangleOutput;
    out.position = vec4<f32>(pixel_to_ndc(in.position), 0.0, 1.0);
    out.color = in.color;
    return out;
}}

@fragment
fn fs_triangle(in: TriangleOutput) -> @location(0) vec4<f32> {{
    return in.color;
}}
"#,
        common = common_shader()
    )
}

/// Text shader - renders glyphs from a font atlas texture
pub fn text_shader() -> String {
    format!(
        r#"
{common}

@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct TextVertex {{
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
}}

struct TextOutput {{
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
}}

@vertex
fn vs_text(in: TextVertex) -> TextOutput {{
    var out: TextOutput;
    out.position = vec4<f32>(pixel_to_ndc(in.position), 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = in.color;
    return out;
}}

@fragment
fn fs_text(in: TextOutput) -> @location(0) vec4<f32> {{
    let alpha = textureSample(atlas_texture, atlas_sampler, in.tex_coord).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}}
"#,
        common = common_shader()
    )
}
