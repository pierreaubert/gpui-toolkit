//! Retained 2D drawing surfaces rendered with native GPUI primitives.
//!
//! Scene2D accepts a validated, ordered display list with stable object IDs.
//! It paints quads, tessellated paths, and GPUI-shaped text directly into the
//! current frame; it does not use a Vello readback path. One contain-fit
//! transform maps view-box coordinates for both paint and hit testing.
//!
//! Store a clone of `Scene2DState` in the owning view when updates need native
//! transitions or direct input routing. Call `replace_scene` on the UI thread,
//! then notify the owning view. A `GameSurface` can also be constructed from a
//! one-off scene snapshot for static diagrams and custom controls.
//!
//! # Examples
//!
//! ```ignore
//! use gpui_ui_kit::scene2d::{GameSurface, Scene2DNode, Scene2DNodeKind, Scene2DScene, SceneRect};
//!
//! let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 100.0, 100.0));
//! // Add stable-ID nodes to the ordered display list before rendering.
//! let surface = GameSurface::new("board", scene)?;
//! ```

// Rust guideline compliant 2026-02-21

mod animation;
mod geometry;
mod input;
mod painter;
mod types;

#[doc(inline)]
pub use animation::Scene2DState;
#[doc(inline)]
pub use geometry::{Scene2DHit, Scene2DViewTransform};
#[doc(inline)]
pub use input::{
    Scene2DButton, Scene2DInput, Scene2DInputRouter, Scene2DKeyPhase, Scene2DLifecycleReason,
    Scene2DModifier, Scene2DPointerDevice, Scene2DPointerEvent, Scene2DPointerPhase, normalize_key,
};
#[doc(inline)]
pub use painter::GameSurface;
#[doc(inline)]
pub use types::{
    SCENE2D_SCHEMA_VERSION, Scene2DBrush, Scene2DColor, Scene2DEasing, Scene2DGrid,
    Scene2DGridCell, Scene2DInputConfig, Scene2DNode, Scene2DNodeKind, Scene2DPathCommand,
    Scene2DScene, Scene2DSemantic, Scene2DSemanticRole, Scene2DShadow, Scene2DStroke,
    Scene2DTextAlign, Scene2DTransform, Scene2DTransition, Scene2DValidationError, ScenePoint,
    SceneRect,
};
