//! Retained scene specifications for the GPUI Python wrapper.
//!
//! Python owns declarations: stable ids, arrays, cameras, and callbacks.
//! Rust owns validation, retained-resource dirty classification, and the
//! renderer-facing adapters. Raw `wgpu` objects stay behind `gpui-d3rs`.

pub mod audio_stream;
mod cache;
mod error;
#[cfg(feature = "gpui")]
pub mod gpui_adapter;
mod scene3d;
pub mod session;
#[cfg(feature = "showcase")]
pub mod showcase;
pub mod spec_cache;
pub mod ui_ir;

pub use cache::{CacheUpdate, DirtyResources, RetainedSceneCache};
pub use error::Scene3DError;
pub use scene3d::{
    AxisLabels, CameraSpec, ColorRgba, ColormapSpec, GridData, InteractionMode, LightSpec,
    LineSegmentSpec, LineStripSpec, LinesSpec, MaterialSpec, MeshSpec, OrbitCameraSpec,
    PerspectiveCameraSpec, Point3, ScalarRange, SceneNode, SceneSpec, SurfaceSpec, ViewportSize,
};
pub use session::{
    HostMessage, PYTHON_APP_SESSION_VERSION, PythonMessage, SessionError, SessionState,
};
pub use spec_cache::{
    DEFAULT_TYPED_SPEC_CACHE_MAX_ENTRIES, SCENE3D_SPEC_SCHEMA_VERSION, scene3d_spec_schema_version,
    validate_scene3d_spec_schema_version,
};
pub use ui_ir::{PYTHON_APP_IR_SCHEMA_VERSION, PythonAppIr, UiIrError};
