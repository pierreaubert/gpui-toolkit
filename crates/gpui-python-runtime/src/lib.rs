#![forbid(unsafe_code)]

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
#[cfg(feature = "showcase")]
pub mod native_mesh_plot;
pub mod mesh_frames;
pub mod meshplot;
mod scene3d;
pub mod session;
#[cfg(feature = "showcase")]
pub mod showcase;
pub mod spec_cache;
pub mod ui_ir;

pub use cache::{CacheUpdate, DirtyResources, RetainedSceneCache};
pub use error::Scene3DError;
pub use mesh_frames::{
    MAX_MESH_FRAME_BYTES, MAX_MESH_RESOURCE_BYTES, MeshDtype, MeshFrame, MeshFrameError,
    MeshFrameKind, MeshFrameOutcome, MeshFrameStats, MeshFrameStore, RetainedMeshResource,
};
pub use meshplot::{MESHPLOT_SPEC_SCHEMA_VERSION, MeshPlotSpec};
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
pub use ui_ir::{MeshPlotNode, PYTHON_APP_IR_SCHEMA_VERSION, PythonAppIr, UiIrError};
