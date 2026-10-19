//! Scene and actor descriptions.

// Rust guideline compliant 2026-02-21

use crate::transfer::TransferFunction;
use gpui_mesh::{Association, DataSet};
use std::sync::{Arc, Mutex};

/// A clipping plane: the positive half-space is kept.
///
/// Origins use world coordinates. Renderers subtract their local origin before
/// projection and clipping. The GPU surface backend accepts up to six planes;
/// unsupported configurations return explicit errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipPlane {
    /// A point on the plane.
    pub origin: [f64; 3],
    /// Plane normal (need not be unit length).
    pub normal: [f64; 3],
}

impl ClipPlane {
    /// Checks that the supplied point and normal contain finite coefficients.
    ///
    /// Zero normals retain every point, matching the nonnegative half-space rule.
    ///
    /// # Errors
    ///
    /// Returns an error when any coefficient is nonfinite.
    pub fn validate(&self) -> Result<(), crate::SceneError> {
        if self
            .origin
            .iter()
            .chain(self.normal.iter())
            .any(|v| !v.is_finite())
        {
            return Err(crate::SceneError::InvalidClipPlane(
                "point and normal must be finite",
            ));
        }
        Ok(())
    }

    /// Signed half-space value; negative points are clipped away.
    ///
    /// This utility scales with the normal and follows `f64` arithmetic.
    /// Renderers use validated local equations instead of world-space products.
    #[must_use]
    pub fn distance(&self, point: [f64; 3]) -> f64 {
        (point[0] - self.origin[0]) * self.normal[0]
            + (point[1] - self.origin[1]) * self.normal[1]
            + (point[2] - self.origin[2]) * self.normal[2]
    }

    /// Builds a finite world-space shader equation `[normal.xyz, offset]`.
    ///
    /// The positive half-space is `dot(position, normal) >= offset`. Coefficients
    /// are rescaled together before float conversion. For large coordinates,
    /// use the scene uniform builder, which subtracts the actor's local origin.
    ///
    /// # Errors
    ///
    /// Returns an error for nonfinite inputs, an offset beyond `f64` range,
    /// or a required coefficient below the shader's normal float range.
    pub fn to_uniform(&self) -> Result<[f32; 4], crate::SceneError> {
        self.relative_to([0.0; 3])?.to_uniform()
    }

    pub(crate) fn relative_to(self, origin: [f64; 3]) -> Result<PlaneEquation, crate::SceneError> {
        self.validate()?;
        let scale = self.normal.iter().fold(0.0f64, |m, n| m.max(n.abs()));
        if scale == 0.0 {
            return Ok(PlaneEquation {
                normal: [0.0; 3],
                offset: 0.0,
            });
        }
        let normal = self.normal.map(|n| n / scale);
        let point: [f64; 3] = std::array::from_fn(|axis| self.origin[axis] - origin[axis]);
        if point.iter().any(|v| !v.is_finite()) {
            return Err(crate::SceneError::InvalidClipPlane(
                "local point exceeds f64 range",
            ));
        }
        let offset = compensated_dot(point, normal).ok_or(crate::SceneError::InvalidClipPlane(
            "local offset exceeds f64 range",
        ))?;
        Ok(PlaneEquation { normal, offset })
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PlaneEquation {
    normal: [f64; 3],
    offset: f64,
}

impl PlaneEquation {
    pub(crate) fn distance(&self, point: [f64; 3]) -> f64 {
        self.normal[0].mul_add(
            point[0],
            self.normal[1].mul_add(point[1], self.normal[2].mul_add(point[2], -self.offset)),
        )
    }

    pub(crate) fn to_uniform(self) -> Result<[f32; 4], crate::SceneError> {
        // Leave headroom for fragment arithmetic, including a large offset.
        // If every normal component underflows, a sufficiently distant plane
        // can still be represented by its constant half-space result.
        let scale = (self.offset.abs() / (f64::from(f32::MAX) / 16.0)).max(1.0);
        let coefficients = [
            (self.normal[0] / scale) as f32,
            (self.normal[1] / scale) as f32,
            (self.normal[2] / scale) as f32,
            (self.offset / scale) as f32,
        ];
        let max_local_dot = self.normal.iter().map(|n| n.abs()).sum::<f64>() * f64::from(f32::MAX);
        if self.offset.abs() > max_local_dot {
            return Ok([0.0, 0.0, 0.0, self.offset.signum() as f32]);
        }
        if (0..3).any(|axis| {
            self.normal[axis] != 0.0
                && (coefficients[axis] == 0.0 || coefficients[axis].is_subnormal())
        }) || (self.offset != 0.0 && (coefficients[3] == 0.0 || coefficients[3].is_subnormal()))
        {
            return Err(crate::SceneError::UnsupportedPrecision(
                "clipping coefficient is below normal f32 range",
            ));
        }
        Ok(coefficients)
    }

    fn is_active(&self) -> bool {
        self.normal != [0.0; 3]
    }
}

#[cfg(test)]
mod plane_equation_tests {
    use super::*;

    #[test]
    fn normal_scaling_and_tangential_point_choice_preserve_the_equation() {
        for scale in [1e-300, 1.0, 1e300] {
            for tangent in [0.0, 1e12] {
                let plane = ClipPlane {
                    origin: [tangent + 0.125, -tangent, 1e250],
                    normal: [scale, scale, 0.0],
                };
                assert_eq!(plane.to_uniform().unwrap(), [1.0, 1.0, 0.0, 0.125]);
                let local = plane.relative_to([0.0; 3]).unwrap();
                assert_eq!(local.distance([0.125, 0.0, 0.0]), 0.0);
                assert!(local.distance([0.0; 3]) < 0.0);
            }
        }
    }

    #[test]
    fn offsets_cancel_before_overflow_and_distant_half_spaces_stay_finite() {
        let plane = ClipPlane {
            origin: [f64::MAX, f64::MAX, -f64::MAX],
            normal: [1.0; 3],
        };
        assert_eq!(plane.relative_to([0.0; 3]).unwrap().offset, f64::MAX);
        assert_eq!(plane.to_uniform().unwrap(), [0.0, 0.0, 0.0, 1.0]);
        let opposite = ClipPlane {
            normal: [-1.0; 3],
            ..plane
        };
        assert_eq!(opposite.to_uniform().unwrap(), [0.0, 0.0, 0.0, -1.0]);
        let overflow = ClipPlane {
            origin: [f64::MAX; 3],
            ..plane
        };
        assert!(matches!(
            overflow.to_uniform(),
            Err(crate::SceneError::InvalidClipPlane(_))
        ));
    }

    #[test]
    fn zero_normal_is_inactive_and_nonfinite_or_lost_coefficients_are_explicit() {
        let zero = ClipPlane {
            origin: [1e250; 3],
            normal: [0.0; 3],
        };
        assert!(local_planes(&[zero], [0.0; 3]).unwrap().is_empty());
        assert_eq!(zero.to_uniform().unwrap(), [0.0; 4]);
        for bad in [
            ClipPlane {
                origin: [f64::NAN; 3],
                ..zero
            },
            ClipPlane {
                normal: [f64::INFINITY; 3],
                ..zero
            },
        ] {
            assert!(matches!(
                bad.to_uniform(),
                Err(crate::SceneError::InvalidClipPlane(_))
            ));
        }
        for tiny in [1e-100, 1e-40] {
            let underflow = ClipPlane {
                origin: [0.0; 3],
                normal: [1.0, tiny, 0.0],
            };
            assert!(
                underflow
                    .relative_to([0.0; 3])
                    .unwrap()
                    .distance([0.0, -1.0, 0.0])
                    < 0.0
            );
            assert!(matches!(
                underflow.to_uniform(),
                Err(crate::SceneError::UnsupportedPrecision(_))
            ));
            let offset = ClipPlane {
                origin: [tiny, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            };
            assert!(matches!(
                offset.to_uniform(),
                Err(crate::SceneError::UnsupportedPrecision(_))
            ));
        }
    }
}

pub(crate) fn local_planes(
    planes: &[ClipPlane],
    origin: [f64; 3],
) -> Result<smallvec::SmallVec<[PlaneEquation; 6]>, crate::SceneError> {
    let mut result = smallvec::SmallVec::new();
    for plane in planes {
        let equation = plane.relative_to(origin)?;
        if equation.is_active() {
            result.push(equation);
        }
    }
    Ok(result)
}

fn compensated_dot(a: [f64; 3], b: [f64; 3]) -> Option<f64> {
    let products: [f64; 3] = std::array::from_fn(|axis| a[axis] * b[axis]);
    let errors: [f64; 3] = std::array::from_fn(|axis| a[axis].mul_add(b[axis], -products[axis]));
    // Cancel opposite signs first so an intermediate same-sign sum cannot
    // overflow when the final offset is still representable.
    let [x, y, z] = if products[0].is_sign_negative() != products[1].is_sign_negative() {
        products
    } else if products[0].is_sign_negative() != products[2].is_sign_negative() {
        [products[0], products[2], products[1]]
    } else {
        products
    };
    let pair = x + y;
    if !pair.is_finite() {
        return None;
    }
    let virtual_y = pair - x;
    let pair_error = (x - (pair - virtual_y)) + (y - virtual_y);
    let sum = pair + z;
    if !sum.is_finite() {
        return None;
    }
    let virtual_z = sum - pair;
    let sum_error = (pair - (sum - virtual_z)) + (z - virtual_z);
    let value = sum + (pair_error + sum_error + errors[0] + errors[1] + errors[2]);
    value.is_finite().then_some(value)
}

/// An axis-aligned clipping box: the intersection of six half-spaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipBox {
    /// Minimum corner kept.
    pub min: [f64; 3],
    /// Maximum corner kept.
    pub max: [f64; 3],
}

impl ClipBox {
    /// Box from dataset bounds.
    #[must_use]
    pub fn from_bounds(bounds: gpui_mesh::Bounds) -> Self {
        Self {
            min: bounds.min,
            max: bounds.max,
        }
    }

    /// The six clipping planes (positive half-spaces kept).
    #[must_use]
    pub fn planes(&self) -> [ClipPlane; 6] {
        [
            ClipPlane {
                origin: [self.min[0], 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            },
            ClipPlane {
                origin: [self.max[0], 0.0, 0.0],
                normal: [-1.0, 0.0, 0.0],
            },
            ClipPlane {
                origin: [0.0, self.min[1], 0.0],
                normal: [0.0, 1.0, 0.0],
            },
            ClipPlane {
                origin: [0.0, self.max[1], 0.0],
                normal: [0.0, -1.0, 0.0],
            },
            ClipPlane {
                origin: [0.0, 0.0, self.min[2]],
                normal: [0.0, 0.0, 1.0],
            },
            ClipPlane {
                origin: [0.0, 0.0, self.max[2]],
                normal: [0.0, 0.0, -1.0],
            },
        ]
    }
}

/// One dataset placed in a scene, coloured by an optional field.
#[derive(Debug, Clone)]
pub struct Actor {
    /// Dataset to draw (surface extraction happens at preparation).
    pub dataset: DataSet,
    /// Point or cell field used for colouring; `None` draws `color`.
    pub field: Option<(Association, String)>,
    /// Transfer function for the colouring field.
    pub transfer: Option<TransferFunction>,
    /// Flat colour when no field is set, as RGBA in `[0,1]`.
    pub color: [f32; 4],
    /// Whether the actor is drawn.
    pub visible: bool,
    /// Whether unique black edges one physical pixel wide overlay the surface.
    /// Edges use the surface's scalar alpha/opacity, clipping and depth test.
    /// Clipping keeps original mesh edge segments without creating cut outlines.
    pub wireframe: bool,
    /// Overall opacity multiplier; below 1 the actor renders in the
    /// sorted transparent pass.
    pub opacity: f32,
    /// Clipping planes applied to this actor in every render path.
    pub clip_planes: Vec<ClipPlane>,
}

impl Actor {
    /// Creates a flat-coloured actor.
    pub fn new(dataset: DataSet) -> Self {
        Self {
            dataset,
            field: None,
            transfer: None,
            color: [0.8, 0.8, 0.85, 1.0],
            visible: true,
            wireframe: false,
            opacity: 1.0,
            clip_planes: Vec::new(),
        }
    }

    /// Colours the actor by a point field with a transfer function.
    #[must_use]
    pub fn coloured_by(mut self, field: impl Into<String>, transfer: TransferFunction) -> Self {
        self.field = Some((Association::Point, field.into()));
        self.transfer = Some(transfer);
        self
    }

    /// Colours the actor by a cell field with a transfer function.
    #[must_use]
    pub fn coloured_by_cell(
        mut self,
        field: impl Into<String>,
        transfer: TransferFunction,
    ) -> Self {
        self.field = Some((Association::Cell, field.into()));
        self.transfer = Some(transfer);
        self
    }

    /// Sets the flat colour.
    #[must_use]
    pub fn color(mut self, rgba: [f32; 4]) -> Self {
        self.color = rgba;
        self
    }

    /// Sets the overall opacity; below 1 the actor is transparent and
    /// renders back-to-front after the opaque pass.
    #[must_use]
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Adds a clipping plane (positive half-space kept).
    #[must_use]
    pub fn clip(mut self, plane: ClipPlane) -> Self {
        self.clip_planes.push(plane);
        self
    }

    /// Clips to an axis-aligned box (adds its six planes).
    #[must_use]
    pub fn clip_box(mut self, clip_box: ClipBox) -> Self {
        self.clip_planes.extend(clip_box.planes());
        self
    }

    /// Whether this actor needs the transparent pass.
    #[must_use]
    pub fn is_transparent(&self) -> bool {
        self.opacity < 1.0
            || self
                .active_transfer()
                .map_or(self.color[3] < 1.0, |tf| !tf.is_opaque())
    }

    pub(crate) fn active_transfer(&self) -> Option<&TransferFunction> {
        self.field.as_ref().and(self.transfer.as_ref())
    }

    #[cfg(any(test, feature = "gpu", feature = "gpui"))]
    pub(crate) fn effective_color(&self) -> [f32; 4] {
        if self.active_transfer().is_some() {
            [0.0; 4]
        } else {
            self.color
        }
    }

    /// Toggles black wireframe edges one physical pixel wide with surface alpha/opacity.
    #[must_use]
    pub fn wireframe(mut self, enabled: bool) -> Self {
        self.wireframe = enabled;
        self
    }

    /// Resolved scalar values per point or cell, when a field is set.
    #[must_use]
    pub fn scalars(&self) -> Option<(Association, &[f64])> {
        let (association, name) = self.field.as_ref()?;
        let field = self.dataset.field(*association, name)?;
        field.as_scalar().map(|values| (*association, values))
    }
}

#[cfg(test)]
mod material_tests {
    use super::*;

    #[test]
    fn transfer_alpha_controls_the_render_pass() {
        let dataset = DataSet::surface("empty", Vec::new(), &[]);
        for alpha in [0.0, 0.25, 1.0] {
            let transfer = TransferFunction::from_stops(
                [0.0, 1.0],
                &[(0.0, [1.0, 0.0, 0.0, alpha]), (1.0, [0.0, 0.0, 1.0, 1.0])],
            );
            let actor = Actor::new(dataset.clone()).coloured_by("p", transfer);
            assert_eq!(actor.is_transparent(), alpha < 1.0);
        }
        assert!(
            Actor::new(dataset.clone())
                .coloured_by("p", TransferFunction::from_stops([0.0, 1.0], &[]))
                .is_transparent()
        );
        let mut flat = Actor::new(dataset).color([1.0, 0.0, 0.0, 0.5]);
        flat.transfer = Some(TransferFunction::viridis([0.0, 1.0]));
        assert!(
            flat.is_transparent(),
            "an unused transfer must not hide flat-color alpha"
        );
    }
}

/// A set of actors plus background, rendered from one camera.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    /// Actors in draw order.
    pub actors: Vec<Actor>,
    /// Background RGBA in `[0,1]`.
    pub background: [f32; 4],
    /// Text annotations for hosts with a text system (GPUI element).
    pub annotations: Vec<crate::legend::Annotation>,
    /// Workspace allowance for newly built surface geometry.
    ///
    /// Changing it does not discard already prepared geometry. Clear preparation
    /// explicitly to release cached buffers; existing snapshots may still own them.
    pub surface_limits: gpui_mesh::SurfaceExtractionLimits,
    /// GPU resource and CPU primitive-packing allowances applied on upload/sync.
    pub gpu_upload_limits: crate::GpuUploadLimits,
    pub(crate) resource_retry: u64,
    pub(crate) preparation: Arc<Mutex<crate::retained::PreparationCache>>,
}

#[cfg(test)]
pub(crate) fn translated_clipped_scene(offset: f64) -> Scene {
    let mut scene = Scene::new();
    for (index, delta) in [-1.25, 1.25].into_iter().enumerate() {
        let center = [offset + delta, offset, offset];
        let dataset =
            DataSet::structured_grid("translated", [3; 3], center.map(|v| v - 0.5), [0.5; 3])
                .unwrap();
        let color = if index == 0 {
            [1.0, 0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0, 1.0]
        };
        let normal = if index == 0 {
            [1.0, 0.5, -0.25]
        } else {
            [-0.5, 1.0, 0.25]
        };
        scene
            .actors
            .push(Actor::new(dataset).color(color).clip(ClipPlane {
                origin: center,
                normal,
            }));
    }
    scene
}

#[cfg(all(test, any(feature = "gpu", feature = "gpui")))]
pub(crate) fn coefficient_sensitive_scene() -> Scene {
    let dataset = DataSet::surface(
        "precision",
        vec![[0.0, -1.0, -1.0], [0.0, 1.0, -1.0], [0.0, 0.0, 1.0]],
        &[[0, 1, 2]],
    );
    Scene::new().with_actor(
        Actor::new(dataset)
            .color([1.0, 0.0, 0.0, 1.0])
            .clip(ClipPlane {
                origin: [0.0; 3],
                normal: [1.0, 1e-100, 0.0],
            }),
    )
}

impl Scene {
    pub(crate) fn validate_render_state(&self) -> Result<(), crate::SceneError> {
        validate_rgba(self.background)?;
        for actor in self.actors.iter().filter(|actor| actor.visible) {
            if !actor.opacity.is_finite() || !(0.0..=1.0).contains(&actor.opacity) {
                return Err(crate::SceneError::InvalidMaterial(
                    "opacity must be finite and in [0,1]",
                ));
            }
            if let Some(transfer) = actor.active_transfer() {
                transfer.validate()?;
            } else {
                validate_rgba(actor.color)?;
            }
            for plane in &actor.clip_planes {
                plane.validate()?;
            }
        }
        Ok(())
    }

    /// Creates an empty scene with a dark background.
    pub fn new() -> Self {
        Self {
            actors: Vec::new(),
            background: [0.08, 0.09, 0.11, 1.0],
            annotations: Vec::new(),
            surface_limits: gpui_mesh::SurfaceExtractionLimits::default(),
            gpu_upload_limits: crate::GpuUploadLimits::default(),
            resource_retry: 0,
            preparation: Arc::default(),
        }
    }

    /// Adds an actor.
    #[must_use]
    pub fn with_actor(mut self, actor: Actor) -> Self {
        self.actors.push(actor);
        self
    }

    /// Sets the surface workspace allowance for future geometry builds.
    ///
    /// Cached geometry remains reusable because it needs no face workspace.
    #[must_use]
    pub fn with_surface_limits(mut self, limits: gpui_mesh::SurfaceExtractionLimits) -> Self {
        self.surface_limits = limits;
        self
    }

    /// Sets requested-resource and primitive-packing limits for GPU uploads.
    #[must_use]
    pub fn with_gpu_upload_limits(mut self, limits: crate::GpuUploadLimits) -> Self {
        self.gpu_upload_limits = limits;
        self
    }

    /// Requests another GPU resource attempt after a transient presentation failure.
    ///
    /// Call after freeing resources or recovering from pressure, then request a
    /// host repaint. Cached CPU geometry and successful GPU geometry remain valid.
    /// Camera/background ticks alone do not retry a failed resource allocation.
    pub fn retry_gpu_resources(&mut self) {
        self.resource_retry = self.resource_retry.wrapping_add(1);
    }

    /// Returns shared geometry, rebuilding only changed dataset revisions.
    ///
    /// Scene clones share this cache. Cameras and materials do not invalidate
    /// geometry; safe Arc mutations of positions, topology, scalars or masks do.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid datasets, missing/non-scalar fields or exceeded
    /// face-workspace limits during a new geometry build.
    ///
    /// # Panics
    ///
    /// Panics if a previous preparation panicked while owning the cache lock.
    pub fn prepare(&self) -> Result<Arc<crate::PreparedScene>, crate::SceneError> {
        self.preparation
            .lock()
            .expect("scene preparation poisoned")
            .prepare(self)
    }

    /// Releases cached derived geometry, for example under memory pressure.
    ///
    /// Existing render snapshots remain valid until their owners drop them.
    ///
    /// # Panics
    ///
    /// Panics if a previous preparation poisoned the cache lock.
    pub fn clear_preparation(&self) {
        self.preparation
            .lock()
            .expect("scene preparation poisoned")
            .clear();
    }

    /// Returns geometry build, hit and retained-byte counters.
    ///
    /// # Panics
    ///
    /// Panics if a previous preparation poisoned the cache lock.
    pub fn preparation_stats(&self) -> crate::PreparationStats {
        self.preparation
            .lock()
            .expect("scene preparation poisoned")
            .stats()
    }

    /// Adds a text annotation at normalised frame coordinates.
    #[must_use]
    pub fn with_annotation(mut self, annotation: crate::legend::Annotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Combined bounds of all visible actors.
    #[must_use]
    pub fn bounds(&self) -> Option<gpui_mesh::Bounds> {
        let mut iter = self
            .actors
            .iter()
            .filter(|a| a.visible)
            .filter_map(|a| a.dataset.bounds());
        let first = iter.next()?;
        let mut bounds = first;
        for other in iter {
            for axis in 0..3 {
                bounds.min[axis] = bounds.min[axis].min(other.min[axis]);
                bounds.max[axis] = bounds.max[axis].max(other.max[axis]);
            }
        }
        Some(bounds)
    }
}

pub(crate) fn validate_rgba(color: [f32; 4]) -> Result<(), crate::SceneError> {
    if color
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(crate::SceneError::InvalidMaterial(
            "RGBA components must be finite and in [0,1]",
        ));
    }
    Ok(())
}
