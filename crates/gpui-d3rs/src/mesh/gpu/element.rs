//! Mesh-element state for swapping a decimated proxy during camera drags.
//!
//! The renderer can ask [`MeshLodController::active_mesh`] for the geometry to
//! upload or draw. Proxy construction happens when geometry or the threshold
//! changes, never on a camera frame; settling the drag therefore only changes
//! which retained mesh is selected.

use crate::mesh::{TriangleMesh, decimate::decimate_vertex_clustering};

/// Default triangle count above which the element keeps an interactive proxy.
pub const DEFAULT_LOD_THRESHOLD: usize = 2_000_000;

/// Retained full/proxy mesh selection for an interactive GPU element.
#[derive(Debug, Clone)]
pub struct MeshLodController {
    full_mesh: TriangleMesh,
    proxy_mesh: Option<TriangleMesh>,
    lod_threshold: usize,
    camera_dragging: bool,
}

impl MeshLodController {
    /// Create a controller using [`DEFAULT_LOD_THRESHOLD`].
    pub fn new(mesh: TriangleMesh) -> Self {
        Self::with_lod_threshold(mesh, DEFAULT_LOD_THRESHOLD)
    }

    /// Create a controller with a configurable proxy threshold.
    pub fn with_lod_threshold(mesh: TriangleMesh, lod_threshold: usize) -> Self {
        let proxy_mesh = make_proxy(&mesh, lod_threshold);
        Self {
            full_mesh: mesh,
            proxy_mesh,
            lod_threshold,
            camera_dragging: false,
        }
    }

    /// Replace the source geometry and rebuild its retained proxy.
    pub fn set_mesh(&mut self, mesh: TriangleMesh) {
        self.proxy_mesh = make_proxy(&mesh, self.lod_threshold);
        self.full_mesh = mesh;
        self.camera_dragging = false;
    }

    /// Change the threshold and rebuild the retained proxy once.
    pub fn set_lod_threshold(&mut self, lod_threshold: usize) {
        self.proxy_mesh = make_proxy(&self.full_mesh, lod_threshold);
        self.lod_threshold = lod_threshold;
    }

    /// Mark the beginning of camera manipulation.
    pub fn begin_camera_drag(&mut self) {
        self.camera_dragging = true;
    }

    /// Mark camera manipulation settled; the next frame selects the full mesh.
    pub fn end_camera_drag(&mut self) {
        self.camera_dragging = false;
    }

    /// Set the drag state from the element's pointer lifecycle.
    pub fn set_camera_dragging(&mut self, dragging: bool) {
        self.camera_dragging = dragging;
    }

    /// Geometry selected for the current frame.
    pub fn active_mesh(&self) -> &TriangleMesh {
        if self.camera_dragging {
            self.proxy_mesh.as_ref().unwrap_or(&self.full_mesh)
        } else {
            &self.full_mesh
        }
    }

    /// The canonical geometry, regardless of interaction state.
    pub fn full_mesh(&self) -> &TriangleMesh {
        &self.full_mesh
    }

    /// The retained proxy, if the source exceeds the configured threshold.
    pub fn proxy_mesh(&self) -> Option<&TriangleMesh> {
        self.proxy_mesh.as_ref()
    }

    pub fn lod_threshold(&self) -> usize {
        self.lod_threshold
    }

    pub fn camera_dragging(&self) -> bool {
        self.camera_dragging
    }

    pub fn uses_proxy(&self) -> bool {
        self.camera_dragging && self.proxy_mesh.is_some()
    }
}

fn make_proxy(mesh: &TriangleMesh, lod_threshold: usize) -> Option<TriangleMesh> {
    if lod_threshold == 0 || mesh.triangles.len() <= lod_threshold {
        None
    } else {
        Some(decimate_vertex_clustering(mesh, lod_threshold))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_mesh(width: usize, height: usize) -> TriangleMesh {
        let positions = (0..height)
            .flat_map(|y| (0..width).map(move |x| [x as f64, y as f64, 0.0]))
            .collect::<Vec<_>>();
        let mut triangles = Vec::with_capacity((width - 1) * (height - 1) * 2);
        for y in 0..height - 1 {
            for x in 0..width - 1 {
                let a = (y * width + x) as u32;
                let b = a + 1;
                let c = a + width as u32;
                let d = c + 1;
                triangles.push([a, b, c]);
                triangles.push([b, d, c]);
            }
        }
        TriangleMesh {
            id: "grid".into(),
            positions: positions.into(),
            triangles: triangles.into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn drag_selects_proxy_and_settle_restores_full_mesh() {
        let mesh = grid_mesh(50, 50);
        let full_triangle_count = mesh.triangles.len();
        let mut controller = MeshLodController::with_lod_threshold(mesh, 500);

        assert!(!controller.uses_proxy());
        assert_eq!(
            controller.active_mesh().triangles.len(),
            full_triangle_count
        );

        controller.begin_camera_drag();
        assert!(controller.uses_proxy());
        assert!(controller.active_mesh().triangles.len() < full_triangle_count);

        controller.end_camera_drag();
        assert!(!controller.uses_proxy());
        assert_eq!(
            controller.active_mesh().triangles.len(),
            full_triangle_count
        );
    }

    #[test]
    fn small_mesh_does_not_allocate_a_proxy() {
        let mesh = grid_mesh(4, 4);
        let mut controller = MeshLodController::with_lod_threshold(mesh, 100);
        controller.begin_camera_drag();
        assert!(controller.proxy_mesh().is_none());
        assert!(!controller.uses_proxy());
    }
}
