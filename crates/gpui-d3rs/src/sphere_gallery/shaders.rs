//! WGSL shaders for sphere gallery rendering

/// Common struct definitions shared by all shaders
pub const COMMON_DEFINITIONS: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    // Atlas layout
    atlas_cols: f32,
    atlas_rows: f32,
    cell_count: f32,
    // Selection state
    selected_index: f32,
    hovered_index: f32,
    // Lighting
    ambient: f32,
    diffuse: f32,
    _pad: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var atlas_texture: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) cell_index: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) cell_index: f32,
    @location(3) world_pos: vec3<f32>,
}
"#;

/// Vertex shader
pub const VERTEX_SHADER: &str = r#"
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_pos = world_pos.xyz;

    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz
    );
    out.world_normal = normalize(normal_matrix * in.normal);
    out.uv = in.uv;
    out.cell_index = in.cell_index;

    return out;
}
"#;

/// Fragment shader - samples from texture atlas with selection highlighting
pub const FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let cell = i32(in.cell_index + 0.5);

    // Skip fragments for cells beyond the image count
    if (f32(cell) >= uniforms.cell_count) {
        // Draw a dark empty cell
        return vec4<f32>(0.08, 0.08, 0.10, 1.0);
    }

    // Compute atlas UV: map local cell UV to position in atlas grid
    let atlas_col = cell % i32(uniforms.atlas_cols);
    let atlas_row = cell / i32(uniforms.atlas_cols);

    let cell_u = (f32(atlas_col) + in.uv.x) / uniforms.atlas_cols;
    let cell_v = (f32(atlas_row) + in.uv.y) / uniforms.atlas_rows;

    let atlas_uv = vec2<f32>(cell_u, cell_v);

    // Sample texture
    var color = textureSample(atlas_texture, atlas_sampler, atlas_uv);

    // Lighting
    let normal = normalize(in.world_normal);
    let light_dir = normalize(vec3<f32>(0.3, 1.0, 0.5));
    let ndotl = max(dot(normal, light_dir), 0.0);
    let lighting = uniforms.ambient + uniforms.diffuse * ndotl;
    color = vec4<f32>(color.rgb * lighting, color.a);

    // Cell border (thin dark line between cells)
    let border_width = 0.02;
    let border_u = min(in.uv.x, 1.0 - in.uv.x);
    let border_v = min(in.uv.y, 1.0 - in.uv.y);
    let border_dist = min(border_u, border_v);
    let border_alpha = 1.0 - smoothstep(0.0, border_width, border_dist);
    color = mix(color, vec4<f32>(0.0, 0.0, 0.0, 1.0), border_alpha * 0.6);

    // Hover highlight
    let is_hovered = abs(in.cell_index - uniforms.hovered_index) < 0.5;
    if (is_hovered && uniforms.hovered_index >= 0.0) {
        let highlight_width = 0.04;
        let highlight_dist = min(border_u, border_v);
        let highlight_alpha = 1.0 - smoothstep(0.0, highlight_width, highlight_dist);
        color = mix(color, vec4<f32>(0.5, 0.7, 1.0, 1.0), highlight_alpha * 0.7);

        // Subtle glow on the whole cell
        color = vec4<f32>(color.rgb + vec3<f32>(0.03, 0.05, 0.08), color.a);
    }

    // Selection highlight
    let is_selected = abs(in.cell_index - uniforms.selected_index) < 0.5;
    if (is_selected && uniforms.selected_index >= 0.0) {
        let sel_width = 0.05;
        let sel_dist = min(border_u, border_v);
        let sel_alpha = 1.0 - smoothstep(0.0, sel_width, sel_dist);
        color = mix(color, vec4<f32>(1.0, 0.85, 0.3, 1.0), sel_alpha * 0.9);

        // Brighter glow on the whole cell
        color = vec4<f32>(color.rgb + vec3<f32>(0.05, 0.04, 0.0), color.a);
    }

    return color;
}
"#;

/// Combined shader source
pub fn combined_shader() -> String {
    format!(
        "{}\n{}\n{}",
        COMMON_DEFINITIONS, VERTEX_SHADER, FRAGMENT_SHADER,
    )
}
