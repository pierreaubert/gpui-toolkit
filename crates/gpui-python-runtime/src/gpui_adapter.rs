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
use d3rs::mesh::MeshUpload;
use d3rs::mesh::gpu::{
    FieldRevision, GeometryRevision, MeshColorConfig, MeshSceneElement, MeshSceneState,
    WgpuMesh3DRenderer,
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
    meshes: HashMap<String, MeshSceneElement>,
    mesh_states: HashMap<String, Rc<RefCell<Lines3DState>>>,
    scene_states: HashMap<String, Rc<RefCell<Lines3DState>>>,
    scenes: HashMap<String, MeshSceneElement>,
    gpu_states: HashMap<String, Rc<RefCell<MeshSceneState>>>,
    gpu_renderers: HashMap<String, Rc<WgpuMesh3DRenderer>>,
}

/// Host-side retained state for declarative mesh plots.
///
/// The typed spec is replaced only after validation and the cache reports
/// independent geometry/field/style/camera dirty domains to the renderer.
#[derive(Debug, Default)]
pub struct GpuiMeshPlotCache {
    resources: RetainedSceneCache,
    specs: HashMap<String, MeshPlotSpec>,
    revisions: HashMap<String, u64>,
}

impl GpuiMeshPlotCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, spec: MeshPlotSpec) -> Result<MeshPlotCacheUpdate, String> {
        let requested_id = spec.cache_id();
        if self
            .revisions
            .get(&requested_id)
            .is_some_and(|current| spec.revision < *current)
        {
            return Err(format!(
                "stale mesh_plot revision {} for {requested_id}; current revision is {}",
                spec.revision, self.revisions[&requested_id]
            ));
        }
        let update = self.resources.upsert_meshplot(&spec)?;
        let id = update.id.clone();
        self.revisions.insert(id.clone(), spec.revision);
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
        self.revisions.retain(|id, _| live.contains(id.as_str()));
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

    /// Build a retained indexed mesh GPU draw while retaining the same orbit
    /// state used by sparse 3D lines. Material colors are passed directly to
    /// the WGPU vertex stream instead of expanding triangles into CPU paths.
    pub fn mesh_element(&mut self, spec: &MeshSpec) -> Result<MeshSceneElement, Scene3DError> {
        let update = self.resources.upsert_mesh(spec)?;
        if update.dirty.is_unchanged()
            && let Some(element) = self.meshes.get(&spec.id)
        {
            if let (Some(renderer), Some(state)) = (
                self.gpu_renderers.get(&spec.id),
                self.mesh_states.get(&spec.id),
            ) {
                renderer.set_camera(&state.borrow().camera);
            }
            return Ok(element.clone());
        }

        let state = match self.mesh_states.entry(spec.id.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry
                .insert(Rc::new(RefCell::new(Lines3DState::default())))
                .clone(),
        };
        let (upload, vertex_colors) = mesh_gpu_upload(spec);
        let element = self.gpu_element(&spec.id, state, upload, vertex_colors, false);
        self.meshes.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    /// Render a composed scene containing lines and indexed meshes through one
    /// retained orbit state. Geometry/material/camera fingerprints are tracked
    /// independently, so camera-only patches preserve the element and its GPU
    /// scene while geometry/material patches rebuild only the draw data.
    pub fn scene_element(&mut self, spec: &SceneSpec) -> Result<MeshSceneElement, Scene3DError> {
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
            if let Some(renderer) = self.gpu_renderers.get(&spec.id) {
                renderer.set_camera(&state.borrow().camera);
            }
            return Ok(element.clone());
        }

        let (upload, vertex_colors, wireframe) = scene_gpu_upload(spec);
        let element = self.gpu_element(&spec.id, state, upload, vertex_colors, wireframe);
        self.scenes.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    fn gpu_element(
        &mut self,
        id: &str,
        orbit_state: Rc<RefCell<Lines3DState>>,
        upload: MeshUpload,
        vertex_colors: Vec<[f32; 4]>,
        wireframe: bool,
    ) -> MeshSceneElement {
        let state = self
            .gpu_states
            .entry(id.to_string())
            .or_insert_with(|| {
                Rc::new(RefCell::new(MeshSceneState {
                    geometry_rev: GeometryRevision(0),
                    field_rev: FieldRevision(0),
                    ..MeshSceneState::default()
                }))
            })
            .clone();

        let changed = {
            let current = state.borrow();
            current.upload.as_ref() != Some(&upload)
                || current.vertex_colors.as_deref() != Some(vertex_colors.as_slice())
        };
        if changed {
            let mut current = state.borrow_mut();
            current.geometry_rev = GeometryRevision(current.geometry_rev.0.saturating_add(1));
            current.field_rev = FieldRevision(current.field_rev.0.saturating_add(1));
            current.upload = Some(upload);
            current.vertex_colors = Some(vertex_colors);
            current.color = MeshColorConfig {
                // Direct RGBA material colors are already lit by the typed
                // scene adapter, so every child can share one depth pass.
                unlit: true,
                wireframe,
                ..MeshColorConfig::default()
            };
        }

        let renderer = self
            .gpu_renderers
            .entry(id.to_string())
            .or_insert_with(|| {
                Rc::new(WgpuMesh3DRenderer::new_with_camera(
                    state.clone(),
                    Rc::new(RefCell::new(orbit_state.borrow().camera.clone())),
                ))
            })
            .clone();
        renderer.set_camera(&orbit_state.borrow().camera);
        MeshSceneElement::new(state).with_custom_id(renderer.custom_id())
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
        self.gpu_states.retain(|id, _| live.contains(id));
        self.gpu_renderers.retain(|id, _| live.contains(id));
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

fn push_colored_triangle(
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    indices: &mut Vec<u32>,
    vertices: [Point3; 3],
    color: Rgba,
    normalize: impl Fn(Point3) -> Vec3,
) {
    let base = positions.len() as u32;
    positions.extend(
        vertices
            .into_iter()
            .map(|point| normalize(point).to_array()),
    );
    colors.extend(std::iter::repeat_n([color.r, color.g, color.b, color.a], 3));
    indices.extend([base, base + 1, base + 2]);
}

fn push_colored_line(
    positions: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
    edges: &mut Vec<u32>,
    from: Point3,
    to: Point3,
    color: Rgba,
    normalize: impl Fn(Point3) -> Vec3,
) {
    let base = positions.len() as u32;
    positions.extend([normalize(from).to_array(), normalize(to).to_array()]);
    let color = [color.r, color.g, color.b, color.a];
    colors.extend([color, color]);
    edges.extend([base, base + 1]);
}

fn mesh_gpu_upload(spec: &MeshSpec) -> (MeshUpload, Vec<[f32; 4]>) {
    let (min, max) = mesh_bounds(spec);
    let center = (min + max) * 0.5;
    let scale = 2.0 / (max - min).max_element().max(f32::EPSILON);
    let normalize = |point: Point3| (vec3(point) - center) * scale;

    // Cell-associated values need a distinct flat color for every triangle,
    // so their expanded representation is intentional. Uniform and
    // vertex-associated meshes can keep their shared vertices and original
    // topology: the custom draw indexes the matching per-vertex color buffer.
    if !matches!(
        spec.scalar_field.as_ref().map(|field| field.association),
        Some(ScalarAssociation::Cell)
    ) {
        let positions = spec
            .vertices
            .iter()
            .copied()
            .map(|point| normalize(point).to_array())
            .collect();
        let colors = (0..spec.vertices.len())
            .map(|index| {
                let color = mesh_triangle_fill(spec, &[index as u32], 0);
                [color.r, color.g, color.b, color.a]
            })
            .collect();
        return (
            MeshUpload {
                positions_f32: positions,
                origin: [0.0; 3],
                indices: spec.indices.clone(),
                edge_indices: Vec::new(),
                values_f32: None,
                cell_values_f32: None,
            },
            colors,
        );
    }

    let mut positions = Vec::with_capacity(spec.indices.len());
    let mut colors = Vec::with_capacity(spec.indices.len());
    let mut indices = Vec::with_capacity(spec.indices.len());
    for (triangle_index, triangle) in spec.indices.chunks_exact(3).enumerate() {
        let vertices = [
            spec.vertices[triangle[0] as usize],
            spec.vertices[triangle[1] as usize],
            spec.vertices[triangle[2] as usize],
        ];
        push_colored_triangle(
            &mut positions,
            &mut colors,
            &mut indices,
            vertices,
            mesh_triangle_fill(spec, triangle, triangle_index),
            normalize,
        );
    }
    (
        MeshUpload {
            positions_f32: positions,
            origin: [0.0; 3],
            indices,
            edge_indices: Vec::new(),
            values_f32: None,
            cell_values_f32: None,
        },
        colors,
    )
}

fn scene_gpu_upload(spec: &SceneSpec) -> (MeshUpload, Vec<[f32; 4]>, bool) {
    let (min, max) = scene_bounds(spec);
    let center = (min + max) * 0.5;
    let scale = 2.0 / (max - min).max_element().max(f32::EPSILON);
    let normalize = |point: Point3| (vec3(point) - center) * scale;
    let lights = spec
        .children
        .iter()
        .filter_map(|child| match child {
            SceneNode::Light(light) => Some(light),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut edges = Vec::new();

    for child in &spec.children {
        match child {
            SceneNode::Lines(lines) => {
                for segment in lines.flattened_segments() {
                    push_colored_line(
                        &mut positions,
                        &mut colors,
                        &mut edges,
                        segment.from,
                        segment.to,
                        rgba(segment.color),
                        normalize,
                    );
                }
            }
            SceneNode::Mesh(mesh) => {
                // Lighting is evaluated per face, which requires isolated
                // vertices. Without lights, uniform and vertex-associated
                // meshes can share their source vertices and index buffer.
                if lights.is_empty()
                    && !matches!(
                        mesh.scalar_field.as_ref().map(|field| field.association),
                        Some(ScalarAssociation::Cell)
                    )
                {
                    let base = positions.len() as u32;
                    positions.extend(
                        mesh.vertices
                            .iter()
                            .copied()
                            .map(|point| normalize(point).to_array()),
                    );
                    colors.extend((0..mesh.vertices.len()).map(|index| {
                        let color = mesh_triangle_fill(mesh, &[index as u32], 0);
                        [color.r, color.g, color.b, color.a]
                    }));
                    indices.extend(mesh.indices.iter().map(|index| base + *index));
                    continue;
                }

                for (triangle_index, triangle) in mesh.indices.chunks_exact(3).enumerate() {
                    let vertices = [
                        mesh.vertices[triangle[0] as usize],
                        mesh.vertices[triangle[1] as usize],
                        mesh.vertices[triangle[2] as usize],
                    ];
                    let color = scene_lit_color(
                        mesh_triangle_fill(mesh, triangle, triangle_index),
                        &vertices,
                        &lights,
                    );
                    push_colored_triangle(
                        &mut positions,
                        &mut colors,
                        &mut indices,
                        vertices,
                        color,
                        normalize,
                    );
                }
            }
            SceneNode::Surface(surface) => {
                let xs = surface.x_values();
                let ys = surface.y_values();
                let (z_flat, width, height) = surface.z.as_flat();
                let (minimum, maximum) = surface.z_range.map_or_else(
                    || {
                        surface
                            .z
                            .values
                            .iter()
                            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                                (min.min(*value), max.max(*value))
                            })
                    },
                    |range| (range.min, range.max),
                );
                let mut wire_edges = std::collections::HashSet::new();

                for row in 0..height.saturating_sub(1) {
                    for column in 0..width.saturating_sub(1) {
                        let first = row * width + column;
                        for cell in [
                            [first, first + 1, first + width],
                            [first + 1, first + width + 1, first + width],
                        ] {
                            let vertices = cell.map(|index| {
                                let row = index / width;
                                let column = index % width;
                                Point3::new(xs[column] as f32, ys[row] as f32, z_flat[index] as f32)
                            });
                            let value =
                                vertices.iter().map(|point| point.z as f64).sum::<f64>() / 3.0;
                            let normalized_value = if maximum > minimum {
                                ((value - minimum) / (maximum - minimum)).clamp(0.0, 1.0) as f32
                            } else {
                                0.5
                            };
                            let color = scene_lit_color(
                                scalar_color(surface.colormap, normalized_value),
                                &vertices,
                                &lights,
                            );
                            if surface.wireframe {
                                for (from_index, to_index, from, to) in [
                                    (cell[0], cell[1], vertices[0], vertices[1]),
                                    (cell[1], cell[2], vertices[1], vertices[2]),
                                    (cell[2], cell[0], vertices[2], vertices[0]),
                                ] {
                                    let edge = if from_index < to_index {
                                        (from_index, to_index)
                                    } else {
                                        (to_index, from_index)
                                    };
                                    if wire_edges.insert(edge) {
                                        push_colored_line(
                                            &mut positions,
                                            &mut colors,
                                            &mut edges,
                                            from,
                                            to,
                                            color,
                                            normalize,
                                        );
                                    }
                                }
                            } else {
                                push_colored_triangle(
                                    &mut positions,
                                    &mut colors,
                                    &mut indices,
                                    vertices,
                                    color,
                                    normalize,
                                );
                            }
                        }
                    }
                }
            }
            SceneNode::Light(_) => {}
        }
    }

    let wireframe = !edges.is_empty();
    (
        MeshUpload {
            positions_f32: positions,
            origin: [0.0; 3],
            indices,
            edge_indices: edges,
            values_f32: None,
            cell_values_f32: None,
        },
        colors,
        wireframe,
    )
}

fn scene_bounds(spec: &SceneSpec) -> (Vec3, Vec3) {
    let mut bounds: Option<(Vec3, Vec3)> = None;
    let mut include = |point: Point3| {
        let point = vec3(point);
        bounds = Some(match bounds {
            Some((min, max)) => (min.min(point), max.max(point)),
            None => (point, point),
        });
    };
    for child in &spec.children {
        match child {
            SceneNode::Lines(lines) => {
                for segment in lines.flattened_segments() {
                    include(segment.from);
                    include(segment.to);
                }
            }
            SceneNode::Mesh(mesh) => mesh.vertices.iter().copied().for_each(&mut include),
            SceneNode::Surface(surface) => {
                surface_points(surface).into_iter().for_each(&mut include)
            }
            SceneNode::Light(_) => {}
        }
    }
    bounds.unwrap_or((Vec3::ZERO, Vec3::ONE))
}

#[allow(dead_code)] // Reserved for a non-WGPU platform fallback.
fn mesh_polygons(spec: &MeshSpec) -> Vec<Polygon3D> {
    let (min, max) = mesh_bounds(spec);
    let center = (min + max) * 0.5;
    let scale = 2.0 / (max - min).max_element().max(f32::EPSILON);
    let normalize = |point: Point3| (vec3(point) - center) * scale;
    spec.indices
        .chunks_exact(3)
        .enumerate()
        .map(|(triangle_index, triangle)| Polygon3D {
            vertices: triangle
                .iter()
                .map(|&index| normalize(spec.vertices[index as usize]))
                .collect(),
            fill: Some(mesh_triangle_fill(spec, triangle, triangle_index)),
            stroke: None,
        })
        .collect()
}

/// Convert the typed CPU scene into one indexed GPU upload. Positions in the
/// `Lines3DScene` are already normalized by `scene_scene`, and colors include
/// each mesh material, surface colormap, opacity, and declarative lighting.
/// Keeping all triangles in one draw is what gives overlapping scene children
/// a single depth buffer instead of GPUI-child paint ordering.
#[allow(dead_code)] // Reserved for a non-WGPU platform fallback.
fn legacy_scene_gpu_upload(scene: &Lines3DScene) -> (MeshUpload, Vec<[f32; 4]>) {
    let triangle_vertices = scene
        .polygons
        .iter()
        .map(|polygon| polygon.vertices.len())
        .sum();
    let mut positions = Vec::with_capacity(triangle_vertices + scene.lines.len() * 2);
    let mut colors = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity(triangle_vertices);
    let mut edges = Vec::new();

    for polygon in &scene.polygons {
        let Some(color) = polygon
            .fill
            .or_else(|| polygon.stroke.map(|(color, _)| color))
        else {
            continue;
        };
        let base = positions.len() as u32;
        positions.extend(polygon.vertices.iter().map(|point| point.to_array()));
        colors.extend(std::iter::repeat_n(
            [color.r, color.g, color.b, color.a],
            polygon.vertices.len(),
        ));
        for index in 1..polygon.vertices.len().saturating_sub(1) {
            indices.extend([base, base + index as u32, base + index as u32 + 1]);
        }
        if polygon.stroke.is_some() {
            for index in 0..polygon.vertices.len() {
                edges.extend([
                    base + index as u32,
                    base + ((index + 1) % polygon.vertices.len()) as u32,
                ]);
            }
        }
    }

    for line in &scene.lines {
        let base = positions.len() as u32;
        positions.extend([line.from.to_array(), line.to.to_array()]);
        let color = [line.color.r, line.color.g, line.color.b, line.color.a];
        colors.extend([color, color]);
        edges.extend([base, base + 1]);
    }

    (
        MeshUpload {
            positions_f32: positions,
            origin: [0.0; 3],
            indices,
            edge_indices: edges,
            values_f32: None,
            cell_values_f32: None,
        },
        colors,
    )
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

#[allow(dead_code)] // Reserved for a non-WGPU platform fallback.
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

#[allow(dead_code)] // Used only by the retained CPU fallback above.
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
    fn indexed_mesh_uses_one_retained_gpu_upload_with_direct_colors() {
        let spec = MeshSpec {
            id: "gpu-mesh".into(),
            vertices: vec![
                Point3::new(-1.0, -1.0, 0.0),
                Point3::new(1.0, -1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2],
            material: crate::scene3d::MaterialSpec {
                color: ColorRgba {
                    r: 0.2,
                    g: 0.4,
                    b: 0.6,
                    a: 0.5,
                },
                opacity: 0.5,
            },
            scalar_field: None,
        };
        let mut cache = Gpui3DCache::new();

        cache.mesh_element(&spec).expect("valid mesh");
        let state = cache
            .gpu_states
            .get("gpu-mesh")
            .expect("retained state")
            .borrow();
        let upload = state.upload.as_ref().expect("geometry upload");
        assert_eq!(upload.indices, vec![0, 1, 2]);
        assert_eq!(state.vertex_colors.as_ref().map(Vec::len), Some(3));
        assert_eq!(
            state.vertex_colors.as_ref().expect("direct colors")[0],
            [0.2, 0.4, 0.6, 0.25]
        );
    }

    #[test]
    fn vertex_scalar_mesh_retains_shared_vertices_and_topology() {
        let spec = MeshSpec {
            id: "vertex-scalar".into(),
            vertices: vec![
                Point3::new(-1.0, -1.0, 0.0),
                Point3::new(1.0, -1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(-1.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            material: crate::scene3d::MaterialSpec::default(),
            scalar_field: Some(crate::scene3d::MeshScalarField {
                association: ScalarAssociation::Vertex,
                colormap: crate::scene3d::ColormapSpec::Viridis,
                values: vec![0.0, 0.25, 0.75, 1.0],
                range: None,
                label: None,
            }),
        };

        let (upload, colors) = mesh_gpu_upload(&spec);

        assert_eq!(upload.positions_f32.len(), spec.vertices.len());
        assert_eq!(upload.indices, spec.indices);
        assert_eq!(colors.len(), spec.vertices.len());
        assert_ne!(colors[0], colors[3]);
    }

    #[test]
    fn cell_scalar_mesh_keeps_flat_per_triangle_expansion() {
        let spec = MeshSpec {
            id: "cell-scalar".into(),
            vertices: vec![
                Point3::new(-1.0, -1.0, 0.0),
                Point3::new(1.0, -1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(-1.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            material: crate::scene3d::MaterialSpec::default(),
            scalar_field: Some(crate::scene3d::MeshScalarField {
                association: ScalarAssociation::Cell,
                colormap: crate::scene3d::ColormapSpec::Viridis,
                values: vec![0.0, 1.0],
                range: None,
                label: None,
            }),
        };

        let (upload, colors) = mesh_gpu_upload(&spec);

        assert_eq!(upload.positions_f32.len(), spec.indices.len());
        assert_eq!(upload.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(colors.len(), spec.indices.len());
    }

    #[test]
    fn unlit_scene_mesh_reuses_indexed_vertex_geometry() {
        let mesh = MeshSpec {
            id: "scene-vertex-scalar".into(),
            vertices: vec![
                Point3::new(-1.0, -1.0, 0.0),
                Point3::new(1.0, -1.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(-1.0, 1.0, 0.0),
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
            material: crate::scene3d::MaterialSpec::default(),
            scalar_field: Some(crate::scene3d::MeshScalarField {
                association: ScalarAssociation::Vertex,
                colormap: crate::scene3d::ColormapSpec::Viridis,
                values: vec![0.0, 0.25, 0.75, 1.0],
                range: None,
                label: None,
            }),
        };
        let scene = crate::scene3d::SceneSpec {
            id: "unlit-indexed-scene".into(),
            camera: CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 45.0, 25.0)),
            children: vec![crate::scene3d::SceneNode::Mesh(mesh.clone())],
            interactions: Vec::new(),
            background: None,
            size: None,
        };

        let (upload, colors, wireframe) = scene_gpu_upload(&scene);

        assert_eq!(upload.positions_f32.len(), mesh.vertices.len());
        assert_eq!(upload.indices, mesh.indices);
        assert_eq!(colors.len(), mesh.vertices.len());
        assert!(!wireframe);
    }

    #[test]
    fn scene_wireframe_surface_deduplicates_shared_grid_edges() {
        let mut surface =
            SurfaceSpec::from_flat("wireframe-surface", vec![0.0, 1.0, 2.0, 3.0], 2, 2);
        surface.wireframe = true;
        let scene = crate::scene3d::SceneSpec {
            id: "wireframe-scene".into(),
            camera: CameraSpec::Orbit(OrbitCameraSpec::new(3.5, 45.0, 25.0)),
            children: vec![crate::scene3d::SceneNode::Surface(surface)],
            interactions: Vec::new(),
            background: None,
            size: None,
        };

        let (upload, colors, wireframe) = scene_gpu_upload(&scene);

        // A two-triangle grid has four boundary edges and one diagonal.
        assert_eq!(upload.positions_f32.len(), 10);
        assert_eq!(upload.edge_indices.len(), 10);
        assert!(upload.indices.is_empty());
        assert_eq!(colors.len(), 10);
        assert!(wireframe);
    }

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

    #[test]
    fn meshplot_invalid_upsert_preserves_last_valid_spec_and_retain_only_drops_it() {
        let valid = MeshPlotSpec::from_value(serde_json::json!({
            "schema_version": 1,
            "id": "plot",
            "revision": 2,
            "geometry": {
                "positions": [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                "triangles": [[0, 1, 2]]
            },
            "mode": "mesh"
        }))
        .unwrap();
        let mut cache = GpuiMeshPlotCache::new();
        cache.upsert(valid.clone()).unwrap();

        let mut invalid = valid.clone();
        invalid.geometry = serde_json::json!({"positions": [], "triangles": []});
        assert!(cache.upsert(invalid).is_err());
        assert_eq!(cache.get("plot"), Some(&valid));

        let mut stale = valid.clone();
        stale.revision = 1;
        stale.field = Some(serde_json::json!({"values": [2.0, 2.0, 2.0]}));
        assert!(
            cache
                .upsert(stale)
                .unwrap_err()
                .contains("stale mesh_plot revision")
        );
        assert_eq!(cache.get("plot"), Some(&valid));

        cache.retain_only(std::iter::empty::<&str>());
        assert_eq!(cache.len(), 0);
        assert!(cache.get("plot").is_none());
    }
}
