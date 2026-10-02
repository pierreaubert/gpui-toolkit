#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

//! Retained scene specifications for the GPUI Python wrapper.
//!
//! Python owns declarations: stable ids, arrays, cameras, and callbacks.
//! Rust owns validation, retained-resource dirty classification, and the
//! renderer-facing adapters. Raw `wgpu` objects stay behind `gpui-d3rs`.

pub mod audio_stream;
mod cache;
pub mod dataset_frames;
mod error;
#[cfg(feature = "gpui")]
pub mod gpui_adapter;
pub mod mesh_frames;
pub mod meshplot;
#[cfg(feature = "showcase")]
pub mod native_mesh_plot;
pub mod scene2d;
mod scene3d;
pub mod session;
#[cfg(feature = "showcase")]
pub mod showcase;
pub mod spec_cache;
pub mod ui_ir;

/// The wheel's private abi3 extension. It exposes pure synchronous work only;
/// GPUI windows and rendering remain host-owned.
#[cfg(feature = "python-extension")]
mod python_extension;

pub use cache::{CacheUpdate, DirtyResources, RetainedSceneCache};
pub use error::Scene3DError;
pub use mesh_frames::{
    MAX_MESH_FRAME_BYTES, MAX_MESH_RESOURCE_BYTES, MeshDtype, MeshFrame, MeshFrameError,
    MeshFrameKind, MeshFrameOutcome, MeshFrameStats, MeshFrameStore, RetainedMeshResource,
};
pub use meshplot::{MESHPLOT_SPEC_SCHEMA_VERSION, MeshPlotSpec};
pub use scene2d::{
    GridSpec, SCENE2D_SCHEMA_VERSION, Scene2DBrush, Scene2DCache, Scene2DCacheUpdate, Scene2DColor,
    Scene2DDevice, Scene2DEasing, Scene2DError, Scene2DGridCell, Scene2DInput, Scene2DInputConfig,
    Scene2DKeyPhase, Scene2DLifecycleReason, Scene2DModifier, Scene2DNode, Scene2DNodeKind,
    Scene2DPatch, Scene2DPathCommand, Scene2DPointerButton, Scene2DPointerPhase, Scene2DScene,
    Scene2DSemantic, Scene2DSemanticRole, Scene2DShadow, Scene2DStroke, Scene2DTextAlign,
    Scene2DTransform, Scene2DTransition, ScenePoint, SceneRect, scene2d_schema_version,
};
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
