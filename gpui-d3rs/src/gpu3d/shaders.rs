//! WGSL shaders for 3D surface rendering

/// Common struct definitions shared by all shaders
pub const COMMON_DEFINITIONS: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    light_dir: vec3<f32>,
    colormap: f32,
    ambient: f32,
    diffuse: f32,
    opacity: f32,
    z_min: f32,
    x_min_log: f32,
    x_range_log: f32,
    is_log_x: f32,
    show_surface_isolines: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) value: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) normalized_value: f32,
    @location(2) world_pos: vec3<f32>,
}
"#;

/// Vertex shader for surface rendering
pub const SURFACE_VERTEX_SHADER: &str = r#"
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let world_pos = uniforms.model * vec4<f32>(in.position, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_pos = world_pos.xyz;

    // Transform normal to world space (assuming uniform scaling)
    let normal_matrix = mat3x3<f32>(
        uniforms.model[0].xyz,
        uniforms.model[1].xyz,
        uniforms.model[2].xyz
    );
    out.world_normal = normalize(normal_matrix * in.normal);

    // Pass through normalized value for colormap
    out.normalized_value = in.value;

    return out;
}
"#;

/// Fragment shader for surface rendering with colormap
pub const SURFACE_FRAGMENT_SHADER: &str = r#"

// Viridis colormap approximation
fn viridis(t: f32) -> vec3<f32> {
    let c0 = vec3<f32>(0.2777, 0.0054, 0.3340);
    let c1 = vec3<f32>(0.1050, 0.6387, 0.2383);
    let c2 = vec3<f32>(-0.3308, 0.3143, 0.5287);
    let c3 = vec3<f32>(-4.6342, -5.7991, -19.3324);
    let c4 = vec3<f32>(6.2282, 14.1799, 56.6905);
    let c5 = vec3<f32>(4.7763, -13.7451, -65.3530);
    let c6 = vec3<f32>(-5.4354, 4.6456, 26.3124);

    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let t6 = t5 * t;

    return clamp(c0 + c1*t + c2*t2 + c3*t3 + c4*t4 + c5*t5 + c6*t6, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Plasma colormap approximation
fn plasma(t: f32) -> vec3<f32> {
    let c0 = vec3<f32>(0.0504, 0.0298, 0.5280);
    let c1 = vec3<f32>(2.0280, -0.3996, -0.1361);
    let c2 = vec3<f32>(-2.1285, 1.3971, -1.8103);
    let c3 = vec3<f32>(-10.2107, 6.8536, 18.8406);
    let c4 = vec3<f32>(33.6908, -21.2851, -41.8887);
    let c5 = vec3<f32>(-38.8641, 25.8915, 35.6632);
    let c6 = vec3<f32>(12.8861, -7.9772, -11.5408);

    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let t6 = t5 * t;

    return clamp(c0 + c1*t + c2*t2 + c3*t3 + c4*t4 + c5*t5 + c6*t6, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Inferno colormap approximation
fn inferno(t: f32) -> vec3<f32> {
    let c0 = vec3<f32>(0.0002, 0.0016, 0.0139);
    let c1 = vec3<f32>(0.1260, 0.4023, 1.3241);
    let c2 = vec3<f32>(1.1661, 0.0868, -2.1073);
    let c3 = vec3<f32>(-1.0127, 2.0841, 2.4048);
    let c4 = vec3<f32>(-8.8174, 0.1567, -2.5439);
    let c5 = vec3<f32>(17.5174, -4.5424, 0.8282);
    let c6 = vec3<f32>(-9.5028, 3.3025, 0.0987);

    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;
    let t6 = t5 * t;

    return clamp(c0 + c1*t + c2*t2 + c3*t3 + c4*t4 + c5*t5 + c6*t6, vec3<f32>(0.0), vec3<f32>(1.0));
}

// Turbo colormap (Google's improved rainbow)
fn turbo(t: f32) -> vec3<f32> {
    let r = clamp(0.13572 + t * (4.6153 + t * (-42.6592 + t * (138.5676 + t * (-152.3494 + t * 59.2859)))), 0.0, 1.0);
    let g = clamp(0.09140 + t * (2.2537 + t * (0.6487 + t * (-23.3910 + t * (38.3522 - t * 18.0858)))), 0.0, 1.0);
    let b = clamp(0.10667 + t * (12.5925 + t * (-60.5820 + t * (109.7316 + t * (-88.2949 + t * 26.7236)))), 0.0, 1.0);
    return vec3<f32>(r, g, b);
}

// Cool-warm diverging colormap
fn coolwarm(t: f32) -> vec3<f32> {
    // Blue (cool) to red (warm) through white
    let mid = 0.5;
    if (t < mid) {
        let s = t / mid;
        return mix(vec3<f32>(0.23, 0.30, 0.75), vec3<f32>(0.87, 0.87, 0.87), s);
    } else {
        let s = (t - mid) / (1.0 - mid);
        return mix(vec3<f32>(0.87, 0.87, 0.87), vec3<f32>(0.71, 0.02, 0.15), s);
    }
}

fn get_color(t: f32, map_id: f32) -> vec3<f32> {
    if (map_id < 0.5) {
        return viridis(t);
    } else if (map_id < 1.5) {
        return plasma(t);
    } else if (map_id < 2.5) {
        return inferno(t);
    } else if (map_id < 3.5) {
        return turbo(t);
    } else {
        return coolwarm(t);
    }
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate lighting
    let normal = normalize(in.world_normal);
    let light_dir = normalize(uniforms.light_dir);

    // Two-sided lighting
    let ndotl = abs(dot(normal, light_dir));
    let lighting = uniforms.ambient + uniforms.diffuse * ndotl;

    // Apply colormap
    let base_color = get_color(in.normalized_value, uniforms.colormap);

    // Combine lighting with color
    let final_color = base_color * lighting;

    var color = final_color;

    // Isolines on surface (controlled by uniform)
    if (uniforms.show_surface_isolines > 0.5) {
        // Every 3dB. Assuming normalized value 0..1 maps to range (e.g. 50dB).
        // 3dB is approx 0.06 normalized units.
        let step = 0.06;
        let line_width = 0.001;
        let feather = 0.0005;

        let dist = abs(fract(in.normalized_value / step) - 0.5) * step;

        // Anti-aliased line
        let line_alpha = 1.0 - smoothstep(line_width - feather, line_width + feather, dist);

        if (line_alpha > 0.0) {
            // Blend black line
            color = mix(color, vec3<f32>(0.0, 0.0, 0.0), line_alpha * 0.5);
        }
    }

    return vec4<f32>(color, uniforms.opacity);
}
"#;

/// Simple wireframe shader (uses same vertex shader)
pub const WIREFRAME_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.2, 0.2, 0.2, 0.5);
}
"#;

/// Vertex shader for projection/isolines
pub const PROJECTION_VERTEX_SHADER: &str = r#"
@vertex
fn vs_projection(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Flatten to Y plane (bottom/floor)
    // Position is (x=Freq, y=SPL, z=Angle)
    // We want to project to the floor (y = -0.5)
    // Offset slightly to -0.499 to avoid Z-fighting with grid floor
    let flat_pos = vec3<f32>(in.position.x, -0.499, in.position.z);
    let world_pos = uniforms.model * vec4<f32>(flat_pos, 1.0);
    out.clip_position = uniforms.view_proj * world_pos;
    out.world_pos = world_pos.xyz;
    out.world_normal = vec3<f32>(0.0, 1.0, 0.0); // Normal points up
    out.normalized_value = in.value;

    return out;
}
"#;

/// Fragment shader for isolines
pub const PROJECTION_FRAGMENT_SHADER: &str = r#"
@fragment
fn fs_projection(in: VertexOutput) -> @location(0) vec4<f32> {
    let value = in.normalized_value;
    // Isolines every 3dB
    let step = 0.06;
    let line_width = 0.001;
    let feather = 0.0005;

    let dist = abs(fract(value / step) - 0.5) * step;

    // Anti-aliased line
    let line_alpha = 1.0 - smoothstep(line_width - feather, line_width + feather, dist);

    if (line_alpha > 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, line_alpha * 0.8); // Black isolines
    } else {
        discard; // Transparent between lines
    }
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
}
"#;

/// Fragment shader for grid box
pub const GRID_FRAGMENT_SHADER: &str = r#"

// Helper: compute distance to nearest log-spaced grid line
// For frequency axis with standard audio ticks: 100, 200, 500, 1k, 2k, 5k, 10k, 20k
// Major lines at decades (100, 1000, 10000)
// Minor lines at 2x and 5x within each decade (200, 500, 2000, 5000, 20000)
fn log_grid_distance(u: f32, x_min_log: f32, x_range_log: f32) -> vec2<f32> {
    // Convert u [0,1] to log10 frequency value
    let log_freq = u * x_range_log + x_min_log;
    let freq = pow(10.0, log_freq);

    // Find which decade we're in
    let decade = floor(log_freq);
    let decade_base = pow(10.0, decade);

    // Standard tick positions within decade: 1x, 2x, 5x (and 10x = next decade)
    let multipliers = array<f32, 4>(1.0, 2.0, 5.0, 10.0);

    var min_major_dist = 1.0;
    var min_minor_dist = 1.0;

    // Check ticks in current and adjacent decades
    for (var d = -1; d <= 1; d++) {
        let base = pow(10.0, decade + f32(d));
        for (var i = 0u; i < 4u; i++) {
            let tick_freq = base * multipliers[i];
            let tick_log = log2(tick_freq) / log2(10.0); // log10
            let tick_u = (tick_log - x_min_log) / x_range_log;
            let dist = abs(u - tick_u);

            // Major ticks: 1x of each decade (100, 1000, 10000)
            if (i == 0u) {
                min_major_dist = min(min_major_dist, dist);
            }
            // Minor ticks: 2x and 5x (200, 500, 2000, 5000, etc.)
            min_minor_dist = min(min_minor_dist, dist);
        }
    }

    return vec2<f32>(min_major_dist, min_minor_dist);
}

@fragment
fn fs_grid(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = in.world_pos;

    // Determine which face we are on and set up UV coordinates
    // Box bounds: X=[-1,1], Y=[-0.5,0.5], Z=[-1,1]
    //
    // Face axis mapping:
    // - Bottom/Top faces (Y=-0.5, Y=0.5): u=X (freq), v=Z (angle)
    // - Left/Right faces (X=-1, X=1): u=Z (angle), v=Y (SPL)
    // - Front/Back faces (Z=-1, Z=1): u=X (freq), v=Y (SPL)
    //
    // Edge ownership to avoid double-drawing:
    // - Bottom/Top faces: own all 4 edges
    // - Left/Right faces: own vertical edges only (Z=-1 and Z=1)
    // - Front/Back faces: own NO edges

    var u = 0.0;
    var v = 0.0;
    var is_face = false;
    var draw_u_border = false;
    var draw_v_border = false;
    var draw_u_grid = true;   // Draw grid lines in u direction
    var draw_v_grid = true;   // Draw grid lines in v direction

    // Axis type flags
    var u_is_freq = false;   // u is frequency (X) axis
    var u_is_angle = false;  // u is angle (Z) axis
    var v_is_angle = false;  // v is angle (Z) axis
    var v_is_spl = false;    // v is SPL (Y) axis

    let eps = 0.01;
    let edge_margin = 0.02;  // Margin to avoid drawing grid lines at shared edges

    // Check faces in priority order for edge ownership
    if (abs(pos.y + 0.5) < eps) {
        // Bottom face (Y=-0.5): u=X (freq), v=Z (angle)
        // Owns all edges - draws everything
        u = (pos.x + 1.0) * 0.5;
        v = (pos.z + 1.0) * 0.5;
        is_face = true;
        draw_u_border = true;
        draw_v_border = true;
        u_is_freq = true;
        v_is_angle = true;
    } else if (abs(pos.y - 0.5) < eps) {
        // Top face (Y=0.5): u=X (freq), v=Z (angle)
        // Owns all edges - draws everything
        u = (pos.x + 1.0) * 0.5;
        v = (pos.z + 1.0) * 0.5;
        is_face = true;
        draw_u_border = true;
        draw_v_border = true;
        u_is_freq = true;
        v_is_angle = true;
    } else if (abs(pos.x + 1.0) < eps) {
        // Left face (X=-1): u=Z (angle), v=Y (SPL)
        // Owns u edges (Z), not v edges (Y - owned by top/bottom)
        u = (pos.z + 1.0) * 0.5;
        v = pos.y + 0.5;
        is_face = true;
        draw_u_border = true;
        draw_v_border = false;
        u_is_angle = true;
        v_is_spl = true;
        // Don't draw v grid lines at top/bottom edges (v near 0 or 1)
        if (v < edge_margin || v > 1.0 - edge_margin) {
            draw_v_grid = false;
        }
    } else if (abs(pos.x - 1.0) < eps) {
        // Right face (X=1): u=Z (angle), v=Y (SPL)
        u = (pos.z + 1.0) * 0.5;
        v = pos.y + 0.5;
        is_face = true;
        draw_u_border = true;
        draw_v_border = false;
        u_is_angle = true;
        v_is_spl = true;
        // Don't draw v grid lines at top/bottom edges
        if (v < edge_margin || v > 1.0 - edge_margin) {
            draw_v_grid = false;
        }
    } else if (abs(pos.z + 1.0) < eps) {
        // Back face (Z=-1): u=X (freq), v=Y (SPL)
        // Owns no edges - avoid drawing at all boundaries
        u = (pos.x + 1.0) * 0.5;
        v = pos.y + 0.5;
        is_face = true;
        draw_u_border = false;
        draw_v_border = false;
        u_is_freq = true;
        v_is_spl = true;
        // Don't draw grid lines at edges owned by other faces
        if (u < edge_margin || u > 1.0 - edge_margin) {
            draw_u_grid = false;  // X edges owned by left/right
        }
        if (v < edge_margin || v > 1.0 - edge_margin) {
            draw_v_grid = false;  // Y edges owned by top/bottom
        }
    } else if (abs(pos.z - 1.0) < eps) {
        // Front face (Z=1): u=X (freq), v=Y (SPL)
        u = (pos.x + 1.0) * 0.5;
        v = pos.y + 0.5;
        is_face = true;
        draw_u_border = false;
        draw_v_border = false;
        u_is_freq = true;
        v_is_spl = true;
        // Don't draw grid lines at edges owned by other faces
        if (u < edge_margin || u > 1.0 - edge_margin) {
            draw_u_grid = false;
        }
        if (v < edge_margin || v > 1.0 - edge_margin) {
            draw_v_grid = false;
        }
    }

    if (!is_face) {
        discard;
    }

    // Analytic AA using derivatives
    let du = clamp(fwidth(u), 0.0001, 0.1);
    let dv = clamp(fwidth(v), 0.0001, 0.1);

    // Distance to border lines
    let dist_u_border = min(u, 1.0 - u);
    let dist_v_border = min(v, 1.0 - v);

    // Grid line distances
    var dist_u_maj = 0.0;
    var dist_u_min = 0.0;
    var dist_v_maj = 0.0;
    var dist_v_min = 0.0;

    // Grid parameters:
    // - Frequency (X): logarithmic with decades as major, 2x/5x as minor
    // - Angle (Z): 30° major, 10° minor (12 major / 36 minor for 360° range)
    // - SPL (Y): 10dB major, 2dB minor (5 major / 25 minor for 50dB range)

    // U-axis grid
    if (u_is_freq && uniforms.is_log_x > 0.5) {
        // Frequency axis - logarithmic
        let log_dists = log_grid_distance(u, uniforms.x_min_log, uniforms.x_range_log);
        dist_u_maj = log_dists.x;
        dist_u_min = log_dists.y;
    } else if (u_is_freq) {
        // Linear frequency (rare case)
        dist_u_maj = abs(fract(u * 5.0 + 0.5) - 0.5) / 5.0;
        dist_u_min = abs(fract(u * 25.0 + 0.5) - 0.5) / 25.0;
    } else if (u_is_angle) {
        // Angle axis: 30° major (12 divisions), 10° minor (36 divisions)
        dist_u_maj = abs(fract(u * 12.0 + 0.5) - 0.5) / 12.0;
        dist_u_min = abs(fract(u * 36.0 + 0.5) - 0.5) / 36.0;
    }

    // V-axis grid
    if (v_is_angle) {
        // Angle axis: 30° major, 10° minor
        dist_v_maj = abs(fract(v * 12.0 + 0.5) - 0.5) / 12.0;
        dist_v_min = abs(fract(v * 36.0 + 0.5) - 0.5) / 36.0;
    } else if (v_is_spl) {
        // SPL axis: 10dB major, 2dB minor (5 major / 25 minor for 50dB range)
        dist_v_maj = abs(fract(v * 5.0 + 0.5) - 0.5) / 5.0;
        dist_v_min = abs(fract(v * 25.0 + 0.5) - 0.5) / 25.0;
    }

    // Thin continuous lines with proper antialiasing
    // Use smoothstep with a tight 1-pixel transition centered at half_pixel
    // This avoids the dashed appearance from hard thresholds while keeping lines thin
    let half_pixel_u = du * 0.5;
    let half_pixel_v = dv * 0.5;

    // Feather is the AA transition width - keep it minimal but nonzero
    let feather_u = du * 0.5;
    let feather_v = dv * 0.5;

    // Compute border alpha (edge lines)
    var border_u_alpha = 0.0;
    if (draw_u_border) {
        border_u_alpha = 1.0 - smoothstep(half_pixel_u - feather_u, half_pixel_u + feather_u, dist_u_border);
    }

    var border_v_alpha = 0.0;
    if (draw_v_border) {
        border_v_alpha = 1.0 - smoothstep(half_pixel_v - feather_v, half_pixel_v + feather_v, dist_v_border);
    }

    let border_alpha = max(border_u_alpha, border_v_alpha);

    // Compute major grid line alpha
    var maj_u_alpha = 0.0;
    var maj_v_alpha = 0.0;
    if (draw_u_grid) {
        maj_u_alpha = 1.0 - smoothstep(half_pixel_u - feather_u, half_pixel_u + feather_u, dist_u_maj);
    }
    if (draw_v_grid) {
        maj_v_alpha = 1.0 - smoothstep(half_pixel_v - feather_v, half_pixel_v + feather_v, dist_v_maj);
    }
    let maj_alpha_val = max(maj_u_alpha, maj_v_alpha);

    // Compute minor grid line alpha
    var min_u_alpha = 0.0;
    var min_v_alpha = 0.0;
    if (draw_u_grid) {
        min_u_alpha = 1.0 - smoothstep(half_pixel_u - feather_u, half_pixel_u + feather_u, dist_u_min);
    }
    if (draw_v_grid) {
        min_v_alpha = 1.0 - smoothstep(half_pixel_v - feather_v, half_pixel_v + feather_v, dist_v_min);
    }
    let min_alpha_val = max(min_u_alpha, min_v_alpha);

    // Colors (same line width, different colors)
    let border_color = vec3<f32>(0.0, 0.0, 0.0);      // Black for edge lines
    let major_color = vec3<f32>(0.4, 0.4, 0.4);       // Dark grey for major ticks
    let minor_color = vec3<f32>(0.75, 0.75, 0.75);    // Light grey for minor ticks

    // Combine layers: border > major > minor
    // All lines have same width, just different colors
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if (border_alpha > 0.01) {
        color = vec4<f32>(border_color, border_alpha);
    } else if (maj_alpha_val > 0.01) {
        color = vec4<f32>(major_color, maj_alpha_val);
    } else if (min_alpha_val > 0.01) {
        color = vec4<f32>(minor_color, min_alpha_val);
    } else {
        discard;
    }

    return color;
}
"#;

/// Combined shader source
pub fn combined_shader() -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        COMMON_DEFINITIONS,
        SURFACE_VERTEX_SHADER,
        SURFACE_FRAGMENT_SHADER,
        WIREFRAME_FRAGMENT_SHADER,
        PROJECTION_VERTEX_SHADER,
        PROJECTION_FRAGMENT_SHADER,
        GRID_FRAGMENT_SHADER
    )
}
