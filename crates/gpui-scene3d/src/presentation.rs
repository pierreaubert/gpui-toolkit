//! Physical-size and scene-revision keys shared by presentation paths.

// Rust guideline compliant 2026-02-21

use crate::retained::SceneRevision;
use crate::{Camera, ClipPlane, Scene, SceneError, TransferFunction};

#[derive(PartialEq)]
struct MaterialKey {
    color: [f32; 4],
    transfer: Option<TransferFunction>,
    opacity: f32,
    clip_planes: Vec<ClipPlane>,
}

impl MaterialKey {
    fn matches(&self, actor: &crate::Actor) -> bool {
        self.color == actor.effective_color()
            && self.transfer.as_ref() == actor.active_transfer()
            && self.opacity == actor.opacity
            && self.clip_planes == actor.clip_planes
    }
}

pub(crate) struct RenderKey {
    prepared: SceneRevision,
    camera: Camera,
    size: [u32; 2],
    background: [f32; 4],
    surface_limits: gpui_mesh::SurfaceExtractionLimits,
    gpu_upload_limits: crate::GpuUploadLimits,
    resource_retry: u64,
    materials: Vec<MaterialKey>,
}

impl RenderKey {
    #[cfg(feature = "gpui-gpu")]
    pub(crate) fn geometry_matches(&self, scene: &Scene) -> Result<bool, SceneError> {
        self.prepared.matches(scene)
    }

    #[cfg(feature = "gpui-gpu")]
    pub(crate) fn camera_matches(&self, camera: Camera) -> bool {
        self.camera == camera
    }

    #[cfg(feature = "gpui-gpu")]
    pub(crate) fn resource_inputs_match(
        &self,
        scene: &Scene,
        size: [u32; 2],
    ) -> Result<bool, SceneError> {
        Ok(self.size == size
            && self.surface_limits == scene.surface_limits
            && self.gpu_upload_limits == scene.gpu_upload_limits
            && self.resource_retry == scene.resource_retry
            && self.prepared.matches(scene)?
            && self
                .materials
                .iter()
                .zip(scene.actors.iter().filter(|actor| actor.visible))
                .all(|(material, actor)| material.matches(actor)))
    }

    pub(crate) fn new(scene: &Scene, camera: Camera, size: [u32; 2]) -> Result<Self, SceneError> {
        camera.validate()?;
        scene.validate_render_state()?;
        Ok(Self {
            prepared: SceneRevision::new(scene)?,
            camera,
            size,
            background: scene.background,
            surface_limits: scene.surface_limits,
            gpu_upload_limits: scene.gpu_upload_limits,
            resource_retry: scene.resource_retry,
            materials: scene
                .actors
                .iter()
                .filter(|actor| actor.visible)
                .map(|actor| MaterialKey {
                    color: actor.effective_color(),
                    transfer: actor.active_transfer().cloned(),
                    opacity: actor.opacity,
                    clip_planes: actor.clip_planes.clone(),
                })
                .collect(),
        })
    }

    pub(crate) fn matches(
        &self,
        scene: &Scene,
        camera: Camera,
        size: [u32; 2],
    ) -> Result<bool, SceneError> {
        Ok(self.camera == camera
            && self.size == size
            && self.background == scene.background
            && self.surface_limits == scene.surface_limits
            && self.gpu_upload_limits == scene.gpu_upload_limits
            && self.resource_retry == scene.resource_retry
            && self.prepared.matches(scene)?
            && self
                .materials
                .iter()
                .zip(scene.actors.iter().filter(|actor| actor.visible))
                .all(|(material, actor)| material.matches(actor)))
    }
}

pub(crate) fn physical_size(width: f32, height: f32, scale: f32) -> Option<[u32; 2]> {
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let convert = |value: f32| {
        let pixels = (f64::from(value) * f64::from(scale)).ceil();
        (pixels.is_finite() && pixels > 0.0 && pixels <= f64::from(u32::MAX))
            .then_some(pixels as u32)
    };
    Some([convert(width)?, convert(height)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Actor;
    use gpui_mesh::{DataSet, Field};
    use std::sync::Arc;

    #[test]
    fn invalid_cameras_fail_before_presentation_retention() {
        let camera = Camera {
            projection: crate::Projection::Perspective { fov_y: 0.0 },
            ..Camera::default()
        };
        let scene = Scene::new();
        assert!(matches!(
            RenderKey::new(&scene, camera, [32, 32]),
            Err(SceneError::InvalidCamera(_))
        ));
        assert_eq!(scene.preparation_stats().geometry_builds, 0);
    }

    #[test]
    fn frame_key_reuses_static_frames_and_tracks_effective_render_inputs() {
        let dataset = DataSet::structured_grid("key", [2; 3], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 8]));
        let scene = Scene::new().with_actor(
            Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])),
        );
        let camera = Camera::default();
        let key = RenderKey::new(&scene, camera, [100, 50]).unwrap();
        assert!(key.matches(&scene.clone(), camera, [100, 50]).unwrap());
        assert!(!key.matches(&scene, camera, [200, 100]).unwrap());
        let mut moved = camera;
        moved.orbit(0.1, 0.0);
        assert!(!key.matches(&scene, moved, [100, 50]).unwrap());
        for mutation in 0..7 {
            let mut changed = scene.clone();
            match mutation {
                0 => changed.background[0] = 1.0,
                1 => changed.actors[0].color[0] = 0.0,
                2 => changed.actors[0].opacity = 0.3,
                3 => changed.actors[0].transfer = Some(TransferFunction::coolwarm([0.0, 1.0])),
                4 => changed.actors[0].clip_planes.push(ClipPlane {
                    origin: [0.0; 3],
                    normal: [1.0, 0.0, 0.0],
                }),
                5 => changed.actors[0].visible = false,
                _ => Arc::make_mut(&mut changed.actors[0].dataset.positions)[0][0] += 1.0,
            }
            assert_eq!(
                key.matches(&changed, camera, [100, 50]).unwrap(),
                mutation == 1,
                "effective render mutation {mutation}"
            );
        }
    }

    #[test]
    fn unused_material_settings_do_not_invalidate_or_reject_retained_frames() {
        let dataset = DataSet::structured_grid("unused", [2; 3], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("p", vec![0.5; 8]));
        let camera = Camera::default();
        let mut flat = Scene::new().with_actor(Actor::new(dataset.clone()));
        let flat_key = RenderKey::new(&flat, camera, [16, 16]).unwrap();
        flat.actors[0].transfer = Some(TransferFunction::viridis([f64::NAN, 1.0]));
        flat.validate_render_state().unwrap();
        assert!(flat_key.matches(&flat, camera, [16, 16]).unwrap());

        let mut mapped = Scene::new().with_actor(
            Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])),
        );
        let mapped_key = RenderKey::new(&mapped, camera, [16, 16]).unwrap();
        mapped.actors[0].color = [f32::NAN; 4];
        mapped.validate_render_state().unwrap();
        assert!(mapped_key.matches(&mapped, camera, [16, 16]).unwrap());
    }

    #[test]
    fn policy_changes_refresh_presentation_without_invalidating_geometry_identity() {
        let mut scene = Scene::new().with_actor(Actor::new(DataSet::surface(
            "policy",
            vec![[0.0; 3]; 3],
            &[[0, 1, 2]],
        )));
        let camera = Camera::default();
        let key = RenderKey::new(&scene, camera, [16, 16]).unwrap();
        scene.surface_limits.max_face_bytes = 0;
        assert!(!key.matches(&scene, camera, [16, 16]).unwrap());
        assert!(key.prepared.matches(&scene).unwrap());
        scene.surface_limits = gpui_mesh::SurfaceExtractionLimits::default();
        scene.gpu_upload_limits.max_resource_bytes = 0;
        assert!(!key.matches(&scene, camera, [16, 16]).unwrap());
        assert!(key.prepared.matches(&scene).unwrap());
    }

    #[test]
    fn physical_dimensions_follow_dpi_and_reject_invalid_sizes() {
        assert_eq!(physical_size(100.1, 50.0, 2.0), Some([201, 100]));
        assert_eq!(physical_size(100.1, 50.0, 1.0), Some([101, 50]));
        for (width, scale) in [
            (0.0, 1.0),
            (-1.0, 2.0),
            (f32::NAN, 1.0),
            (1.0, f32::INFINITY),
            (f32::MAX, 2.0),
        ] {
            assert_eq!(physical_size(width, 50.0, scale), None);
        }
    }
}
