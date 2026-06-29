use super::camera_spec::CameraSpec;
use super::color_rgba::ColorRgba;
use super::interaction_mode::InteractionMode;
use super::line_segment_spec::LineSegmentSpec;
use super::line_strip_spec::LineStripSpec;
use super::types::SceneFingerprints;
use super::validate::validate_id;
use super::viewport_size::ViewportSize;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinesSpec {
    pub id: String,
    #[serde(default)]
    pub strips: Vec<LineStripSpec>,
    #[serde(default)]
    pub segments: Vec<LineSegmentSpec>,
    pub background: Option<ColorRgba>,
    pub camera: Option<CameraSpec>,
    #[serde(default)]
    pub interactions: Vec<InteractionMode>,
    pub size: Option<ViewportSize>,
}

impl LinesSpec {
    pub fn validate(&self) -> Result<(), Scene3DError> {
        validate_id(&self.id, "lines.id")?;
        if self.strips.is_empty() && self.segments.is_empty() {
            return Err(Scene3DError::EmptyData { field: "lines" });
        }
        for strip in &self.strips {
            strip.validate()?;
        }
        for segment in &self.segments {
            segment.validate()?;
        }
        if let Some(background) = &self.background {
            background.validate("lines.background")?;
        }
        if let Some(camera) = &self.camera {
            camera.validate()?;
        }
        if let Some(size) = &self.size {
            size.validate()?;
        }
        Ok(())
    }

    #[must_use]
    pub fn flattened_segments(&self) -> Vec<LineSegmentSpec> {
        let strip_segment_count: usize = self
            .strips
            .iter()
            .map(|strip| strip.points.len().saturating_sub(1))
            .sum();
        let mut segments = Vec::with_capacity(self.segments.len() + strip_segment_count);
        segments.extend_from_slice(&self.segments);
        for strip in &self.strips {
            segments.extend(strip.to_segments());
        }
        segments
    }

    pub(crate) fn fingerprints(&self) -> SceneFingerprints {
        let mut geometry = DefaultHasher::new();
        self.id.hash(&mut geometry);
        for strip in &self.strips {
            strip.hash_into(&mut geometry);
        }
        for segment in &self.segments {
            segment.hash_into(&mut geometry);
        }

        let mut material = DefaultHasher::new();
        if let Some(background) = &self.background {
            background.hash_into(&mut material);
        }
        if let Some(size) = &self.size {
            size.hash_into(&mut material);
        }

        let mut camera = DefaultHasher::new();
        let default_camera = CameraSpec::default();
        self.camera
            .as_ref()
            .unwrap_or(&default_camera)
            .hash_into(&mut camera);
        self.interactions.hash(&mut camera);

        SceneFingerprints {
            geometry: geometry.finish(),
            material: material.finish(),
            camera: camera.finish(),
        }
    }
}
