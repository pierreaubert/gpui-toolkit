//! Retained scene specifications for the GPUI Python wrapper.
//!
//! Python owns declarations: stable ids, arrays, cameras, and callbacks.
//! Rust owns validation, retained-resource dirty classification, and the
//! renderer-facing adapters. Raw `wgpu` objects stay behind `gpui-d3rs`.

mod cache;
mod error;
#[cfg(feature = "gpui")]
pub mod gpui_adapter;
mod scene3d;
pub mod ui_ir;

pub use cache::{CacheUpdate, DirtyResources, RetainedSceneCache};
pub use error::Scene3DError;
pub use scene3d::{
    AxisLabels, CameraSpec, ColorRgba, ColormapSpec, GridData, InteractionMode, LightSpec,
    LineSegmentSpec, LineStripSpec, LinesSpec, MaterialSpec, MeshSpec, OrbitCameraSpec,
    PerspectiveCameraSpec, Point3, ScalarRange, SceneNode, SceneSpec, SurfaceSpec, ViewportSize,
};
pub use ui_ir::{PythonAppIr, UiIrError};
