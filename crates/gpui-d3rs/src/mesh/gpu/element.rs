//! GPUI element for a retained mesh scene.

use super::{MeshSceneState, render_offscreen};
use gpui::*;
use image::Frame;
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;
use std::sync::Arc;

pub struct MeshSceneElement {
    state: Rc<RefCell<MeshSceneState>>,
    custom_id: Option<CustomDrawId>,
}

impl MeshSceneElement {
    pub fn new(state: Rc<RefCell<MeshSceneState>>) -> Self {
        Self {
            state,
            custom_id: None,
        }
    }

    pub fn with_custom_id(mut self, custom_id: CustomDrawId) -> Self {
        self.custom_id = Some(custom_id);
        self
    }

    pub fn state(&self) -> Rc<RefCell<MeshSceneState>> {
        self.state.clone()
    }
}

impl IntoElement for MeshSceneElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for MeshSceneElement {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                Style {
                    size: Size {
                        width: relative(1.0).into(),
                        height: relative(1.0).into(),
                    },
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        if let Some(id) = self.custom_id {
            window.paint_custom(id, bounds);
        } else {
            // Keep the element useful on platforms without a registered
            // zero-copy backend. The fallback is deliberately rendered from
            // the retained upload and painted as one GPUI image.
            let width = f32::from(bounds.size.width).max(1.0) as u32;
            let height = f32::from(bounds.size.height).max(1.0) as u32;
            let image = {
                let state = self.state.borrow();
                render_offscreen(state.upload.as_ref(), &state, width, height)
            };
            let frame = Frame::new(image);
            let render_image = RenderImage::new(vec![frame]);
            let _ =
                window.paint_image(bounds, Corners::default(), Arc::new(render_image), 0, false);
        }
    }
}

/// Default triangle count above which the element keeps an interactive proxy.
pub const DEFAULT_LOD_THRESHOLD: usize = 2_000_000;

/// Retained full/proxy mesh selection for an interactive GPU element.
#[derive(Debug, Clone)]
pub struct MeshLodController {
    full_mesh: crate::mesh::TriangleMesh,
    proxy_mesh: Option<crate::mesh::MeshDecimation>,
    lod_threshold: usize,
    camera_dragging: bool,
}

impl MeshLodController {
    pub fn new(mesh: crate::mesh::TriangleMesh) -> Self {
        Self::with_lod_threshold(mesh, DEFAULT_LOD_THRESHOLD)
    }
    pub fn with_lod_threshold(mesh: crate::mesh::TriangleMesh, lod_threshold: usize) -> Self {
        let proxy_mesh = make_proxy(&mesh, lod_threshold);
        Self {
            full_mesh: mesh,
            proxy_mesh,
            lod_threshold,
            camera_dragging: false,
        }
    }
    pub fn set_mesh(&mut self, mesh: crate::mesh::TriangleMesh) {
        self.proxy_mesh = make_proxy(&mesh, self.lod_threshold);
        self.full_mesh = mesh;
        self.camera_dragging = false;
    }
    pub fn set_lod_threshold(&mut self, lod_threshold: usize) {
        self.proxy_mesh = make_proxy(&self.full_mesh, lod_threshold);
        self.lod_threshold = lod_threshold;
    }
    pub fn begin_camera_drag(&mut self) {
        self.camera_dragging = true;
    }
    pub fn end_camera_drag(&mut self) {
        self.camera_dragging = false;
    }
    pub fn set_camera_dragging(&mut self, dragging: bool) {
        self.camera_dragging = dragging;
    }
    pub fn active_mesh(&self) -> &crate::mesh::TriangleMesh {
        if self.camera_dragging {
            self.proxy_mesh
                .as_ref()
                .map_or(&self.full_mesh, |proxy| &proxy.mesh)
        } else {
            &self.full_mesh
        }
    }
    pub fn full_mesh(&self) -> &crate::mesh::TriangleMesh {
        &self.full_mesh
    }
    pub fn proxy_mesh(&self) -> Option<&crate::mesh::TriangleMesh> {
        self.proxy_mesh.as_ref().map(|proxy| &proxy.mesh)
    }
    /// Return the active proxy provenance while a drag is in progress.
    /// Callers use this to map vertex/cell scalar fields without changing the
    /// public full-resolution picking and selection contract.
    pub fn active_proxy(&self) -> Option<&crate::mesh::MeshDecimation> {
        self.camera_dragging
            .then_some(self.proxy_mesh.as_ref())
            .flatten()
    }
    /// Return a scalar field whose samples match the mesh currently rendered
    /// by this controller. Selection remains against `full_mesh`; this is
    /// only for the temporary drag-time visual proxy.
    pub fn active_field(
        &self,
        field: &crate::mesh::ScalarField,
    ) -> Result<crate::mesh::ScalarField, crate::mesh::MeshValidationError> {
        self.active_proxy().map_or_else(
            || Ok(field.clone()),
            |proxy| proxy.map_field(&self.full_mesh, field),
        )
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

fn make_proxy(
    mesh: &crate::mesh::TriangleMesh,
    threshold: usize,
) -> Option<crate::mesh::MeshDecimation> {
    if threshold == 0 || mesh.triangles.len() <= threshold {
        None
    } else {
        Some(crate::mesh::decimate_vertex_clustering_with_mapping(
            mesh, threshold,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mesh() -> crate::mesh::TriangleMesh {
        crate::mesh::TriangleMesh {
            id: "grid".into(),
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ]
            .into(),
            triangles: vec![[0, 1, 2], [0, 2, 3]].into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }
    #[::core::prelude::v1::test]
    fn lod_controller_restores_full_mesh_after_drag() {
        let mesh = mesh();
        let count = mesh.triangles.len();
        let mut controller = MeshLodController::with_lod_threshold(mesh, 1);
        controller.begin_camera_drag();
        assert!(controller.uses_proxy());
        controller.end_camera_drag();
        assert_eq!(controller.active_mesh().triangles.len(), count);
    }
    #[::core::prelude::v1::test]
    fn small_mesh_does_not_allocate_proxy() {
        let mut controller = MeshLodController::with_lod_threshold(mesh(), 100);
        controller.begin_camera_drag();
        assert!(!controller.uses_proxy());
    }

    #[::core::prelude::v1::test]
    fn lod_controller_maps_the_active_proxy_field() {
        let mut controller = MeshLodController::with_lod_threshold(mesh(), 1);
        let field = crate::mesh::ScalarField {
            id: "field".into(),
            label: "Field".into(),
            unit: None,
            values: vec![0.0, 1.0, 2.0, 3.0].into(),
            association: crate::mesh::ScalarAssociation::Vertex,
            valid: None,
        };
        assert_eq!(
            controller.active_field(&field).unwrap().values,
            field.values
        );
        controller.begin_camera_drag();
        let proxy_field = controller.active_field(&field).unwrap();
        assert_eq!(
            proxy_field.values.len(),
            controller.active_mesh().positions.len()
        );
        assert_eq!(
            proxy_field.association,
            crate::mesh::ScalarAssociation::Vertex
        );
    }
}
