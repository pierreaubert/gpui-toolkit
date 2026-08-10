use crate::cache::MeshPlotCacheUpdate;
use crate::cache::RetainedSceneCache;
use crate::error::Scene3DError;
use crate::meshplot::MeshPlotSpec;
use crate::scene3d::{
    CameraSpec, ColorRgba, ColormapSpec, LightSpec, LinesSpec, MeshSpec, OrbitCameraSpec, Point3,
    ScalarAssociation, SceneNode, SceneSpec, SurfaceSpec,
};
use d3rs::gpu3d::{
    Colormap, Line3D, Lines3DElement, Lines3DScene, Lines3DState, Polygon3D, Surface3DConfig,
    Surface3DElement, Surface3DState, SurfaceData,
};
use glam::Vec3;
use gpui::Rgba;
use std::cell::RefCell;
use std::collections::{HashMap, hash_map::Entry};
use std::rc::Rc;

#[derive(Default)]
pub struct Gpui3DCache {
    resources: RetainedSceneCache,
    surfaces: HashMap<String, Surface3DElement>,
    line_states: HashMap<String, Rc<RefCell<Lines3DState>>>,
    lines: HashMap<String, Lines3DElement>,
    meshes: HashMap<String, Lines3DElement>,
    mesh_states: HashMap<String, Rc<RefCell<Lines3DState>>>,
    scene_states: HashMap<String, Rc<RefCell<Lines3DState>>>,
    scenes: HashMap<String, Lines3DElement>,
}

/// Host-side retained state for declarative mesh plots.
///
/// The typed spec is replaced only after validation and the cache reports
/// independent geometry/field/style/camera dirty domains to the renderer.
#[derive(Debug, Default)]
pub struct GpuiMeshPlotCache {
    resources: RetainedSceneCache,
    specs: HashMap<String, MeshPlotSpec>,
}

impl GpuiMeshPlotCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, spec: MeshPlotSpec) -> Result<MeshPlotCacheUpdate, String> {
        let update = self.resources.upsert_meshplot(&spec)?;
        let id = update.id.clone();
        self.specs.insert(id, spec);
        Ok(update)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&MeshPlotSpec> {
        self.specs.get(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    pub fn retain_only<'a>(&mut self, ids: impl IntoIterator<Item = &'a str>) {
        let live = ids.into_iter().collect::<std::collections::HashSet<_>>();
        self.specs.retain(|id, _| live.contains(id.as_str()));
        self.resources.retain_only(live.into_iter());
    }
}

impl std::fmt::Debug for Gpui3DCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gpui3DCache")
            .field("resources", &self.resources)
            .field("surface_count", &self.surfaces.len())
            .field("line_state_count", &self.line_states.len())
            .field("line_count", &self.lines.len())
            .field("mesh_count", &self.meshes.len())
            .field("mesh_state_count", &self.mesh_states.len())
            .field("scene_count", &self.scenes.len())
            .finish()
    }
}

impl Gpui3DCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn surface_element(
        &mut self,
        spec: &SurfaceSpec,
    ) -> Result<Surface3DElement, Scene3DError> {
        let update = self.resources.upsert_surface(spec)?;

        if let Some(element) = self.surfaces.get_mut(&spec.id) {
            if update.dirty.updates_geometry() {
                element.set_data(surface_data(spec));
            }
            if update.dirty.updates_material() {
                element.set_config(surface_config(spec)?);
            }
            if update.dirty.updates_camera() {
                *element.state().borrow_mut() = surface_state(spec)?;
            }
            return Ok(element.clone());
        }

        let element = Surface3DElement::new(surface_data(spec), surface_config(spec)?);
        self.surfaces.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    pub fn lines_element(&mut self, spec: &LinesSpec) -> Result<Lines3DElement, Scene3DError> {
        let update = self.resources.upsert_lines(spec)?;

        if update.dirty.is_unchanged()
            && let Some(element) = self.lines.get(&spec.id)
        {
            return Ok(element.clone());
        }

        let state = match self.line_states.entry(spec.id.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry
                .insert(Rc::new(RefCell::new(lines_state(spec)?)))
                .clone(),
        };

        if update.dirty.updates_camera() {
            *state.borrow_mut() = lines_state(spec)?;
        }

        let scene = lines_scene(spec);
        let element = Lines3DElement::new(state, scene);
        self.lines.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    /// Build a retained mesh element using the same camera and GPUI path
    /// renderer as sparse 3D lines. Mesh vertices are normalized to a unit
    /// cube, then emitted as filled triangle polygons with the requested
    /// material. This is a real renderer path rather than a validation-only
    /// summary, while keeping dense GPU surface rendering separate.
    pub fn mesh_element(&mut self, spec: &MeshSpec) -> Result<Lines3DElement, Scene3DError> {
        let update = self.resources.upsert_mesh(spec)?;
        if update.dirty.is_unchanged()
            && let Some(element) = self.meshes.get(&spec.id)
        {
            return Ok(element.clone());
        }

        let (min, max) = mesh_bounds(spec);
        let center = (min + max) * 0.5;
        let extent = (max - min).max_element().max(f32::EPSILON);
        let scale = 2.0 / extent;
        let normalized = |point: Point3| (vec3(point) - center) * scale;
        let polygons = spec
            .indices
            .chunks_exact(3)
            .enumerate()
            .map(|(triangle_index, triangle)| Polygon3D {
                vertices: triangle
                    .iter()
                    .map(|&index| normalized(spec.vertices[index as usize]))
                    .collect(),
                fill: Some(mesh_triangle_fill(spec, triangle, triangle_index)),
                stroke: None,
            })
            .collect();
        let state = match self.mesh_states.entry(spec.id.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry
                .insert(Rc::new(RefCell::new(Lines3DState::default())))
                .clone(),
        };
        let element = Lines3DElement::new(
            state,
            Lines3DScene {
                background: None,
                lines: Vec::new(),
                polygons,
            },
        );
        self.meshes.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    /// Render a composed scene containing lines and indexed meshes through one
    /// retained orbit state. Geometry/material/camera fingerprints are tracked
    /// independently, so camera-only patches preserve the element and its GPU
    /// scene while geometry/material patches rebuild only the draw data.
    pub fn scene_element(&mut self, spec: &SceneSpec) -> Result<Lines3DElement, Scene3DError> {
        let update = self.resources.upsert_scene(spec)?;
        let state = match self.scene_states.entry(spec.id.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry
                .insert(Rc::new(RefCell::new(scene_state(spec)?)))
                .clone(),
        };
        if update.dirty.updates_camera() {
            *state.borrow_mut() = scene_state(spec)?;
        }
        if update.dirty.is_unchanged()
            && let Some(element) = self.scenes.get(&spec.id)
        {
            return Ok(element.clone());
        }

        let element = Lines3DElement::new(state, scene_scene(spec));
        self.scenes.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    pub fn retain_only<I, S>(&mut self, ids: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let live: std::collections::HashSet<String> =
            ids.into_iter().map(|id| id.as_ref().to_string()).collect();
        self.resources.retain_only(live.iter().map(String::as_str));
        self.surfaces.retain(|id, _| live.contains(id));
        self.line_states.retain(|id, _| live.contains(id));
        self.lines.retain(|id, _| live.contains(id));
        self.meshes.retain(|id, _| live.contains(id));
        self.mesh_states.retain(|id, _| live.contains(id));
        self.scene_states.retain(|id, _| live.contains(id));
        self.scenes.retain(|id, _| live.contains(id));
    }

    /// Shared state for a retained lines viewport. The host owns transient
    /// orbit/pan/zoom mutations, while Python remains authoritative for the
    /// declared camera used by an explicit patch or reset.
    pub fn lines_state(&self, id: &str) -> Option<Rc<RefCell<Lines3DState>>> {
        self.line_states.get(id).cloned()
    }

    /// Shared state for a composed retained scene viewport.
    pub fn scene_state(&self, id: &str) -> Option<Rc<RefCell<Lines3DState>>> {
        self.scene_states.get(id).cloned()
    }

    /// Shared state for an indexed mesh viewport.
    pub fn mesh_state(&self, id: &str) -> Option<Rc<RefCell<Lines3DState>>> {
        self.mesh_states.get(id).cloned()
    }
}

fn mesh_bounds(spec: &MeshSpec) -> (Vec3, Vec3) {
    let mut min = vec3(spec.vertices[0]);
    let mut max = min;
    for &vertex in &spec.vertices[1..] {
        let vertex = vec3(vertex);
        min = min.min(vertex);
        max = max.max(vertex);
    }
    (min, max)
}

fn surface_data(spec: &SurfaceSpec) -> SurfaceData {
    let (z_flat, z_width, z_height) = spec.z.as_flat();
    let mut data = SurfaceData::from_flat_grid(
        spec.x_values().into(),
        spec.y_values().into(),
        z_flat,
        z_width,
        z_height,
    )
    .with_log_x(spec.x_log)
    .with_log_y(spec.y_log)
    .with_log_z(spec.z_log);

    if let Some(label) = &spec.labels.x {
        data = data.with_x_label(label.clone());
    }
    if let Some(label) = &spec.labels.y {
        data = data.with_y_label(label.clone());
    }
    if let Some(label) = &spec.labels.z {
        data = data.with_z_label(label.clone());
    }
    if let Some(range) = spec.z_range {
        data = data.with_z_range(range.min, range.max);
    }

    data
}

fn surface_config(spec: &SurfaceSpec) -> Result<Surface3DConfig, Scene3DError> {
    let mut config = Surface3DConfig::new()
        .colormap(colormap(spec.colormap))
        .wireframe(spec.wireframe);

    if let Some(camera) = spec.camera.as_ref() {
        let orbit = camera_orbit(camera)?;
        config = config.camera_position(orbit.distance, orbit.azimuth_deg, orbit.elevation_deg);
    }

    Ok(config)
}

fn surface_state(spec: &SurfaceSpec) -> Result<Surface3DState, Scene3DError> {
    if let Some(camera) = spec.camera.as_ref() {
        let orbit = camera_orbit(camera)?;
        Ok(Surface3DState::new(
            orbit.distance,
            orbit.azimuth_deg,
            orbit.elevation_deg,
        ))
    } else {
        Ok(Surface3DState::default())
    }
}

fn lines_state(spec: &LinesSpec) -> Result<Lines3DState, Scene3DError> {
    if let Some(camera) = spec.camera.as_ref() {
        let orbit = camera_orbit(camera)?;
        Ok(Lines3DState::new(
            orbit.distance,
            orbit.azimuth_deg,
            orbit.elevation_deg,
        ))
    } else {
        Ok(Lines3DState::default())
    }
}

fn lines_scene(spec: &LinesSpec) -> Lines3DScene {
    Lines3DScene {
        background: spec.background.map(rgba),
        lines: spec
            .flattened_segments()
            .into_iter()
            .map(|segment| Line3D {
                from: vec3(segment.from),
                to: vec3(segment.to),
                color: rgba(segment.color),
                width: segment.width,
            })
            .collect(),
        polygons: Vec::new(),
    }
}

fn scene_state(spec: &SceneSpec) -> Result<Lines3DState, Scene3DError> {
    let orbit = camera_orbit(&spec.camera)?;
    Ok(Lines3DState::new(
        orbit.distance,
        orbit.azimuth_deg,
        orbit.elevation_deg,
    ))
}

fn scene_scene(spec: &SceneSpec) -> Lines3DScene {
    let points = spec
        .children
        .iter()
        .flat_map(|child| match child {
            SceneNode::Lines(lines) => lines
                .strips
                .iter()
                .flat_map(|strip| strip.points.iter().copied())
                .collect::<Vec<_>>(),
            SceneNode::Mesh(mesh) => mesh.vertices.clone(),
            SceneNode::Surface(surface) => surface_points(surface),
            SceneNode::Light(_) => Vec::new(),
        })
        .collect::<Vec<_>>();
    let (min, max) = points
        .first()
        .map(|point| {
            points
                .iter()
                .skip(1)
                .fold((vec3(*point), vec3(*point)), |(min, max), point| {
                    (min.min(vec3(*point)), max.max(vec3(*point)))
                })
        })
        .unwrap_or((Vec3::ZERO, Vec3::ONE));
    let center = (min + max) * 0.5;
    let scale = 2.0 / (max - min).max_element().max(f32::EPSILON);
    let normalize = |point: Point3| (vec3(point) - center) * scale;

    let mut lines = Vec::new();
    let mut polygons = Vec::new();
    let lights = spec
        .children
        .iter()
        .filter_map(|child| match child {
            SceneNode::Light(light) => Some(light),
            _ => None,
        })
        .collect::<Vec<_>>();
    for child in &spec.children {
        match child {
            SceneNode::Lines(spec) => {
                lines.extend(spec.flattened_segments().into_iter().map(|segment| Line3D {
                    from: normalize(segment.from),
                    to: normalize(segment.to),
                    color: rgba(segment.color),
                    width: segment.width,
                }))
            }
            SceneNode::Mesh(mesh) => {
                polygons.extend(mesh.indices.chunks_exact(3).enumerate().map(
                    |(triangle_index, triangle)| {
                        Polygon3D {
                            vertices: triangle
                                .iter()
                                .map(|&index| normalize(mesh.vertices[index as usize]))
                                .collect(),
                            fill: Some(scene_lit_color(
                                mesh_triangle_fill(mesh, triangle, triangle_index),
                                &triangle
                                    .iter()
                                    .map(|&index| mesh.vertices[index as usize])
                                    .collect::<Vec<_>>(),
                                &lights,
                            )),
                            stroke: None,
                        }
                    },
                ));
            }
            SceneNode::Surface(surface) => {
                polygons.extend(surface_polygons(surface, &normalize, &lights))
            }
            SceneNode::Light(_) => {}
        }
    }
    Lines3DScene {
        background: spec.background.map(rgba),
        lines,
        polygons,
    }
}

fn surface_points(spec: &SurfaceSpec) -> Vec<Point3> {
    let x = spec.x_values();
    let y = spec.y_values();
    let (z, width, height) = spec.z.as_flat();
    let mut points = Vec::with_capacity(width * height);
    for row in 0..height {
        for column in 0..width {
            points.push(Point3 {
                x: x[column] as f32,
                y: y[row] as f32,
                z: z[row * width + column] as f32,
            });
        }
    }
    points
}

fn surface_polygons(
    spec: &SurfaceSpec,
    normalize: &impl Fn(Point3) -> Vec3,
    lights: &[&LightSpec],
) -> Vec<Polygon3D> {
    let points = surface_points(spec);
    let (_, width, height) = spec.z.as_flat();
    let (minimum, maximum) = spec
        .z_range
        .map(|range| (range.min, range.max))
        .unwrap_or_else(|| {
            spec.z
                .values
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                    (min.min(*value), max.max(*value))
                })
        });
    let mut polygons = Vec::with_capacity(width.saturating_sub(1) * height.saturating_sub(1) * 2);
    for row in 0..height.saturating_sub(1) {
        for column in 0..width.saturating_sub(1) {
            let first = row * width + column;
            for indices in [
                [first, first + 1, first + width],
                [first + 1, first + width + 1, first + width],
            ] {
                let vertices = indices.map(|index| points[index]);
                let value = vertices.iter().map(|point| point.z as f64).sum::<f64>() / 3.0;
                let normalized_value = if maximum > minimum {
                    ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0) as f32
                } else {
                    0.5
                };
                let color = scene_lit_color(
                    scalar_color(spec.colormap, normalized_value),
                    &vertices,
                    lights,
                );
                polygons.push(Polygon3D {
                    vertices: vertices.into_iter().map(normalize).collect(),
                    fill: (!spec.wireframe).then_some(color),
                    stroke: spec.wireframe.then_some((color, 1.0)),
                });
            }
        }
    }
    polygons
}

fn scene_lit_color(mut color: Rgba, vertices: &[Point3], lights: &[&LightSpec]) -> Rgba {
    if lights.is_empty() || vertices.len() < 3 {
        return color;
    }
    let a = vec3(vertices[0]);
    let normal = (vec3(vertices[1]) - a)
        .cross(vec3(vertices[2]) - a)
        .normalize_or_zero();
    let illumination = lights
        .iter()
        .fold(0.2, |total, light| {
            let direction = vec3(light.direction).normalize_or_zero();
            total + normal.dot(-direction).max(0.0) * light.intensity
        })
        .min(1.5);
    color.r = (color.r * illumination).min(1.0);
    color.g = (color.g * illumination).min(1.0);
    color.b = (color.b * illumination).min(1.0);
    color
}

fn mesh_triangle_fill(mesh: &MeshSpec, triangle: &[u32], triangle_index: usize) -> Rgba {
    let opacity = mesh.material.color.a * mesh.material.opacity;
    let Some(field) = &mesh.scalar_field else {
        return Rgba {
            r: mesh.material.color.r,
            g: mesh.material.color.g,
            b: mesh.material.color.b,
            a: opacity,
        };
    };
    let value = match field.association {
        ScalarAssociation::Vertex => {
            triangle
                .iter()
                .map(|index| field.values[*index as usize])
                .sum::<f64>()
                / triangle.len() as f64
        }
        ScalarAssociation::Cell => field.values[triangle_index],
    };
    let (min, max) = field
        .range
        .map(|range| (range.min, range.max))
        .unwrap_or_else(|| {
            field
                .values
                .iter()
                .copied()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |range, value| {
                    (range.0.min(value), range.1.max(value))
                })
        });
    let normalized = (if max > min {
        ((value - min) / (max - min)).clamp(0.0, 1.0)
    } else {
        0.5
    }) as f32;
    let mut color = scalar_color(field.colormap, normalized);
    color.a *= opacity;
    color
}

fn scalar_color(colormap: ColormapSpec, t: f32) -> Rgba {
    let lerp = |a: (f32, f32, f32), b: (f32, f32, f32), t: f32| Rgba {
        r: a.0 + (b.0 - a.0) * t,
        g: a.1 + (b.1 - a.1) * t,
        b: a.2 + (b.2 - a.2) * t,
        a: 1.0,
    };
    match colormap {
        ColormapSpec::Viridis => lerp((0.267, 0.005, 0.329), (0.993, 0.906, 0.144), t),
        ColormapSpec::Plasma => lerp((0.050, 0.030, 0.528), (0.940, 0.975, 0.131), t),
        ColormapSpec::Inferno => lerp((0.001, 0.000, 0.014), (0.988, 0.998, 0.645), t),
        ColormapSpec::Turbo => lerp((0.190, 0.071, 0.232), (0.479, 0.016, 0.010), t),
        ColormapSpec::CoolWarm => lerp((0.230, 0.299, 0.754), (0.706, 0.016, 0.150), t),
    }
}

fn camera_orbit(camera: &CameraSpec) -> Result<&OrbitCameraSpec, Scene3DError> {
    camera.as_orbit().ok_or(Scene3DError::UnsupportedNode {
        kind: "perspective_camera",
    })
}

fn colormap(value: ColormapSpec) -> Colormap {
    match value {
        ColormapSpec::Viridis => Colormap::Viridis,
        ColormapSpec::Plasma => Colormap::Plasma,
        ColormapSpec::Inferno => Colormap::Inferno,
        ColormapSpec::Turbo => Colormap::Turbo,
        ColormapSpec::CoolWarm => Colormap::CoolWarm,
    }
}

fn vec3(point: Point3) -> Vec3 {
    Vec3::new(point.x, point.y, point.z)
}

fn rgba(color: ColorRgba) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene3d::{CameraSpec, LineStripSpec, PerspectiveCameraSpec};

    #[test]
    fn retained_surface_cache_reuses_element_state() {
        let mut cache = Gpui3DCache::new();
        let mut spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        spec.camera = Some(CameraSpec::Orbit(OrbitCameraSpec::new(4.0, 60.0, 25.0)));

        let first = cache.surface_element(&spec).expect("first surface");
        first.state().borrow_mut().controls.azimuth = 0.123;
        let second = cache.surface_element(&spec).expect("same surface");

        assert_eq!(Rc::as_ptr(&first.state()), Rc::as_ptr(&second.state()));
        assert!((second.state().borrow().controls.azimuth - 0.123).abs() < f32::EPSILON);
    }

    #[test]
    fn lines_adapter_builds_segments_from_strip() {
        let mut cache = Gpui3DCache::new();
        let spec = LinesSpec {
            id: "lines".to_string(),
            strips: vec![LineStripSpec {
                id: "path".to_string(),
                points: vec![
                    Point3::new(0.0, 0.0, 0.0),
                    Point3::new(1.0, 0.0, 0.0),
                    Point3::new(1.0, 1.0, 0.0),
                ],
                color: ColorRgba::from_rgb_u8(255, 255, 255),
                width: 1.0,
            }],
            ..LinesSpec::default()
        };

        let _element = cache.lines_element(&spec).expect("lines element");
    }

    #[test]
    fn orbit_adapters_reject_future_perspective_camera_without_panicking() {
        let mut cache = Gpui3DCache::new();
        let spec = LinesSpec {
            id: "lines".to_string(),
            strips: vec![LineStripSpec {
                id: "path".to_string(),
                points: vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
                color: ColorRgba::from_rgb_u8(255, 255, 255),
                width: 1.0,
            }],
            camera: Some(CameraSpec::Perspective(PerspectiveCameraSpec::default())),
            ..LinesSpec::default()
        };

        assert!(matches!(
            cache.lines_element(&spec),
            Err(Scene3DError::UnsupportedNode {
                kind: "perspective_camera"
            })
        ));
    }

    #[test]
    fn lines_element_is_cached_and_reused_when_unchanged() {
        let mut cache = Gpui3DCache::new();
        let spec = LinesSpec {
            id: "lines".to_string(),
            strips: vec![LineStripSpec {
                id: "path".to_string(),
                points: vec![
                    Point3::new(0.0, 0.0, 0.0),
                    Point3::new(1.0, 0.0, 0.0),
                    Point3::new(1.0, 1.0, 0.0),
                ],
                color: ColorRgba::from_rgb_u8(255, 255, 255),
                width: 1.0,
            }],
            ..LinesSpec::default()
        };

        let _first = cache.lines_element(&spec).expect("first lines");
        let _second = cache.lines_element(&spec).expect("second lines");

        assert_eq!(cache.lines.len(), 1);
        assert_eq!(cache.line_states.len(), 1);
    }

    #[test]
    fn mesh_adapter_builds_and_caches_rendering_element() {
        let mut cache = Gpui3DCache::new();
        let spec = MeshSpec {
            id: "mesh".to_string(),
            vertices: vec![
                Point3::new(-1.0, -1.0, 0.0),
                Point3::new(1.0, -1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2],
            material: crate::scene3d::MaterialSpec::default(),
            scalar_field: None,
        };

        let _first = cache.mesh_element(&spec).expect("first mesh");
        let _second = cache.mesh_element(&spec).expect("cached mesh");
        assert_eq!(cache.meshes.len(), 1);
        let first_state = cache.mesh_state("mesh").expect("retained mesh state");
        first_state.borrow_mut().controls.azimuth = 0.42;
        let _third = cache.mesh_element(&spec).expect("same mesh state");
        assert!(
            (cache.mesh_state("mesh").unwrap().borrow().controls.azimuth - 0.42).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn composed_scene_renders_meshes_and_retains_camera_state() {
        let mut cache = Gpui3DCache::new();
        let mut scene = crate::scene3d::SceneSpec {
            id: "scene".into(),
            camera: CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 45.0, 25.0)),
            children: vec![crate::scene3d::SceneNode::Mesh(MeshSpec {
                id: "speaker".into(),
                vertices: vec![
                    Point3::new(-1.0, -1.0, 0.0),
                    Point3::new(1.0, -1.0, 0.0),
                    Point3::new(0.0, 1.0, 0.0),
                ],
                indices: vec![0, 1, 2],
                material: crate::scene3d::MaterialSpec::default(),
                scalar_field: None,
            })],
            interactions: Vec::new(),
            background: None,
            size: None,
        };
        let _first = cache.scene_element(&scene).expect("first scene");
        let first_state = cache.scene_states.get("scene").unwrap().clone();
        first_state.borrow_mut().controls.azimuth = 0.2;
        scene.camera = CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 90.0, 25.0));
        let _second = cache.scene_element(&scene).expect("camera update");
        let second_state = cache.scene_states.get("scene").unwrap().clone();
        assert_eq!(Rc::as_ptr(&first_state), Rc::as_ptr(&second_state));
        assert!((second_state.borrow().controls.azimuth - 90.0_f32.to_radians()).abs() < 1e-4);
        assert_eq!(cache.scenes.len(), 1);
    }

    #[test]
    fn composed_scene_includes_surface_triangles_and_lighting() {
        let scene = crate::scene3d::SceneSpec {
            id: "scene".into(),
            camera: CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 45.0, 25.0)),
            children: vec![
                crate::scene3d::SceneNode::Surface(SurfaceSpec::from_flat(
                    "field",
                    vec![0.0, 1.0, 2.0, 3.0],
                    2,
                    2,
                )),
                crate::scene3d::SceneNode::Light(LightSpec {
                    id: "key".into(),
                    direction: Point3::new(0.0, 0.0, -1.0),
                    intensity: 0.8,
                    color: ColorRgba::default(),
                }),
            ],
            interactions: Vec::new(),
            background: None,
            size: None,
        };
        let rendered = scene_scene(&scene);
        assert_eq!(rendered.polygons.len(), 2);
        assert!(
            rendered
                .polygons
                .iter()
                .all(|polygon| polygon.fill.is_some())
        );
    }

    #[test]
    fn surface_camera_change_updates_state_only() {
        let mut cache = Gpui3DCache::new();
        let mut spec = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        spec.camera = Some(CameraSpec::Orbit(OrbitCameraSpec::new(4.0, 60.0, 25.0)));

        let first = cache.surface_element(&spec).expect("first surface");
        first.state().borrow_mut().controls.azimuth = 0.111;

        spec.camera = Some(CameraSpec::Orbit(OrbitCameraSpec::new(4.0, 90.0, 25.0)));
        let second = cache.surface_element(&spec).expect("camera-only update");

        assert_eq!(Rc::as_ptr(&first.state()), Rc::as_ptr(&second.state()));
        assert!((second.state().borrow().controls.azimuth - 90.0_f32.to_radians()).abs() < 1e-4);
    }

    #[test]
    fn retain_only_drops_orphaned_resources() {
        let mut cache = Gpui3DCache::new();
        let surface = SurfaceSpec::from_flat("surface", vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        let lines = LinesSpec {
            id: "lines".to_string(),
            strips: vec![LineStripSpec {
                id: "path".to_string(),
                points: vec![Point3::new(0.0, 0.0, 0.0), Point3::new(1.0, 0.0, 0.0)],
                color: ColorRgba::from_rgb_u8(255, 255, 255),
                width: 1.0,
            }],
            ..LinesSpec::default()
        };

        cache.surface_element(&surface).expect("surface");
        cache.lines_element(&lines).expect("lines");
        assert_eq!(cache.resources.len(), 2);

        cache.retain_only(["surface"]);

        assert_eq!(cache.resources.len(), 1);
        assert!(cache.surfaces.contains_key("surface"));
        assert!(cache.lines.is_empty());
        assert!(cache.line_states.is_empty());
    }
}
