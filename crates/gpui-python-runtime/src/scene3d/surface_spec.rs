use super::axis_labels::AxisLabels;
use super::camera_spec::CameraSpec;
use super::colormap_spec::ColormapSpec;
use super::grid_data::GridData;
use super::hash::hash_optional_f64_slice;
use super::interaction_mode::InteractionMode;
use super::scalar_range::ScalarRange;
use super::types::SceneFingerprints;
use super::validate::validate_axis;
use super::validate::validate_id;
use super::validate::validate_positive_f64_slice;
use super::viewport_size::ViewportSize;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{OnceLock, RwLock};

static DEFAULT_AXIS_CACHE: OnceLock<RwLock<HashMap<usize, &'static [f64]>>> = OnceLock::new();

fn default_axis_values(size: usize) -> &'static [f64] {
    let cache = DEFAULT_AXIS_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Some(values) = cache
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&size)
        .copied()
    {
        return values;
    }

    let mut map = cache
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *map.entry(size).or_insert_with(|| {
        let values: &'static [f64] = Box::leak(
            (0..size)
                .map(|value| value as f64)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        values
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceSpec {
    pub id: String,
    pub z: GridData,
    pub x: Option<Vec<f64>>,
    pub y: Option<Vec<f64>>,
    #[serde(default)]
    pub colormap: ColormapSpec,
    #[serde(default)]
    pub wireframe: bool,
    #[serde(default)]
    pub x_log: bool,
    #[serde(default)]
    pub y_log: bool,
    #[serde(default)]
    pub z_log: bool,
    pub z_range: Option<ScalarRange>,
    #[serde(default)]
    pub labels: AxisLabels,
    pub camera: Option<CameraSpec>,
    #[serde(default)]
    pub interactions: Vec<InteractionMode>,
    pub size: Option<ViewportSize>,
}

impl SurfaceSpec {
    #[must_use]
    pub fn from_flat(id: impl Into<String>, z: Vec<f64>, width: usize, height: usize) -> Self {
        Self {
            id: id.into(),
            z: GridData::from_flat(z, width, height),
            x: None,
            y: None,
            colormap: ColormapSpec::default(),
            wireframe: false,
            x_log: false,
            y_log: false,
            z_log: false,
            z_range: None,
            labels: AxisLabels::default(),
            camera: None,
            interactions: Vec::new(),
            size: None,
        }
    }

    pub fn from_rows(id: impl Into<String>, rows: Vec<Vec<f64>>) -> Result<Self, Scene3DError> {
        Ok(Self {
            z: GridData::from_rows(rows)?,
            ..Self::from_flat(id, Vec::new(), 0, 0)
        })
    }

    pub fn validate(&self) -> Result<(), Scene3DError> {
        validate_id(&self.id, "surface.id")?;
        self.z.validate()?;
        if let Some(x) = &self.x {
            validate_axis(x, self.z.width, "x", "grid_width", self.x_log)?;
        } else if self.x_log {
            return Err(Scene3DError::InvalidData {
                field: "x",
                reason: "log axis requires explicit positive values",
            });
        }
        if let Some(y) = &self.y {
            validate_axis(y, self.z.height, "y", "grid_height", self.y_log)?;
        } else if self.y_log {
            return Err(Scene3DError::InvalidData {
                field: "y",
                reason: "log axis requires explicit positive values",
            });
        }
        if self.z_log {
            validate_positive_f64_slice(&self.z.values, "z")?;
        }
        if let Some(range) = &self.z_range {
            if self.z_log {
                range.validate_positive("z_range")?;
            } else {
                range.validate("z_range")?;
            }
        }
        if let Some(camera) = &self.camera {
            camera.validate()?;
        }
        if let Some(size) = &self.size {
            size.validate()?;
        }
        Ok(())
    }

    /// Return the x-axis coordinates.
    ///
    /// If explicit `x` values were supplied, they are borrowed from `self`.
    /// Otherwise a cached default `[0, 1, ...]` vector is cloned.
    #[must_use]
    pub fn x_values(&self) -> Cow<'_, [f64]> {
        match &self.x {
            Some(values) => Cow::Borrowed(values),
            None => Cow::Borrowed(default_axis_values(self.z.width)),
        }
    }

    /// Return the y-axis coordinates.
    ///
    /// If explicit `y` values were supplied, they are borrowed from `self`.
    /// Otherwise a cached default `[0, 1, ...]` vector is cloned.
    #[must_use]
    pub fn y_values(&self) -> Cow<'_, [f64]> {
        match &self.y {
            Some(values) => Cow::Borrowed(values),
            None => Cow::Borrowed(default_axis_values(self.z.height)),
        }
    }

    pub(crate) fn fingerprints(&self) -> SceneFingerprints {
        let mut geometry = DefaultHasher::new();
        self.id.hash(&mut geometry);
        self.z.hash_into(&mut geometry);
        hash_optional_f64_slice(&self.x, &mut geometry);
        hash_optional_f64_slice(&self.y, &mut geometry);
        self.x_log.hash(&mut geometry);
        self.y_log.hash(&mut geometry);
        self.z_log.hash(&mut geometry);
        if let Some(range) = &self.z_range {
            range.hash_into(&mut geometry);
        }

        let mut material = DefaultHasher::new();
        self.colormap.hash(&mut material);
        self.wireframe.hash(&mut material);
        self.labels.hash_into(&mut material);
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
