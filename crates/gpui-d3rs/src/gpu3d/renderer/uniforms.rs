use super::super::camera::Camera3D;
use super::super::config::Surface3DConfig;
use super::Uniforms;
use glam::Mat4;

impl Uniforms {
    pub(super) fn new(
        camera: &Camera3D,
        config: &Surface3DConfig,
        log_settings: Option<(f32, f32)>,
    ) -> Self {
        let view_proj = camera.view_projection_matrix();
        let model = Mat4::IDENTITY;
        let light_dir = config.normalized_light_direction();

        let (is_log, min_log, range_log) = if let Some((min, max)) = log_settings {
            // min and max are already in linear space (e.g. 20 and 20000)
            // We need their logs.
            // Avoid log(0).
            let min_v = min.max(1e-10).log10();
            let max_v = max.max(1e-10).log10();
            let range = (max_v - min_v).max(1e-10);
            (1.0, min_v, range)
        } else {
            (0.0, 0.0, 1.0)
        };

        Self {
            view_proj: view_proj.to_cols_array_2d(),
            model: model.to_cols_array_2d(),
            light_dir: light_dir.to_array(),
            colormap: config.colormap.shader_index() as f32,
            ambient: config.ambient,
            diffuse: config.diffuse,
            opacity: config.opacity,
            z_min: 0.0,
            x_min_log: min_log,
            x_range_log: range_log,
            is_log_x: is_log,
            show_surface_isolines: if config.isolines { 1.0 } else { 0.0 },
            isoline_step: config.isoline_step.max(0.001),
            isoline_width_px: config.isoline_width_px.max(0.25),
            isoline_opacity: config.isoline_opacity.clamp(0.0, 1.0),
            isoline_color_r: config.isoline_color[0],
            isoline_color_g: config.isoline_color[1],
            isoline_color_b: config.isoline_color[2],
            _padding0: 0.0,
            _padding1: 0.0,
        }
    }
}
