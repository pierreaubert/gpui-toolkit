use crate::cache::RetainedSceneCache;
use crate::error::Scene3DError;
use crate::scene3d::{
    CameraSpec, ColorRgba, ColormapSpec, LinesSpec, OrbitCameraSpec, Point3, SurfaceSpec,
};
use d3rs::gpu3d::{
    Colormap, Line3D, Lines3DElement, Lines3DScene, Lines3DState, Surface3DConfig,
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
}

impl std::fmt::Debug for Gpui3DCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gpui3DCache")
            .field("resources", &self.resources)
            .field("surface_count", &self.surfaces.len())
            .field("line_state_count", &self.line_states.len())
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
        let data = surface_data(spec);
        let config = surface_config(spec)?;

        if let Some(element) = self.surfaces.get_mut(&spec.id) {
            if update.dirty.updates_geometry() {
                element.set_data(data);
            }
            if update.dirty.updates_material() {
                element.set_config(config);
            }
            if update.dirty.updates_camera() {
                *element.state().borrow_mut() = surface_state(spec)?;
            }
            return Ok(element.clone());
        }

        let element = Surface3DElement::new(data, config);
        self.surfaces.insert(spec.id.clone(), element.clone());
        Ok(element)
    }

    pub fn lines_element(&mut self, spec: &LinesSpec) -> Result<Lines3DElement, Scene3DError> {
        let update = self.resources.upsert_lines(spec)?;
        let scene = lines_scene(spec);

        let state = match self.line_states.entry(spec.id.clone()) {
            Entry::Occupied(entry) => entry.get().clone(),
            Entry::Vacant(entry) => entry
                .insert(Rc::new(RefCell::new(lines_state(spec)?)))
                .clone(),
        };

        if update.dirty.updates_camera() {
            *state.borrow_mut() = lines_state(spec)?;
        }

        Ok(Lines3DElement::new(state, scene))
    }

    pub fn retain_only<I, S>(&mut self, ids: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let live: Vec<String> = ids.into_iter().map(|id| id.as_ref().to_string()).collect();
        self.resources.retain_only(live.iter().map(String::as_str));
        let live: std::collections::HashSet<String> = live.into_iter().collect();
        self.surfaces.retain(|id, _| live.contains(id));
        self.line_states.retain(|id, _| live.contains(id));
    }
}

fn surface_data(spec: &SurfaceSpec) -> SurfaceData {
    let mut data = SurfaceData::from_grid(spec.x_values(), spec.y_values(), spec.z.rows())
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
}
