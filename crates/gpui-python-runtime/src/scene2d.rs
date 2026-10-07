//! Validated `Scene2D` data and revisioned transactions for Python-authored apps.
//!
//! The wire model is renderer-independent. Hosts validate complete snapshots
//! and patches before publishing them to a native surface. Object IDs remain
//! stable across patches, and failed updates leave the committed scene intact.
//!
//! # Examples
//!
//! ```
//! use gpui_python_runtime::scene2d::{Scene2DCache, Scene2DScene};
//! let scene: Scene2DScene = serde_json::from_str(r#"{
//!   "version":1,"revision":1,"view_box":{"x":0,"y":0,"width":10,"height":10},
//!   "nodes":[],"grid":null,
//!   "input":{"pointer":true,"continuous":false,"capture":true,"keyboard":true}
//! }"#)?;
//! let mut cache = Scene2DCache::default();
//! cache.replace(scene)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Current version of the Python-to-native `Scene2D` schema.
pub const SCENE2D_SCHEMA_VERSION: u16 = 1;

// These limits bound retained state and expensive per-frame native work.
const MAX_NODES: usize = 65_536;
const MAX_PATH_COMMANDS: usize = 65_536;
const MAX_ID_BYTES: usize = 256;
const MAX_TEXT_BYTES: usize = 1_048_576;
const MAX_GRID_CELLS: usize = 1_000_000;
const MAX_DIMENSION: f32 = 1_000_000.0;
const MAX_GROUP_DEPTH: usize = 64;

/// Errors found while decoding, validating, or applying a `Scene2D` update.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum Scene2DError {
    /// The snapshot uses a different schema version.
    #[error("unsupported scene2d schema version {received}; supported version is {supported}")]
    UnsupportedVersion { received: u16, supported: u16 },
    /// A required scene or node identifier is empty or too long.
    #[error("scene2d {field} must contain 1..={max_bytes} bytes")]
    InvalidId {
        field: &'static str,
        max_bytes: usize,
    },
    /// Two display-list objects use the same stable ID.
    #[error("scene2d node id {id:?} appears more than once")]
    DuplicateNodeId { id: String },
    /// A geometric or timing value is invalid.
    #[error("scene2d {field} is outside the supported range")]
    InvalidValue { field: &'static str },
    /// A color channel is outside the normalized zero-to-one range.
    #[error("scene2d color channel {channel} must be in 0..=1")]
    InvalidColor { channel: &'static str },
    /// A scene exceeds a documented collection limit.
    #[error("scene2d {field} count {count} exceeds limit {limit}")]
    LimitExceeded {
        field: &'static str,
        count: usize,
        limit: usize,
    },
    /// A path has no start command or fewer than two instructions.
    #[error("scene2d path {id:?} must start with move_to and contain drawable commands")]
    InvalidPath { id: String },
    /// Text must stay on one line because the v1 native painter shapes lines.
    #[error("scene2d text node {id:?} contains a line break; v1 text supports one line")]
    MultilineText { id: String },
    /// A grid has invalid dimensions or cell geometry.
    #[error("scene2d grid dimensions or cell geometry are invalid")]
    InvalidGrid,
    /// A complete replacement is older than the committed scene.
    #[error("scene2d revision {received} is stale; current revision is {current}")]
    StaleRevision { received: u64, current: u64 },
    /// A delta patch targets a different committed base revision.
    #[error("scene2d patch expects base revision {expected}; current revision is {current}")]
    RevisionConflict { expected: u64, current: u64 },
    /// A delta patch skips a required revision.
    #[error("scene2d patch revision {received} must immediately follow {base}")]
    RevisionGap { base: u64, received: u64 },
    /// A patch removes an unknown stable node ID.
    #[error("scene2d patch removes unknown node id {id:?}")]
    UnknownNode { id: String },
    /// A patch removes and upserts the same stable node ID.
    #[error("scene2d patch both removes and upserts node id {id:?}")]
    RemoveAndUpsert { id: String },
    /// A patch arrived before the first complete scene.
    #[error("scene2d patch arrived before the first scene snapshot")]
    MissingSnapshot,
}

/// A point in the scene's logical coordinate space.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScenePoint {
    /// Horizontal coordinate.
    pub x: f32,
    /// Vertical coordinate.
    pub y: f32,
}

/// A rectangle in the scene's logical coordinate space.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneRect {
    /// Left coordinate.
    pub x: f32,
    /// Top coordinate.
    pub y: f32,
    /// Rectangle width.
    pub width: f32,
    /// Rectangle height.
    pub height: f32,
}

/// A normalized red, green, blue, and alpha color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Scene2DColor {
    /// Red channel in `0.0..=1.0`.
    pub r: f32,
    /// Green channel in `0.0..=1.0`.
    pub g: f32,
    /// Blue channel in `0.0..=1.0`.
    pub b: f32,
    /// Alpha channel in `0.0..=1.0`.
    pub a: f32,
}

/// A fill brush supported by the first `Scene2D` schema.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DBrush {
    /// A single solid color.
    Solid {
        /// Color applied to a filled shape.
        color: Scene2DColor,
    },
    /// A clockwise two-stop gradient whose zero degrees points up.
    LinearGradient {
        /// Direction in degrees.
        angle_degrees: f32,
        /// Gradient start color.
        from: Scene2DColor,
        /// Gradient end color.
        to: Scene2DColor,
    },
}

/// A stroked outline for a path or shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DStroke {
    /// Stroke width in logical units.
    pub width: f32,
    /// Stroke color.
    pub color: Scene2DColor,
}

/// A grid that maps logical positions to row and column identities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridSpec {
    /// Number of rows.
    pub rows: u32,
    /// Number of columns.
    pub columns: u32,
    /// Left edge of the first cell.
    pub x: f32,
    /// Top edge of the first cell.
    pub y: f32,
    /// Width of each cell.
    pub cell_width: f32,
    /// Height of each cell.
    pub cell_height: f32,
    /// Empty logical distance between adjacent cells.
    pub gap: f32,
    /// Optional row names used by accessibility adapters.
    #[serde(default)]
    pub row_labels: Vec<String>,
    /// Optional column names used by accessibility adapters.
    #[serde(default)]
    pub column_labels: Vec<String>,
}

/// A semantic role used by native accessibility adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DSemanticRole {
    /// A generic named group.
    Group,
    /// A table-like grid.
    Grid,
    /// A row within a grid.
    Row,
    /// A cell within a grid.
    GridCell,
    /// An actionable element.
    Button,
    /// A named image or illustration.
    Image,
    /// A live status message.
    Status,
}

/// Accessible name and state attached to a scene or object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DSemantic {
    /// Platform-neutral accessibility role.
    pub role: Scene2DSemanticRole,
    /// Accessible name.
    pub label: String,
    /// Longer accessible description.
    #[serde(default)]
    pub description: Option<String>,
    /// Current value announced by accessibility tools.
    #[serde(default)]
    pub value_text: Option<String>,
    /// Whether the object is selected.
    #[serde(default)]
    pub selected: Option<bool>,
    /// Whether the object is disabled.
    #[serde(default)]
    pub disabled: Option<bool>,
}

/// A native drawing transform applied to a node.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DTransform {
    /// Horizontal translation in logical units.
    pub translate_x: f32,
    /// Vertical translation in logical units.
    pub translate_y: f32,
    /// Horizontal scale factor.
    pub scale_x: f32,
    /// Vertical scale factor.
    pub scale_y: f32,
    /// Rotation in degrees.
    pub rotation_degrees: f32,
    /// Horizontal shear used when nested group transforms compose.
    #[serde(default)]
    pub shear_x: f32,
}

impl Default for Scene2DTransform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            rotation_degrees: 0.0,
            shear_x: 0.0,
        }
    }
}

impl Scene2DTransform {
    fn identity() -> Self {
        Self::default()
    }

    fn linear(self) -> [f32; 4] {
        let angle = self.rotation_degrees.to_radians();
        let (sin, cos) = angle.sin_cos();
        [
            cos * self.scale_x,
            cos * self.shear_x - sin * self.scale_y,
            sin * self.scale_x,
            sin * self.shear_x + cos * self.scale_y,
        ]
    }

    fn compose(parent: Self, child: Self) -> Self {
        let [pa, pb, pc, pd] = parent.linear();
        let [ca, cb, cc, cd] = child.linear();
        let a = pa * ca + pb * cc;
        let b = pa * cb + pb * cd;
        let c = pc * ca + pd * cc;
        let d = pc * cb + pd * cd;
        let scale_x = (a * a + c * c).sqrt();
        let rotation = c.atan2(a);
        let shear_x = (a * b + c * d) / scale_x;
        let scale_y = (a * d - b * c) / scale_x;
        Self {
            translate_x: pa * child.translate_x + pb * child.translate_y + parent.translate_x,
            translate_y: pc * child.translate_x + pd * child.translate_y + parent.translate_y,
            scale_x,
            scale_y,
            shear_x,
            rotation_degrees: rotation.to_degrees(),
        }
    }

    fn is_valid(self) -> bool {
        [
            self.translate_x,
            self.translate_y,
            self.scale_x,
            self.scale_y,
            self.shear_x,
            self.rotation_degrees,
        ]
        .into_iter()
        .all(f32::is_finite)
            && self.scale_x.abs() > f32::EPSILON
            && self.scale_y.abs() > f32::EPSILON
            && self.linear().into_iter().all(f32::is_finite)
    }
}

/// Easing curve used by a native visual transition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DEasing {
    Linear,
    EaseOutQuad,
    #[default]
    EaseOutCubic,
    EaseInOutCubic,
    EaseOutBack,
    EaseOutBounce,
}

/// Target-property transition requested for a changed scene object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DTransition {
    /// Transition duration in milliseconds.
    pub duration_ms: u64,
    /// Easing curve.
    #[serde(default)]
    pub easing: Scene2DEasing,
    /// Optional stable completion token emitted by the native surface once.
    #[serde(default)]
    pub completion_id: Option<String>,
    /// Interpolate supported paint colors alongside transform and opacity.
    #[serde(default)]
    pub animate_color: bool,
    /// Reveal a path from its start to end over this transition.
    #[serde(default)]
    pub reveal_path: bool,
}

/// A bounded, single-layer shadow behind a drawable primitive.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DShadow {
    /// Horizontal offset in local logical units.
    pub offset_x: f32,
    /// Vertical offset in local logical units.
    pub offset_y: f32,
    /// Blur radius in local logical units.
    pub blur_radius: f32,
    /// Shadow color.
    pub color: Scene2DColor,
}

/// Text alignment used by a `Scene2D` text object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DTextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// A path drawing instruction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DPathCommand {
    /// Begin a subpath at `point`.
    MoveTo {
        /// New subpath origin.
        point: ScenePoint,
    },
    /// Add a straight line to `point`.
    LineTo {
        /// Line endpoint.
        point: ScenePoint,
    },
    /// Add a quadratic Bézier segment.
    QuadraticTo {
        /// Single control point.
        control: ScenePoint,
        /// Segment endpoint.
        point: ScenePoint,
    },
    /// Add a cubic Bézier segment.
    CubicTo {
        /// First control point.
        control_a: ScenePoint,
        /// Second control point.
        control_b: ScenePoint,
        /// Segment endpoint.
        point: ScenePoint,
    },
    /// Close the current subpath.
    Close,
}

/// A supported native drawing primitive.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DNodeKind {
    /// A small ordered transform/opacity group with inherited clipping.
    Group {
        /// Child drawables in back-to-front order.
        children: Vec<Scene2DNode>,
    },
    /// An axis-aligned rectangle.
    Rect {
        /// Rectangle bounds.
        rect: SceneRect,
        /// Optional fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional outline.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// A rectangle with rounded corners.
    RoundedRect {
        /// Rectangle bounds.
        rect: SceneRect,
        /// Corner radius in logical units.
        radius: f32,
        /// Optional fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional outline.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// A circle.
    Circle {
        /// Circle center.
        center: ScenePoint,
        /// Circle radius in logical units.
        radius: f32,
        /// Optional fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional outline.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// A straight line segment.
    Line {
        /// Segment start.
        start: ScenePoint,
        /// Segment end.
        end: ScenePoint,
        /// Required line outline.
        stroke: Scene2DStroke,
    },
    /// A multi-segment path.
    Path {
        /// Ordered path instructions.
        commands: Vec<Scene2DPathCommand>,
        /// Optional fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional outline.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// Shaped text at a logical origin.
    Text {
        /// Text origin.
        origin: ScenePoint,
        /// Text content.
        content: String,
        /// Font size in logical units.
        size: f32,
        /// Text color.
        color: Scene2DColor,
        /// Optional font family name.
        #[serde(default)]
        font: Option<String>,
        /// Horizontal text alignment.
        #[serde(default)]
        align: Scene2DTextAlign,
    },
}

/// A stable display-list object and its optional semantic hit identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DNode {
    /// Stable object identity used by patches.
    pub id: String,
    /// Semantic identity emitted by native hit testing.
    #[serde(default)]
    pub hit_id: Option<String>,
    /// Optional local bounds used for hit testing independently of paint geometry.
    #[serde(default)]
    pub hit_bounds: Option<SceneRect>,
    /// Local clip rectangle applied to this node or inherited by a group.
    #[serde(default)]
    pub clip: Option<SceneRect>,
    /// Optional shadow supported for primitive nodes.
    #[serde(default)]
    pub shadow: Option<Scene2DShadow>,
    /// Drawing primitive and geometry.
    pub kind: Scene2DNodeKind,
    /// Opacity in the range `0.0..=1.0`; defaults to fully opaque.
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    /// Translation, scale, and rotation; defaults to the identity transform.
    #[serde(default)]
    pub transform: Scene2DTransform,
    /// Transition used when this target object changes.
    #[serde(default)]
    pub transition: Option<Scene2DTransition>,
    /// Accessible role, name, and state.
    #[serde(default)]
    pub semantic: Option<Scene2DSemantic>,
}

fn default_opacity() -> f32 {
    1.0
}

/// Pointer buttons normalized across mouse and direct-contact devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DPointerButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
}

/// Keyboard modifier normalized across desktop platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DModifier {
    Shift,
    Control,
    Alt,
    Meta,
    Fn,
}

/// Pointer phase emitted by a direct or mouse pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DPointerPhase {
    Down,
    Move,
    Up,
    Cancel,
}

/// Pointer device kind normalized across supported platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DDevice {
    Mouse,
    Touch,
    Pen,
}

/// Keyboard phase emitted while the surface owns focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DKeyPhase {
    Down,
    Up,
}

/// A grid cell resolved by native hit testing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DGridCell {
    /// Zero-based row.
    pub row: u32,
    /// Zero-based column.
    pub column: u32,
    /// Row-major zero-based cell index.
    pub index: u32,
    /// Stable cell name in the form `r{row}c{column}`.
    pub id: String,
}

/// Why a `Scene2D` surface cleared held keys or pointer contacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DLifecycleReason {
    /// The surface lost keyboard focus.
    FocusLost,
    /// Its containing window or application was suspended.
    Suspended,
    /// The surface was removed from its view.
    Removed,
    /// The active app section changed.
    SectionChanged,
    /// The Python session disconnected.
    Disconnected,
    /// The containing window or application became active again.
    Resumed,
}

/// Input event normalized by a native `Scene2D` surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DInput {
    /// Pointer contact or mouse event with semantic hit results.
    Pointer {
        /// Pointer phase.
        phase: Scene2DPointerPhase,
        /// Input device.
        device: Scene2DDevice,
        /// Stable identifier for this contact.
        contact_id: u64,
        /// Monotonic event timestamp in nanoseconds.
        timestamp_ns: u64,
        /// Position in scene logical coordinates.
        position: ScenePoint,
        /// Buttons held at sampling time.
        #[serde(default)]
        buttons: Vec<Scene2DPointerButton>,
        /// Active keyboard modifiers.
        #[serde(default)]
        modifiers: Vec<Scene2DModifier>,
        /// Stable semantic object hit by this event.
        #[serde(default)]
        hit_id: Option<String>,
        /// Grid cell hit by this event.
        #[serde(default)]
        cell: Option<Scene2DGridCell>,
    },
    /// Key event delivered while the surface owns focus.
    Key {
        /// Keyboard phase.
        phase: Scene2DKeyPhase,
        /// Normalized key name.
        key: String,
        /// Whether this is an operating-system repeat event.
        #[serde(default)]
        repeat: bool,
        /// Active keyboard modifiers.
        #[serde(default)]
        modifiers: Vec<Scene2DModifier>,
        /// Monotonic event timestamp in nanoseconds.
        timestamp_ns: u64,
    },
    /// Accessibility activation by a semantic hit target.
    Activate {
        /// Stable `Scene2D` node ID selected by the platform accessibility API.
        id: String,
        /// Optional application-provided semantic hit ID.
        #[serde(default)]
        hit_id: Option<String>,
        /// Optional resolved grid cell.
        #[serde(default)]
        cell: Option<Scene2DGridCell>,
        /// Monotonic activation timestamp in nanoseconds.
        timestamp_ns: u64,
    },
    /// Lifecycle notification that clears held keys and captured contacts.
    Lifecycle {
        /// Why held input state must be cleared.
        reason: Scene2DLifecycleReason,
        /// Monotonic event timestamp in nanoseconds.
        timestamp_ns: u64,
    },
    /// One requested native transition reached its target values.
    TransitionComplete {
        /// Stable ID of the drawable node whose transition completed.
        id: String,
        /// Application-provided completion token.
        completion_id: String,
        /// Monotonic event timestamp in nanoseconds.
        timestamp_ns: u64,
    },
}

/// Input behavior requested by a scene owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DInputConfig {
    /// Whether pointer down, up, and cancel events are delivered.
    #[serde(default = "default_true")]
    pub pointer: bool,
    /// Whether pointer move samples are delivered.
    #[serde(default)]
    pub continuous: bool,
    /// Whether active contacts are captured to the surface.
    #[serde(default = "default_true")]
    pub capture: bool,
    /// Whether the surface accepts focused key events.
    #[serde(default = "default_true")]
    pub keyboard: bool,
}

impl Default for Scene2DInputConfig {
    fn default() -> Self {
        Self {
            pointer: true,
            continuous: false,
            capture: true,
            keyboard: true,
        }
    }
}
fn default_true() -> bool {
    true
}

/// A validated ordered display list ready for native rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DScene {
    /// `Scene2D` schema version.
    #[serde(default = "scene2d_schema_version")]
    pub version: u16,
    /// Monotonically increasing scene revision.
    pub revision: u64,
    /// Optional accessible role and label for the complete surface.
    #[serde(default)]
    pub semantic: Option<Scene2DSemantic>,
    /// Optional scene-wide background brush.
    #[serde(default)]
    pub background: Option<Scene2DBrush>,
    /// Logical bounds shared by painting and hit testing.
    pub view_box: SceneRect,
    /// Ordered objects, from back to front.
    #[serde(default)]
    pub nodes: Vec<Scene2DNode>,
    /// Optional grid hit-test geometry.
    #[serde(default)]
    pub grid: Option<GridSpec>,
    /// Focused keyboard and pointer settings.
    #[serde(default)]
    pub input: Scene2DInputConfig,
}

/// Return the current `Scene2D` schema version.
#[must_use]
pub const fn scene2d_schema_version() -> u16 {
    SCENE2D_SCHEMA_VERSION
}

impl Scene2DScene {
    /// Validate scene metadata, every node, and renderer-facing values.
    ///
    /// # Errors
    /// Returns an error when a version, stable ID, geometry, paint value, or
    /// bounded collection is invalid.
    pub fn validate(&self) -> Result<(), Scene2DError> {
        if self.version != SCENE2D_SCHEMA_VERSION {
            return Err(Scene2DError::UnsupportedVersion {
                received: self.version,
                supported: SCENE2D_SCHEMA_VERSION,
            });
        }
        validate_rect(&self.view_box, "view_box", true)?;
        validate_semantic(self.semantic.as_ref())?;
        validate_brush(self.background.as_ref())?;
        validate_grid(self.grid.as_ref())?;
        let mut ids = HashSet::with_capacity(self.nodes.len());
        let mut node_count = 0;
        for node in &self.nodes {
            validate_node(
                node,
                0,
                Scene2DTransform::identity(),
                &mut node_count,
                &mut ids,
            )?;
        }
        Ok(())
    }
}

/// A revisioned patch of objects and optional scene metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DPatch {
    /// Revision the patch was authored against.
    pub base_revision: u64,
    /// New scene revision after the transaction commits.
    pub revision: u64,
    /// Objects to insert or replace by stable ID.
    #[serde(default)]
    pub upsert: Vec<Scene2DNode>,
    /// Stable IDs to remove.
    #[serde(default)]
    pub remove: Vec<String>,
    /// Replacement logical view bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_box: Option<SceneRect>,
    /// Replacement grid; `null` explicitly removes the grid.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_replacement"
    )]
    pub grid: Option<Option<GridSpec>>,
    /// Replacement input configuration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<Scene2DInputConfig>,
    /// Replacement surface semantics; `null` explicitly removes it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_replacement"
    )]
    pub semantic: Option<Option<Scene2DSemantic>>,
    /// Replacement background; `null` explicitly removes it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_replacement"
    )]
    pub background: Option<Option<Scene2DBrush>>,
}

// Double Option is serde's missing-vs-null idiom for patch fields; the shape
// is load-bearing and cannot be flattened without changing the data model.
#[allow(clippy::option_option)]
fn deserialize_optional_replacement<'de, D, T>(
    deserializer: D,
) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// A committed patch summary for retained renderers and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene2DCacheUpdate {
    /// Revision committed by this update.
    pub revision: u64,
    /// IDs inserted by the patch.
    pub inserted: Vec<String>,
    /// IDs updated without changing their display-list position.
    pub updated: Vec<String>,
    /// IDs removed by the patch.
    pub removed: Vec<String>,
}

/// Retains the last valid scene and applies ID-based patches atomically.
#[derive(Debug, Clone, Default)]
pub struct Scene2DCache {
    scene: Option<Scene2DScene>,
    node_indices: HashMap<String, usize>,
}

impl Scene2DCache {
    /// Return the most recent valid scene snapshot.
    #[must_use]
    pub fn scene(&self) -> Option<&Scene2DScene> {
        self.scene.as_ref()
    }

    /// Return the cached display-list index for a stable node ID.
    #[must_use]
    pub fn node_index(&self, id: &str) -> Option<usize> {
        self.node_indices.get(id).copied()
    }

    /// Validate and commit a complete scene snapshot.
    ///
    /// # Errors
    /// Returns an error if the snapshot is invalid or older than the current
    /// revision. The previous valid snapshot remains available on error.
    pub fn replace(&mut self, scene: Scene2DScene) -> Result<(), Scene2DError> {
        scene.validate()?;
        if let Some(current) = self.scene.as_ref()
            && scene.revision <= current.revision
        {
            return Err(Scene2DError::StaleRevision {
                received: scene.revision,
                current: current.revision,
            });
        }
        let indices = node_indices(&scene.nodes);
        self.scene = Some(scene);
        self.node_indices = indices;
        Ok(())
    }

    /// Apply one delta transaction while preserving the last valid scene.
    ///
    /// Existing IDs keep their position and newly inserted IDs append in patch
    /// order. The cache changes only after the staged scene validates.
    ///
    /// # Errors
    /// Returns an error for revision conflicts, unknown removals, duplicate
    /// IDs, or invalid replacement values.
    pub fn apply_patch(
        &mut self,
        patch: &Scene2DPatch,
    ) -> Result<Scene2DCacheUpdate, Scene2DError> {
        let current = self.scene.as_ref().ok_or(Scene2DError::MissingSnapshot)?;
        if patch.base_revision != current.revision {
            return Err(Scene2DError::RevisionConflict {
                expected: patch.base_revision,
                current: current.revision,
            });
        }
        if current.revision.checked_add(1) != Some(patch.revision) {
            return Err(Scene2DError::RevisionGap {
                base: patch.base_revision,
                received: patch.revision,
            });
        }

        let current_ids = node_indices(&current.nodes);
        let mut removed = HashSet::with_capacity(patch.remove.len());
        for id in &patch.remove {
            validate_id(id, "removed node id")?;
            if !removed.insert(id.as_str()) {
                return Err(Scene2DError::DuplicateNodeId { id: id.clone() });
            }
            if !current_ids.contains_key(id) {
                return Err(Scene2DError::UnknownNode { id: id.clone() });
            }
        }
        let mut upserted = HashSet::<String>::with_capacity(patch.upsert.len());
        let mut inserted = Vec::new();
        let mut updated = Vec::new();
        for node in &patch.upsert {
            let mut count = 0;
            let mut subtree = HashSet::new();
            validate_node(
                node,
                0,
                Scene2DTransform::identity(),
                &mut count,
                &mut subtree,
            )?;
            let mut ids = Vec::with_capacity(count);
            collect_node_ids(std::slice::from_ref(node), &mut ids);
            for id in ids {
                if !upserted.insert(id.clone()) {
                    return Err(Scene2DError::DuplicateNodeId { id });
                }
                if removed.contains(id.as_str()) {
                    return Err(Scene2DError::RemoveAndUpsert { id });
                }
                if current_ids.contains_key(&id) {
                    updated.push(id);
                } else {
                    inserted.push(id);
                }
            }
        }

        // Removing a group also removes its descendants. Do not make the
        // same transaction implicitly resurrect one of those descendants.
        let removed_tree_ids = removed_subtree_ids(&current.nodes, &removed);
        if let Some(id) = upserted
            .iter()
            .find(|id| removed_tree_ids.contains(id.as_str()))
        {
            return Err(Scene2DError::RemoveAndUpsert { id: id.clone() });
        }

        let mut next = current.clone();
        next.revision = patch.revision;
        if let Some(value) = patch.view_box {
            next.view_box = value;
        }
        if let Some(value) = &patch.grid {
            next.grid.clone_from(value);
        }
        if let Some(value) = patch.input {
            next.input = value;
        }
        if let Some(value) = &patch.semantic {
            next.semantic.clone_from(value);
        }
        if let Some(value) = patch.background {
            next.background = value;
        }
        remove_node_ids(&mut next.nodes, &removed);
        for node in &patch.upsert {
            if let Some(existing) = find_node_mut(&mut next.nodes, &node.id) {
                *existing = node.clone();
            } else {
                next.nodes.push(node.clone());
            }
        }
        next.validate()?;
        let indices = node_indices(&next.nodes);
        let next_ids: HashSet<_> = indices.keys().map(String::as_str).collect();
        let removed_ids = current_ids
            .keys()
            .filter(|id| !next_ids.contains(id.as_str()))
            .cloned()
            .collect();
        self.scene = Some(next);
        self.node_indices = indices;
        Ok(Scene2DCacheUpdate {
            revision: patch.revision,
            inserted,
            updated,
            removed: removed_ids,
        })
    }
}

fn validate_node(
    node: &Scene2DNode,
    group_depth: usize,
    parent_transform: Scene2DTransform,
    count: &mut usize,
    ids: &mut HashSet<String>,
) -> Result<(), Scene2DError> {
    *count += 1;
    if *count > MAX_NODES {
        return Err(Scene2DError::LimitExceeded {
            field: "nodes",
            count: *count,
            limit: MAX_NODES,
        });
    }
    validate_id(&node.id, "node id")?;
    if !ids.insert(node.id.clone()) {
        return Err(Scene2DError::DuplicateNodeId {
            id: node.id.clone(),
        });
    }
    if let Some(id) = node.hit_id.as_deref() {
        validate_id(id, "hit id")?;
    }
    if let Some(bounds) = node.hit_bounds {
        validate_rect(&bounds, "hit bounds", true)?;
    }
    if let Some(clip) = node.clip {
        validate_rect(&clip, "clip bounds", true)?;
    }
    if !node.opacity.is_finite() || !(0.0..=1.0).contains(&node.opacity) {
        return Err(Scene2DError::InvalidValue { field: "opacity" });
    }
    validate_transform(&node.transform)?;
    let composed_transform = Scene2DTransform::compose(parent_transform, node.transform);
    if !composed_transform.is_valid() {
        return Err(Scene2DError::InvalidValue {
            field: "composed transform",
        });
    }
    if let Some(bounds) = node.hit_bounds
        && !transformed_bounds_are_finite(bounds, composed_transform)
    {
        return Err(Scene2DError::InvalidValue {
            field: "transformed hit bounds",
        });
    }
    if let Some(clip) = node.clip
        && !transformed_bounds_are_finite(clip, composed_transform)
    {
        return Err(Scene2DError::InvalidValue {
            field: "transformed clip bounds",
        });
    }
    if let Some(transition) = &node.transition {
        if transition.duration_ms > 60_000 {
            return Err(Scene2DError::InvalidValue {
                field: "transition duration",
            });
        }
        if let Some(id) = transition.completion_id.as_deref() {
            validate_id(id, "transition completion id")?;
        }
    }
    validate_semantic(node.semantic.as_ref())?;
    if let Some(shadow) = node.shadow {
        for (value, field) in [
            (shadow.offset_x, "shadow offset"),
            (shadow.offset_y, "shadow offset"),
        ] {
            finite(value, field, -MAX_DIMENSION, MAX_DIMENSION)?;
        }
        finite(shadow.blur_radius, "shadow blur radius", 0.0, 128.0)?;
        validate_color(shadow.color)?;
        if matches!(node.kind, Scene2DNodeKind::Group { .. }) {
            return Err(Scene2DError::InvalidValue {
                field: "group shadow",
            });
        }
    }
    if let Scene2DNodeKind::Group { children } = &node.kind {
        if group_depth >= MAX_GROUP_DEPTH {
            return Err(Scene2DError::LimitExceeded {
                field: "group depth",
                count: group_depth + 1,
                limit: MAX_GROUP_DEPTH,
            });
        }
        if children.is_empty() {
            return Err(Scene2DError::InvalidValue {
                field: "empty group",
            });
        }
        for child in children {
            validate_node(child, group_depth + 1, composed_transform, count, ids)?;
        }
    } else {
        validate_kind(node)?;
        if let Some(bounds) = kind_bounds(&node.kind)
            && !transformed_bounds_are_finite(bounds, composed_transform)
        {
            return Err(Scene2DError::InvalidValue {
                field: "transformed geometry",
            });
        }
    }
    Ok(())
}

fn node_indices(nodes: &[Scene2DNode]) -> HashMap<String, usize> {
    fn visit(nodes: &[Scene2DNode], next: &mut usize, result: &mut HashMap<String, usize>) {
        for node in nodes {
            result.insert(node.id.clone(), *next);
            *next += 1;
            if let Scene2DNodeKind::Group { children } = &node.kind {
                visit(children, next, result);
            }
        }
    }
    let mut result = HashMap::new();
    visit(nodes, &mut 0, &mut result);
    result
}

fn collect_node_ids(nodes: &[Scene2DNode], result: &mut Vec<String>) {
    for node in nodes {
        result.push(node.id.clone());
        if let Scene2DNodeKind::Group { children } = &node.kind {
            collect_node_ids(children, result);
        }
    }
}

fn find_node_mut<'a>(nodes: &'a mut [Scene2DNode], id: &str) -> Option<&'a mut Scene2DNode> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Scene2DNodeKind::Group { children } = &mut node.kind
            && let Some(found) = find_node_mut(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn remove_node_ids(nodes: &mut Vec<Scene2DNode>, remove: &HashSet<&str>) {
    nodes.retain_mut(|node| {
        if remove.contains(node.id.as_str()) {
            return false;
        }
        if let Scene2DNodeKind::Group { children } = &mut node.kind {
            remove_node_ids(children, remove);
        }
        true
    });
}

fn removed_subtree_ids(nodes: &[Scene2DNode], remove: &HashSet<&str>) -> HashSet<String> {
    fn collect(node: &Scene2DNode, ids: &mut HashSet<String>) {
        ids.insert(node.id.clone());
        if let Scene2DNodeKind::Group { children } = &node.kind {
            for child in children {
                collect(child, ids);
            }
        }
    }
    fn visit(nodes: &[Scene2DNode], remove: &HashSet<&str>, ids: &mut HashSet<String>) {
        for node in nodes {
            if remove.contains(node.id.as_str()) {
                collect(node, ids);
            } else if let Scene2DNodeKind::Group { children } = &node.kind {
                visit(children, remove, ids);
            }
        }
    }
    let mut ids = HashSet::new();
    visit(nodes, remove, &mut ids);
    ids
}

fn validate_id(id: &str, field: &'static str) -> Result<(), Scene2DError> {
    if id.trim().is_empty() || id.len() > MAX_ID_BYTES {
        return Err(Scene2DError::InvalidId {
            field,
            max_bytes: MAX_ID_BYTES,
        });
    }
    Ok(())
}
fn finite(value: f32, field: &'static str, min: f32, max: f32) -> Result<(), Scene2DError> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(Scene2DError::InvalidValue { field });
    }
    Ok(())
}
fn validate_point(point: ScenePoint, field: &'static str) -> Result<(), Scene2DError> {
    finite(point.x, field, -MAX_DIMENSION, MAX_DIMENSION)?;
    finite(point.y, field, -MAX_DIMENSION, MAX_DIMENSION)
}
fn validate_rect(
    rect: &SceneRect,
    field: &'static str,
    positive: bool,
) -> Result<(), Scene2DError> {
    validate_point(
        ScenePoint {
            x: rect.x,
            y: rect.y,
        },
        field,
    )?;
    let min = if positive { f32::MIN_POSITIVE } else { 0.0 };
    finite(rect.width, field, min, MAX_DIMENSION)?;
    finite(rect.height, field, min, MAX_DIMENSION)
}
fn validate_color(color: Scene2DColor) -> Result<(), Scene2DError> {
    for (channel, value) in [
        ("r", color.r),
        ("g", color.g),
        ("b", color.b),
        ("a", color.a),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(Scene2DError::InvalidColor { channel });
        }
    }
    Ok(())
}
fn validate_semantic(value: Option<&Scene2DSemantic>) -> Result<(), Scene2DError> {
    if let Some(value) = value
        && (value.label.trim().is_empty()
            || value.label.len() > MAX_TEXT_BYTES
            || value
                .description
                .as_ref()
                .is_some_and(|text| text.len() > MAX_TEXT_BYTES)
            || value
                .value_text
                .as_ref()
                .is_some_and(|text| text.len() > MAX_TEXT_BYTES))
    {
        return Err(Scene2DError::InvalidId {
            field: "semantic label",
            max_bytes: MAX_TEXT_BYTES,
        });
    }
    Ok(())
}
fn validate_grid(grid: Option<&GridSpec>) -> Result<(), Scene2DError> {
    let Some(grid) = grid else {
        return Ok(());
    };
    let Ok(rows) = usize::try_from(grid.rows) else {
        return Err(Scene2DError::InvalidGrid);
    };
    let Ok(columns) = usize::try_from(grid.columns) else {
        return Err(Scene2DError::InvalidGrid);
    };
    let Some(cells) = rows.checked_mul(columns) else {
        return Err(Scene2DError::InvalidGrid);
    };
    if rows == 0 || columns == 0 || cells > MAX_GRID_CELLS {
        return Err(Scene2DError::InvalidGrid);
    }
    if grid.row_labels.len() > rows || grid.column_labels.len() > columns {
        return Err(Scene2DError::InvalidGrid);
    }
    if grid
        .row_labels
        .iter()
        .chain(&grid.column_labels)
        .any(|label| label.len() > MAX_TEXT_BYTES)
    {
        return Err(Scene2DError::InvalidGrid);
    }
    for value in [grid.x, grid.y] {
        finite(value, "grid origin", -MAX_DIMENSION, MAX_DIMENSION)
            .map_err(|_| Scene2DError::InvalidGrid)?;
    }
    for value in [grid.cell_width, grid.cell_height] {
        finite(value, "grid cell size", f32::MIN_POSITIVE, MAX_DIMENSION)
            .map_err(|_| Scene2DError::InvalidGrid)?;
    }
    finite(grid.gap, "grid gap", 0.0, MAX_DIMENSION).map_err(|_| Scene2DError::InvalidGrid)
}
fn validate_transform(value: &Scene2DTransform) -> Result<(), Scene2DError> {
    finite(
        value.translate_x,
        "transform translation",
        -MAX_DIMENSION,
        MAX_DIMENSION,
    )?;
    finite(
        value.translate_y,
        "transform translation",
        -MAX_DIMENSION,
        MAX_DIMENSION,
    )?;
    finite(value.scale_x, "transform scale", 0.0, 1_000.0)?;
    finite(value.scale_y, "transform scale", 0.0, 1_000.0)?;
    if value.scale_x <= f32::EPSILON || value.scale_y <= f32::EPSILON {
        return Err(Scene2DError::InvalidValue {
            field: "transform scale",
        });
    }
    finite(
        value.rotation_degrees,
        "transform rotation",
        -36_000.0,
        36_000.0,
    )?;
    finite(value.shear_x, "transform shear", -1_000_000.0, 1_000_000.0)
}

fn kind_bounds(kind: &Scene2DNodeKind) -> Option<SceneRect> {
    match kind {
        Scene2DNodeKind::Group { .. } => None,
        Scene2DNodeKind::Rect { rect, .. } | Scene2DNodeKind::RoundedRect { rect, .. } => {
            Some(*rect)
        }
        Scene2DNodeKind::Circle { center, radius, .. } => Some(SceneRect {
            x: center.x - radius,
            y: center.y - radius,
            width: radius * 2.0,
            height: radius * 2.0,
        }),
        Scene2DNodeKind::Line { start, end, stroke } => Some(SceneRect {
            x: start.x.min(end.x) - stroke.width * 0.5,
            y: start.y.min(end.y) - stroke.width * 0.5,
            width: (start.x - end.x).abs() + stroke.width,
            height: (start.y - end.y).abs() + stroke.width,
        }),
        Scene2DNodeKind::Path {
            commands, stroke, ..
        } => {
            let mut points = commands.iter().flat_map(|command| match command {
                Scene2DPathCommand::MoveTo { point } | Scene2DPathCommand::LineTo { point } => {
                    vec![*point]
                }
                Scene2DPathCommand::QuadraticTo { control, point } => vec![*control, *point],
                Scene2DPathCommand::CubicTo {
                    control_a,
                    control_b,
                    point,
                } => vec![*control_a, *control_b, *point],
                Scene2DPathCommand::Close => Vec::new(),
            });
            let first = points.next()?;
            let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
            for point in points {
                min_x = min_x.min(point.x);
                min_y = min_y.min(point.y);
                max_x = max_x.max(point.x);
                max_y = max_y.max(point.y);
            }
            let padding = stroke.as_ref().map_or(0.0, |edge| edge.width * 0.5);
            Some(SceneRect {
                x: min_x - padding,
                y: min_y - padding,
                width: max_x - min_x + padding * 2.0,
                height: max_y - min_y + padding * 2.0,
            })
        }
        Scene2DNodeKind::Text {
            origin,
            content,
            size,
            ..
        } => Some(SceneRect {
            x: origin.x,
            y: origin.y,
            width: content.chars().count() as f32 * *size * 0.62,
            height: *size * 1.4,
        }),
    }
}

fn transformed_bounds_are_finite(rect: SceneRect, transform: Scene2DTransform) -> bool {
    let [a, b, c, d] = transform.linear();
    [
        (rect.x, rect.y),
        (rect.x + rect.width, rect.y),
        (rect.x, rect.y + rect.height),
        (rect.x + rect.width, rect.y + rect.height),
    ]
    .into_iter()
    .all(|(x, y)| {
        let transformed_x = a * x + b * y + transform.translate_x;
        let transformed_y = c * x + d * y + transform.translate_y;
        transformed_x.is_finite() && transformed_y.is_finite()
    })
}
fn validate_stroke(stroke: &Scene2DStroke) -> Result<(), Scene2DError> {
    finite(stroke.width, "stroke width", 0.0, 16_384.0)?;
    validate_color(stroke.color)
}
fn validate_brush(brush: Option<&Scene2DBrush>) -> Result<(), Scene2DError> {
    if let Some(brush) = brush {
        match brush {
            Scene2DBrush::Solid { color } => validate_color(*color)?,
            Scene2DBrush::LinearGradient {
                angle_degrees,
                from,
                to,
            } => {
                if !angle_degrees.is_finite() {
                    return Err(Scene2DError::InvalidValue {
                        field: "gradient angle",
                    });
                }
                validate_color(*from)?;
                validate_color(*to)?;
            }
        }
    }
    Ok(())
}
fn validate_paint(
    fill: Option<&Scene2DBrush>,
    stroke: Option<&Scene2DStroke>,
) -> Result<(), Scene2DError> {
    validate_brush(fill)?;
    if let Some(stroke) = stroke {
        validate_stroke(stroke)?;
    }
    if fill.is_none() && stroke.is_none() {
        return Err(Scene2DError::InvalidValue {
            field: "node paint",
        });
    }
    Ok(())
}
fn validate_kind(node: &Scene2DNode) -> Result<(), Scene2DError> {
    match &node.kind {
        Scene2DNodeKind::Group { .. } => Ok(()),
        Scene2DNodeKind::Rect { rect, fill, stroke } => {
            validate_rect(rect, "rectangle bounds", false)?;
            validate_paint(fill.as_ref(), stroke.as_ref())
        }
        Scene2DNodeKind::RoundedRect {
            rect,
            radius,
            fill,
            stroke,
        } => {
            validate_rect(rect, "rounded rectangle bounds", false)?;
            finite(*radius, "corner radius", 0.0, MAX_DIMENSION)?;
            validate_paint(fill.as_ref(), stroke.as_ref())
        }
        Scene2DNodeKind::Circle {
            center,
            radius,
            fill,
            stroke,
        } => {
            validate_point(*center, "circle center")?;
            finite(*radius, "circle radius", f32::MIN_POSITIVE, MAX_DIMENSION)?;
            validate_paint(fill.as_ref(), stroke.as_ref())
        }
        Scene2DNodeKind::Line { start, end, stroke } => {
            validate_point(*start, "line start")?;
            validate_point(*end, "line end")?;
            validate_stroke(stroke)
        }
        Scene2DNodeKind::Path {
            commands,
            fill,
            stroke,
        } => {
            if commands.len() < 2
                || !matches!(commands.first(), Some(Scene2DPathCommand::MoveTo { .. }))
            {
                return Err(Scene2DError::InvalidPath {
                    id: node.id.clone(),
                });
            }
            if commands.len() > MAX_PATH_COMMANDS {
                return Err(Scene2DError::LimitExceeded {
                    field: "path commands",
                    count: commands.len(),
                    limit: MAX_PATH_COMMANDS,
                });
            }
            for command in commands {
                match command {
                    Scene2DPathCommand::MoveTo { point } | Scene2DPathCommand::LineTo { point } => {
                        validate_point(*point, "path point")?;
                    }
                    Scene2DPathCommand::QuadraticTo { control, point } => {
                        validate_point(*control, "path control point")?;
                        validate_point(*point, "path point")?;
                    }
                    Scene2DPathCommand::CubicTo {
                        control_a,
                        control_b,
                        point,
                    } => {
                        validate_point(*control_a, "path control point")?;
                        validate_point(*control_b, "path control point")?;
                        validate_point(*point, "path point")?;
                    }
                    Scene2DPathCommand::Close => {}
                }
            }
            validate_paint(fill.as_ref(), stroke.as_ref())
        }
        Scene2DNodeKind::Text {
            origin,
            content,
            size,
            color,
            font,
            ..
        } => {
            validate_point(*origin, "text origin")?;
            if content.is_empty() || content.len() > MAX_TEXT_BYTES {
                return Err(Scene2DError::InvalidId {
                    field: "text content",
                    max_bytes: MAX_TEXT_BYTES,
                });
            }
            if content.bytes().any(|byte| byte == b'\n' || byte == b'\r') {
                return Err(Scene2DError::MultilineText {
                    id: node.id.clone(),
                });
            }
            if font
                .as_ref()
                .is_some_and(|name| name.trim().is_empty() || name.len() > MAX_ID_BYTES)
            {
                return Err(Scene2DError::InvalidId {
                    field: "font family",
                    max_bytes: MAX_ID_BYTES,
                });
            }
            finite(*size, "text size", f32::MIN_POSITIVE, 4096.0)?;
            validate_color(*color)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> Scene2DNode {
        Scene2DNode {
            id: id.into(),
            hit_id: Some(id.into()),
            hit_bounds: None,
            clip: None,
            shadow: None,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect {
                    x: 0.0,
                    y: 0.0,
                    width: 20.0,
                    height: 20.0,
                },
                fill: Some(Scene2DBrush::Solid {
                    color: Scene2DColor {
                        r: 0.2,
                        g: 0.3,
                        b: 0.4,
                        a: 1.0,
                    },
                }),
                stroke: None,
            },
            opacity: 1.0,
            transform: Scene2DTransform::default(),
            transition: None,
            semantic: None,
        }
    }
    fn scene() -> Scene2DScene {
        Scene2DScene {
            version: 1,
            revision: 1,
            semantic: None,
            view_box: SceneRect {
                x: 0.0,
                y: 0.0,
                width: 40.0,
                height: 40.0,
            },
            nodes: vec![node("background"), node("cell-0")],
            grid: Some(GridSpec {
                rows: 2,
                columns: 2,
                x: 0.0,
                y: 0.0,
                cell_width: 20.0,
                cell_height: 20.0,
                gap: 0.0,
                row_labels: Vec::new(),
                column_labels: Vec::new(),
            }),
            background: None,
            input: Scene2DInputConfig::default(),
        }
    }

    #[test]
    fn frozen_scene_schema_deserializes_and_validates() {
        let value = serde_json::json!({"version":1,"revision":4,"view_box":{"x":0.0,"y":0.0,"width":96.0,"height":96.0},
            "nodes":[{"id":"cell-0","hit_id":"cell-0","kind":{"type":"rounded_rect","rect":{"x":0.0,"y":0.0,"width":32.0,"height":32.0},"radius":4.0,
                "fill":{"type":"solid","color":{"r":0.2,"g":0.3,"b":0.4,"a":1.0}},"stroke":null},"semantic":{"role":"grid_cell","label":"Cell 1"}}],
            "grid":{"rows":3,"columns":3,"x":0.0,"y":0.0,"cell_width":32.0,"cell_height":32.0,"gap":0.0},
            "input":{"pointer":true,"continuous":true,"capture":true,"keyboard":true}});
        let parsed: Scene2DScene = serde_json::from_value(value).expect("scene JSON deserializes");
        parsed.validate().expect("scene validates");
        assert_eq!(parsed.nodes[0].id, "cell-0");
        assert!(parsed.input.continuous);
    }

    #[test]
    fn gradient_brush_schema_is_typed_and_validated() {
        let value = serde_json::json!({
            "version": 1,
            "revision": 1,
            "view_box": {"x": 0.0, "y": 0.0, "width": 40.0, "height": 40.0},
            "nodes": [],
            "background": {
                "type": "linear_gradient",
                "angle_degrees": 45.0,
                "from": {"r": 0.0, "g": 0.1, "b": 0.2, "a": 1.0},
                "to": {"r": 0.7, "g": 0.8, "b": 0.9, "a": 1.0}
            },
            "input": {"pointer": false, "continuous": false, "capture": false, "keyboard": false}
        });
        let parsed: Scene2DScene = serde_json::from_value(value).expect("gradient scene decodes");
        parsed.validate().expect("gradient scene validates");
        assert!(matches!(
            parsed.background,
            Some(Scene2DBrush::LinearGradient { .. })
        ));

        let invalid = Scene2DBrush::LinearGradient {
            angle_degrees: f32::NAN,
            from: Scene2DColor::default(),
            to: Scene2DColor::default(),
        };
        assert!(matches!(
            validate_brush(Some(&invalid)),
            Err(Scene2DError::InvalidValue {
                field: "gradient angle"
            })
        ));
    }

    #[test]
    fn optional_text_alignment_and_lifecycle_events_match_the_wire_schema() {
        let value = serde_json::json!({
            "version": 1,
            "revision": 1,
            "view_box": {"x": 0.0, "y": 0.0, "width": 40.0, "height": 40.0},
            "nodes": [{
                "id": "label",
                "kind": {
                    "type": "text",
                    "origin": {"x": 2.0, "y": 4.0},
                    "content": "Tile",
                    "size": 12.0,
                    "color": {"r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0}
                }
            }],
            "input": {"pointer": true, "continuous": false, "capture": true, "keyboard": true}
        });
        let parsed: Scene2DScene = serde_json::from_value(value).expect("text node decodes");
        parsed.validate().expect("text node validates");
        let Scene2DNodeKind::Text { align, .. } = &parsed.nodes[0].kind else {
            panic!("expected text node");
        };
        assert_eq!(*align, Scene2DTextAlign::Left);

        let resumed = Scene2DInput::Lifecycle {
            reason: Scene2DLifecycleReason::Resumed,
            timestamp_ns: 7,
        };
        let wire = serde_json::to_value(&resumed).expect("serialize lifecycle input");
        assert_eq!(wire["type"], "lifecycle");
        assert_eq!(wire["reason"], "resumed");
        assert_eq!(
            serde_json::from_value::<Scene2DInput>(wire).expect("deserialize lifecycle input"),
            resumed
        );

        let completion = Scene2DInput::TransitionComplete {
            id: "piece-4".into(),
            completion_id: "fall-4".into(),
            timestamp_ns: 9,
        };
        let wire = serde_json::to_value(&completion).expect("serialize completion input");
        assert_eq!(wire["type"], "transition_complete");
        assert_eq!(wire["id"], "piece-4");
        assert_eq!(
            serde_json::from_value::<Scene2DInput>(wire).expect("deserialize completion input"),
            completion
        );
    }

    #[test]
    fn recursive_group_ids_are_validated_indexed_and_patched_atomically() {
        let group = Scene2DNode {
            id: "group".into(),
            hit_id: None,
            clip: Some(SceneRect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            }),
            kind: Scene2DNodeKind::Group {
                children: vec![node("nested")],
            },
            ..node("group")
        };
        let mut nested_scene = scene();
        nested_scene.nodes = vec![group.clone()];
        nested_scene.validate().expect("group scene validates");
        let mut cache = Scene2DCache::default();
        cache.replace(nested_scene).expect("group snapshot");
        assert_eq!(cache.node_index("group"), Some(0));
        assert_eq!(cache.node_index("nested"), Some(1));

        let replacement = Scene2DNode {
            kind: Scene2DNodeKind::Group {
                children: vec![node("replacement-child")],
            },
            ..group
        };
        let replaced = cache
            .apply_patch(&Scene2DPatch {
                base_revision: 1,
                revision: 2,
                upsert: vec![replacement],
                remove: vec![],
                view_box: None,
                grid: None,
                input: None,
                semantic: None,
                background: None,
            })
            .expect("replace full group subtree");
        assert_eq!(replaced.inserted, ["replacement-child"]);
        assert_eq!(replaced.updated, ["group"]);
        assert_eq!(replaced.removed, ["nested"]);
        assert_eq!(cache.node_index("replacement-child"), Some(1));

        let removed = cache
            .apply_patch(&Scene2DPatch {
                base_revision: 2,
                revision: 3,
                upsert: vec![],
                remove: vec!["group".into()],
                view_box: None,
                grid: None,
                input: None,
                semantic: None,
                background: None,
            })
            .expect("remove group subtree");
        let mut removed_ids = removed.removed;
        removed_ids.sort();
        assert_eq!(removed_ids, ["group", "replacement-child"]);
        assert_eq!(cache.node_index("replacement-child"), None);
    }

    #[test]
    fn group_depth_limit_matches_native_scene_validation() {
        let mut nested = node("leaf");
        for index in 0..MAX_GROUP_DEPTH {
            let id = format!("group-{index}");
            nested = Scene2DNode {
                kind: Scene2DNodeKind::Group {
                    children: vec![nested],
                },
                ..node(&id)
            };
        }
        let mut accepted = scene();
        accepted.nodes = vec![nested.clone()];
        accepted
            .validate()
            .expect("64 nested groups remain within the limit");

        let too_deep = Scene2DNode {
            kind: Scene2DNodeKind::Group {
                children: vec![nested],
            },
            ..node("group-64")
        };
        accepted.nodes = vec![too_deep];
        assert!(matches!(
            accepted.validate(),
            Err(Scene2DError::LimitExceeded {
                field: "group depth",
                count: 65,
                limit: MAX_GROUP_DEPTH,
            })
        ));
    }

    #[test]
    fn nested_transform_overflow_rejects_patch_without_replacing_valid_scene() {
        let mut nested = node("leaf");
        for index in 0..6 {
            let id = format!("group-{index}");
            nested = Scene2DNode {
                transform: Scene2DTransform {
                    scale_x: 1_000.0,
                    scale_y: 1_000.0,
                    ..Scene2DTransform::default()
                },
                kind: Scene2DNodeKind::Group {
                    children: vec![nested],
                },
                ..node(&id)
            };
        }
        let mut previous = scene();
        previous.nodes = vec![nested];
        previous
            .validate()
            .expect("the original transform chain is finite");
        let mut cache = Scene2DCache::default();
        cache
            .replace(previous.clone())
            .expect("install valid snapshot");

        let mut overflowing_leaf = node("leaf");
        overflowing_leaf.transform.scale_x = 1_000.0;
        overflowing_leaf.transform.scale_y = 1_000.0;
        let patch = Scene2DPatch {
            base_revision: 1,
            revision: 2,
            upsert: vec![overflowing_leaf],
            remove: vec![],
            view_box: None,
            grid: None,
            input: None,
            semantic: None,
            background: None,
        };

        assert!(matches!(
            cache.apply_patch(&patch),
            Err(Scene2DError::InvalidValue {
                field: "composed transform"
            })
        ));
        assert_eq!(cache.scene(), Some(&previous));
    }

    #[test]
    fn finite_json_numbers_outside_f32_are_rejected_during_decode() {
        let value = serde_json::json!({
            "version": 1,
            "revision": 1,
            "view_box": {"x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0},
            "nodes": [{
                "id": "oversized",
                "kind": {
                    "type": "rect",
                    "rect": {"x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0},
                    "fill": null,
                    "stroke": {"width": 1.0, "color": {"r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0}}
                },
                "transform": {
                    "translate_x": 1e300, "translate_y": 0.0,
                    "scale_x": 1.0, "scale_y": 1.0,
                    "rotation_degrees": 0.0, "shear_x": 0.0
                }
            }]
        });
        if let Ok(scene) = serde_json::from_value::<Scene2DScene>(value) {
            assert!(scene.validate().is_err());
        }
    }

    #[test]
    fn python_scene2d_app_export_deserializes_and_validates_in_rust() {
        let python_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("python");
        let source = r##"
import json
from gpui_toolkit import (App, Scene2D, Scene2DClip, Scene2DGroup,
                          Scene2DShadow, Scene2DTransition, SceneRect, section)
scene = Scene2D("board", 20, 20, [Scene2DGroup("group", [
    SceneRect("cell", 1, 1, 8, 8, fill="#f00",
              clip=Scene2DClip(0, 0, 8, 8),
              shadow=Scene2DShadow(1, 1, 2, "#0008"),
              transition=Scene2DTransition(120, completion_id="arrived",
                                           animate_color=True))
])])
app = App(title="cross-language Scene2D", sections=[section("board", "Board", scene)])
print(json.dumps(app.to_spec(), separators=(",", ":")))
"##;
        let output = std::process::Command::new("python3")
            .arg("-c")
            .arg(source)
            .env("PYTHONPATH", python_root)
            .output()
            .expect("run the Python Scene2D declaration exporter");
        assert!(
            output.status.success(),
            "Python Scene2D exporter failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let app: crate::ui_ir::PythonAppIr = serde_json::from_slice(&output.stdout)
            .expect("Python App IR, including its scene2d `kind` discriminator, decodes");
        app.validate()
            .expect("Python App IR validates in the native host");
        assert!(matches!(
            app.sections.first().map(|section| &section.content),
            Some(crate::ui_ir::UiNode::Scene2D(_))
        ));
    }

    #[test]
    fn patch_preserves_ids_order_and_commits_atomically() {
        let mut cache = Scene2DCache::default();
        cache.replace(scene()).expect("snapshot");
        let result = cache
            .apply_patch(&Scene2DPatch {
                base_revision: 1,
                revision: 2,
                upsert: vec![node("cell-0"), node("cell-1")],
                remove: vec!["background".into()],
                view_box: None,
                grid: None,
                input: None,
                semantic: None,
                background: None,
            })
            .expect("patch");
        assert_eq!(result.updated, ["cell-0"]);
        assert_eq!(result.inserted, ["cell-1"]);
        assert_eq!(result.removed, ["background"]);
        let ids = cache
            .scene()
            .unwrap()
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["cell-0", "cell-1"]);
        assert_eq!(cache.node_index("cell-0"), Some(0));
    }

    #[test]
    fn patch_null_distinguishes_explicit_removal_from_omission() {
        let unchanged: Scene2DPatch = serde_json::from_value(serde_json::json!({
            "base_revision": 1,
            "revision": 2
        }))
        .expect("decode unchanged optional properties");
        assert_eq!(unchanged.grid, None);
        assert_eq!(unchanged.background, None);

        let clear: Scene2DPatch = serde_json::from_value(serde_json::json!({
            "base_revision": 1,
            "revision": 2,
            "grid": null,
            "semantic": null,
            "background": null
        }))
        .expect("decode explicit removals");
        assert_eq!(clear.grid, Some(None));
        assert_eq!(clear.semantic, Some(None));
        assert_eq!(clear.background, Some(None));
        let wire = serde_json::to_value(clear).expect("serialize explicit removals");
        assert!(wire["grid"].is_null());
        assert!(wire["background"].is_null());
        assert!(wire.get("input").is_none());
    }

    #[test]
    fn invalid_or_skipped_patch_keeps_last_valid_scene() {
        let mut cache = Scene2DCache::default();
        cache.replace(scene()).expect("snapshot");
        let previous = cache.scene().cloned().unwrap();
        let invalid = Scene2DPatch {
            base_revision: 1,
            revision: 2,
            upsert: vec![Scene2DNode {
                opacity: 1.5,
                ..node("cell-0")
            }],
            remove: vec![],
            view_box: None,
            grid: None,
            input: None,
            semantic: None,
            background: None,
        };
        cache.apply_patch(&invalid).unwrap_err();
        assert_eq!(cache.scene(), Some(&previous));
        let gap = Scene2DPatch {
            base_revision: 1,
            revision: 3,
            upsert: vec![],
            remove: vec![],
            view_box: None,
            grid: None,
            input: None,
            semantic: None,
            background: None,
        };
        assert!(matches!(
            cache.apply_patch(&gap),
            Err(Scene2DError::RevisionGap { .. })
        ));
        assert_eq!(cache.scene(), Some(&previous));

        let mut invalid_snapshot = scene();
        invalid_snapshot.revision = 2;
        invalid_snapshot.nodes[0].kind = Scene2DNodeKind::Text {
            origin: ScenePoint { x: 0.0, y: 0.0 },
            content: "first\nsecond".into(),
            size: 12.0,
            color: Scene2DColor {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: 1.0,
            },
            font: None,
            align: Scene2DTextAlign::Left,
        };
        assert!(matches!(
            cache.replace(invalid_snapshot),
            Err(Scene2DError::MultilineText { .. })
        ));
        assert_eq!(cache.scene(), Some(&previous));
    }

    #[cfg(feature = "showcase")]
    #[test]
    fn python_snapshot_and_text_limits_match_native_toolkit_validation() {
        let snapshot = serde_json::json!({
            "version": 1,
            "revision": 8,
            "view_box": {"x": 0.0, "y": 0.0, "width": 96.0, "height": 64.0},
            "background": {
                "type": "linear_gradient",
                "angle_degrees": 90.0,
                "from": {"r": 0.02, "g": 0.03, "b": 0.05, "a": 1.0},
                "to": {"r": 0.08, "g": 0.10, "b": 0.12, "a": 1.0}
            },
            "semantic": {"role": "group", "label": "Game board"},
            "nodes": [
                {
                    "id": "tile-0",
                    "hit_id": "r0c0",
                    "hit_bounds": {"x": 2.0, "y": 2.0, "width": 28.0, "height": 28.0},
                    "kind": {
                        "type": "rounded_rect",
                        "rect": {"x": 2.0, "y": 2.0, "width": 28.0, "height": 28.0},
                        "radius": 3.0,
                        "fill": {"type": "solid", "color": {"r": 0.8, "g": 0.3, "b": 0.1, "a": 1.0}},
                        "stroke": null
                    },
                    "opacity": 0.9,
                    "transform": {"translate_x": 0.0, "translate_y": 0.0, "scale_x": 1.0, "scale_y": 1.0, "rotation_degrees": 0.0},
                    "transition": {"duration_ms": 160, "easing": "ease_out_cubic"},
                    "semantic": {"role": "grid_cell", "label": "Row 1, column 1"}
                },
                {
                    "id": "label-0",
                    "kind": {
                        "type": "text",
                        "origin": {"x": 10.0, "y": 20.0},
                        "content": "7",
                        "size": 18.0,
                        "color": {"r": 1.0, "g": 1.0, "b": 1.0, "a": 1.0},
                        "font": "System",
                        "align": "center"
                    }
                }
            ],
            "grid": {"rows": 2, "columns": 3, "x": 2.0, "y": 2.0, "cell_width": 28.0, "cell_height": 28.0, "gap": 2.0, "row_labels": ["1", "2"], "column_labels": ["1", "2", "3"]},
            "input": {"pointer": true, "continuous": false, "capture": true, "keyboard": true}
        });

        let runtime: Scene2DScene = serde_json::from_value(snapshot.clone())
            .expect("Python snapshot decodes into the runtime schema");
        runtime
            .validate()
            .expect("runtime accepts the shared Python snapshot");
        let native: gpui_ui_kit::scene2d::Scene2DScene = serde_json::from_value(snapshot.clone())
            .expect("Python snapshot decodes into the native toolkit schema");
        native
            .validate()
            .expect("native toolkit accepts the shared Python snapshot");

        let mut multiline = snapshot;
        multiline["nodes"][1]["kind"]["content"] = serde_json::Value::String("top\nbottom".into());
        let runtime_multiline: Scene2DScene = serde_json::from_value(multiline.clone())
            .expect("multiline text still has a well-formed JSON shape");
        let native_multiline: gpui_ui_kit::scene2d::Scene2DScene =
            serde_json::from_value(multiline).expect("native multiline JSON decodes");
        assert!(matches!(
            runtime_multiline.validate(),
            Err(Scene2DError::MultilineText { .. })
        ));
        assert!(
            native_multiline.validate().is_err(),
            "toolkit must reject text the line painter cannot shape"
        );
    }

    #[test]
    fn normalized_pointer_event_keeps_semantic_hit_and_cell() {
        let event = Scene2DInput::Pointer {
            phase: Scene2DPointerPhase::Move,
            device: Scene2DDevice::Touch,
            contact_id: 7,
            timestamp_ns: 123,
            position: ScenePoint { x: 10.0, y: 10.0 },
            buttons: vec![Scene2DPointerButton::Left],
            modifiers: vec![Scene2DModifier::Shift],
            hit_id: Some("cell-0".into()),
            cell: Some(Scene2DGridCell {
                row: 0,
                column: 0,
                index: 0,
                id: "r0c0".into(),
            }),
        };
        let value = serde_json::to_value(&event).expect("serialize event");
        assert_eq!(value["type"], "pointer");
        assert_eq!(value["phase"], "move");
        assert_eq!(value["device"], "touch");
        assert_eq!(value["hit_id"], "cell-0");
        assert_eq!(value["cell"]["index"], 0);
        assert_eq!(value["cell"]["id"], "r0c0");
        assert_eq!(
            serde_json::from_value::<Scene2DInput>(value).expect("deserialize event"),
            event
        );
    }
}
