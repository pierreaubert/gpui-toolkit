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
    pub(crate) fn relative_to(self, origin: [f64; 3]) -> Self {
        Self {
            origin: std::array::from_fn(|axis| self.origin[axis] - origin[axis]),
            normal: self.normal,
        }
    }

    /// Signed distance of a point; negative points are clipped away.
    #[must_use]
    pub fn distance(&self, point: [f64; 3]) -> f64 {
        (point[0] - self.origin[0]) * self.normal[0]
            + (point[1] - self.origin[1]) * self.normal[1]
            + (point[2] - self.origin[2]) * self.normal[2]
    }

    /// Shader uniform layout: `[origin.xyz, 0, normal.xyz, 0]` as `f32`.
    #[must_use]
    pub fn to_uniform(&self) -> [f32; 8] {
        [
            self.origin[0] as f32,
            self.origin[1] as f32,
            self.origin[2] as f32,
            0.0,
            self.normal[0] as f32,
            self.normal[1] as f32,
            self.normal[2] as f32,
            0.0,
        ]
    }
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
    /// Whether wireframe edges are drawn over the surface.
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

    /// Toggles the wireframe overlay.
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

impl Scene {
    /// Creates an empty scene with a dark background.
    pub fn new() -> Self {
        Self {
            actors: Vec::new(),
            background: [0.08, 0.09, 0.11, 1.0],
            annotations: Vec::new(),
            preparation: Arc::default(),
        }
    }

    /// Adds an actor.
    #[must_use]
    pub fn with_actor(mut self, actor: Actor) -> Self {
        self.actors.push(actor);
        self
    }

    /// Returns shared geometry, rebuilding only changed dataset revisions.
    ///
    /// Scene clones share this cache. Cameras and materials do not invalidate
    /// geometry; safe Arc mutations of positions, topology, scalars or masks do.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid datasets or missing/non-scalar fields.
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
