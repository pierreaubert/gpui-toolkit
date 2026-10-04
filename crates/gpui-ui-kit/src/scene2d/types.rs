//! Serializable scene and input types for native two-dimensional surfaces.
//!
//! Scenes are ordered display lists with stable object identifiers. Geometry
//! uses view-box logical units; rendering and hit testing share one transform.
//! The runtime may replace a whole validated scene while UI-kit owns native
//! presentation, hit resolution, accessibility registration, and animation.
//!
//! # Examples
//!
//! ```
//! use gpui_ui_kit::scene2d::{Scene2DScene, SceneRect};
//!
//! let scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 9.0, 9.0));
//! assert_eq!(scene.version, 1);
//! ```

// Rust guideline compliant 2026-02-21

use serde::{Deserialize, Serialize};
use std::backtrace::Backtrace;
use std::fmt::{Display, Formatter};

/// Current version of the serialized `Scene2D` scene schema.
pub const SCENE2D_SCHEMA_VERSION: u16 = 1;

/// Maximum number of nested transform groups accepted by one scene.
pub const SCENE2D_MAX_GROUP_DEPTH: usize = 64;

fn default_version() -> u16 {
    SCENE2D_SCHEMA_VERSION
}

fn default_one() -> f32 {
    1.0
}

fn default_true() -> bool {
    true
}

/// A two-dimensional point in scene logical coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct ScenePoint {
    /// Horizontal coordinate in view-box units.
    pub x: f32,
    /// Vertical coordinate in view-box units.
    pub y: f32,
}

impl ScenePoint {
    /// Creates a point from its horizontal and vertical coordinates.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui_ui_kit::scene2d::ScenePoint;
    /// assert_eq!(ScenePoint::new(2.0, 3.0).x, 2.0);
    /// ```
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub(crate) fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

/// An axis-aligned rectangle in scene logical coordinates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SceneRect {
    /// Left coordinate in view-box units.
    pub x: f32,
    /// Top coordinate in view-box units.
    pub y: f32,
    /// Rectangle width in view-box units.
    pub width: f32,
    /// Rectangle height in view-box units.
    pub height: f32,
}

impl SceneRect {
    /// Creates a rectangle from its origin and size.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui_ui_kit::scene2d::SceneRect;
    /// assert_eq!(SceneRect::new(2.0, 3.0, 4.0, 5.0).width, 4.0);
    /// ```
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub(crate) fn contains(self, point: ScenePoint) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }

    pub(crate) fn is_finite(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
    }
}

/// A straight-alpha RGBA color with channels from zero through one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DColor {
    /// Red channel in the range zero through one.
    pub r: f32,
    /// Green channel in the range zero through one.
    pub g: f32,
    /// Blue channel in the range zero through one.
    pub b: f32,
    /// Alpha channel in the range zero through one.
    #[serde(default = "default_one")]
    pub a: f32,
}

impl Scene2DColor {
    /// Creates an opaque color from normalized RGB channels.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui_ui_kit::scene2d::Scene2DColor;
    /// assert_eq!(Scene2DColor::rgb(1.0, 0.0, 0.0).a, 1.0);
    /// ```
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Creates a color from normalized RGBA channels.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub(crate) fn is_valid(self) -> bool {
        [self.r, self.g, self.b, self.a]
            .into_iter()
            .all(|channel| channel.is_finite() && (0.0..=1.0).contains(&channel))
    }
}

/// A solid or two-stop linear gradient used by a scene object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DBrush {
    /// A single color.
    Solid {
        /// Color applied throughout the object.
        color: Scene2DColor,
    },
    /// A clockwise gradient whose angle starts at the top.
    LinearGradient {
        /// Direction in degrees, from zero through 360.
        angle_degrees: f32,
        /// Gradient start color.
        from: Scene2DColor,
        /// Gradient end color.
        to: Scene2DColor,
    },
}

impl Scene2DBrush {
    /// Creates a solid brush from a color.
    pub const fn solid(color: Scene2DColor) -> Self {
        Self::Solid { color }
    }

    pub(crate) fn validate(&self, path: &str) -> Result<(), Scene2DValidationError> {
        match self {
            Self::Solid { color } => validate_color(*color, path),
            Self::LinearGradient {
                angle_degrees,
                from,
                to,
            } => {
                if !angle_degrees.is_finite() {
                    return Err(Scene2DValidationError::new(
                        path,
                        "gradient angle must be finite",
                    ));
                }
                validate_color(*from, path)?;
                validate_color(*to, path)
            }
        }
    }
}

/// A stroked edge for a path or geometric object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DStroke {
    /// Stroke thickness in view-box units.
    pub width: f32,
    /// Stroke color.
    pub color: Scene2DColor,
}

/// A path command expressed in scene logical coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DPathCommand {
    /// Move the path pen without drawing.
    MoveTo {
        /// Destination point.
        point: ScenePoint,
    },
    /// Draw a straight segment.
    LineTo {
        /// Destination point.
        point: ScenePoint,
    },
    /// Draw a quadratic Bézier segment.
    QuadraticTo {
        /// Control point.
        control: ScenePoint,
        /// Destination point.
        point: ScenePoint,
    },
    /// Draw a cubic Bézier segment.
    CubicTo {
        /// First control point.
        control_a: ScenePoint,
        /// Second control point.
        control_b: ScenePoint,
        /// Destination point.
        point: ScenePoint,
    },
    /// Close the current subpath.
    Close,
}

/// Text alignment for a single scene line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DTextAlign {
    /// Align text to the left side of its bounds.
    #[default]
    Left,
    /// Center text inside its bounds.
    Center,
    /// Align text to the right side of its bounds.
    Right,
}

/// A native geometric, path, or text drawing instruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Scene2DNodeKind {
    /// Apply a local transform and optional clip to an ordered group.
    Group {
        /// Child objects painted in list order.
        #[serde(default)]
        children: Vec<Scene2DNode>,
    },
    /// Draw a rectangle.
    Rect {
        /// Rectangle geometry.
        rect: SceneRect,
        /// Fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional edge stroke.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// Draw a rounded rectangle.
    RoundedRect {
        /// Rectangle geometry.
        rect: SceneRect,
        /// Uniform corner radius.
        radius: f32,
        /// Fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional edge stroke.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// Draw a circle.
    Circle {
        /// Center point.
        center: ScenePoint,
        /// Circle radius.
        radius: f32,
        /// Fill brush.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional edge stroke.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// Draw one straight segment.
    Line {
        /// Segment start point.
        start: ScenePoint,
        /// Segment end point.
        end: ScenePoint,
        /// Stroke style.
        stroke: Scene2DStroke,
    },
    /// Draw a Bézier path with an optional fill and stroke.
    Path {
        /// Ordered path commands.
        commands: Vec<Scene2DPathCommand>,
        /// Optional closed-path fill.
        #[serde(default)]
        fill: Option<Scene2DBrush>,
        /// Optional edge stroke.
        #[serde(default)]
        stroke: Option<Scene2DStroke>,
    },
    /// Draw one shaped line of text.
    Text {
        /// Top-left line origin.
        origin: ScenePoint,
        /// UTF-8 text content.
        content: String,
        /// Font size in view-box units.
        size: f32,
        /// Text color.
        color: Scene2DColor,
        /// Optional font family name.
        #[serde(default)]
        font: Option<String>,
        /// Text alignment inside optional node hit bounds.
        #[serde(default)]
        align: Scene2DTextAlign,
    },
}

impl Scene2DNodeKind {
    pub(crate) fn validate(
        &self,
        path: &str,
        group_depth: usize,
        parent_transform: Scene2DTransform,
    ) -> Result<(), Scene2DValidationError> {
        let rect = |rect: SceneRect, key: &str| {
            if !rect.is_finite() || rect.width < 0.0 || rect.height < 0.0 {
                Err(Scene2DValidationError::new(
                    format!("{path}.{key}"),
                    "geometry must be finite with nonnegative dimensions",
                ))
            } else {
                Ok(())
            }
        };
        let stroke =
            |stroke: Scene2DStroke, key: &str| validate_stroke(stroke, &format!("{path}.{key}"));
        match self {
            Self::Rect {
                rect: bounds,
                fill,
                stroke: edge,
            } => {
                rect(*bounds, "rect")?;
                if let Some(fill) = fill {
                    fill.validate(&format!("{path}.fill"))?;
                }
                if let Some(edge) = edge {
                    stroke(*edge, "stroke")?;
                }
            }
            Self::RoundedRect {
                rect: bounds,
                radius,
                fill,
                stroke: edge,
            } => {
                rect(*bounds, "rect")?;
                if !radius.is_finite() || *radius < 0.0 {
                    return Err(Scene2DValidationError::new(
                        format!("{path}.radius"),
                        "radius must be finite and nonnegative",
                    ));
                }
                if let Some(fill) = fill {
                    fill.validate(&format!("{path}.fill"))?;
                }
                if let Some(edge) = edge {
                    stroke(*edge, "stroke")?;
                }
            }
            Self::Circle {
                center,
                radius,
                fill,
                stroke: edge,
            } => {
                if !center.is_finite() || !radius.is_finite() || *radius < 0.0 {
                    return Err(Scene2DValidationError::new(
                        path,
                        "circle geometry must be finite with a nonnegative radius",
                    ));
                }
                if let Some(fill) = fill {
                    fill.validate(&format!("{path}.fill"))?;
                }
                if let Some(edge) = edge {
                    stroke(*edge, "stroke")?;
                }
            }
            Self::Line {
                start,
                end,
                stroke: edge,
            } => {
                if !start.is_finite() || !end.is_finite() {
                    return Err(Scene2DValidationError::new(
                        path,
                        "line points must be finite",
                    ));
                }
                stroke(*edge, "stroke")?;
            }
            Self::Path {
                commands,
                fill,
                stroke: edge,
            } => {
                if commands.is_empty() {
                    return Err(Scene2DValidationError::new(
                        format!("{path}.commands"),
                        "path must contain at least one command",
                    ));
                }
                for command in commands {
                    if !command.is_finite() {
                        return Err(Scene2DValidationError::new(
                            format!("{path}.commands"),
                            "path points must be finite",
                        ));
                    }
                }
                if let Some(fill) = fill {
                    fill.validate(&format!("{path}.fill"))?;
                }
                if let Some(edge) = edge {
                    stroke(*edge, "stroke")?;
                }
            }
            Self::Text {
                origin,
                content,
                size,
                color,
                ..
            } => {
                if !origin.is_finite() || !size.is_finite() || *size <= 0.0 {
                    return Err(Scene2DValidationError::new(
                        path,
                        "text origin and positive font size must be finite",
                    ));
                }
                if content.contains('\n') || content.contains('\r') {
                    return Err(Scene2DValidationError::new(
                        format!("{path}.content"),
                        "text content must contain a single line",
                    ));
                }
                validate_color(*color, &format!("{path}.color"))?;
            }
            Self::Group { children } => {
                if group_depth >= SCENE2D_MAX_GROUP_DEPTH {
                    return Err(Scene2DValidationError::new(
                        format!("{path}.children"),
                        format!("group nesting cannot exceed {SCENE2D_MAX_GROUP_DEPTH}"),
                    ));
                }
                for (index, child) in children.iter().enumerate() {
                    child.validate_at_depth(
                        &format!("{path}.children[{index}]"),
                        group_depth + 1,
                        parent_transform,
                    )?;
                }
            }
        }
        Ok(())
    }
}

impl Scene2DPathCommand {
    fn is_finite(self) -> bool {
        match self {
            Self::MoveTo { point } | Self::LineTo { point } => point.is_finite(),
            Self::QuadraticTo { control, point } => control.is_finite() && point.is_finite(),
            Self::CubicTo {
                control_a,
                control_b,
                point,
            } => control_a.is_finite() && control_b.is_finite() && point.is_finite(),
            Self::Close => true,
        }
    }
}

/// Motion relative to a node's local origin.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DTransform {
    /// Horizontal translation in scene units.
    #[serde(default)]
    pub translate_x: f32,
    /// Vertical translation in scene units.
    #[serde(default)]
    pub translate_y: f32,
    /// Horizontal scale factor.
    #[serde(default = "default_one")]
    pub scale_x: f32,
    /// Vertical scale factor.
    #[serde(default = "default_one")]
    pub scale_y: f32,
    /// Horizontal shear introduced by composing nested transform groups.
    #[serde(default)]
    pub shear_x: f32,
    /// Clockwise rotation in degrees.
    #[serde(default)]
    pub rotation_degrees: f32,
}

impl Default for Scene2DTransform {
    fn default() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            shear_x: 0.0,
            rotation_degrees: 0.0,
        }
    }
}

impl Scene2DTransform {
    /// Creates a translation-only transform.
    pub const fn translation(x: f32, y: f32) -> Self {
        Self {
            translate_x: x,
            translate_y: y,
            ..Self::identity()
        }
    }

    /// Returns an identity transform.
    pub const fn identity() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            shear_x: 0.0,
            rotation_degrees: 0.0,
        }
    }

    /// Composes parent and child transforms into one exact affine transform.
    pub(crate) fn compose(parent: Self, child: Self) -> Self {
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

    pub(crate) fn linear(self) -> [f32; 4] {
        let angle = self.rotation_degrees.to_radians();
        let (sin, cos) = angle.sin_cos();
        [
            cos * self.scale_x,
            cos * self.shear_x - sin * self.scale_y,
            sin * self.scale_x,
            sin * self.shear_x + cos * self.scale_y,
        ]
    }

    pub(crate) fn is_valid(self) -> bool {
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
    }
}

/// Easing curve names accepted by the serializable transition contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DEasing {
    /// Interpolate at a constant rate.
    Linear,
    /// Decelerate with a quadratic curve.
    EaseOutQuad,
    /// Decelerate with a cubic curve.
    #[default]
    EaseOutCubic,
    /// Accelerate and then decelerate with a cubic curve.
    EaseInOutCubic,
    /// Decelerate with a small overshoot.
    EaseOutBack,
    /// Decelerate with a bouncing curve.
    EaseOutBounce,
}

/// Native transform and opacity transition settings for a changed node.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DTransition {
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Interpolation curve.
    #[serde(default)]
    pub easing: Scene2DEasing,
    /// Optional stable identifier delivered when this transition completes.
    #[serde(default)]
    pub completion_id: Option<String>,
    /// Interpolate changed color-bearing properties with the transition.
    #[serde(default)]
    pub animate_color: bool,
    /// Progressively reveal a changed path from its first point.
    #[serde(default)]
    pub reveal_path: bool,
}

/// A bounded shadow effect for a native primitive.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Scene2DShadow {
    /// Horizontal shadow offset in scene units.
    pub offset_x: f32,
    /// Vertical shadow offset in scene units.
    pub offset_y: f32,
    /// Blur radius in scene units, limited to keep painting predictable.
    pub blur_radius: f32,
    /// Shadow color and opacity.
    pub color: Scene2DColor,
}

impl Scene2DShadow {
    pub(crate) fn validate(self, path: &str) -> Result<(), Scene2DValidationError> {
        if ![self.offset_x, self.offset_y, self.blur_radius]
            .into_iter()
            .all(f32::is_finite)
            || !(0.0..=64.0).contains(&self.blur_radius)
        {
            return Err(Scene2DValidationError::new(
                path,
                "shadow offsets must be finite and blur must be between zero and 64",
            ));
        }
        validate_color(self.color, &format!("{path}.color"))
    }
}

/// Roles for objects represented in the accessible scene tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scene2DSemanticRole {
    /// A labeled grouping.
    #[default]
    Group,
    /// A grid-like game or diagram surface.
    Grid,
    /// A row within a grid.
    Row,
    /// One cell or selectable item.
    GridCell,
    /// An actionable object.
    Button,
    /// A noninteractive image.
    Image,
    /// A status value or announcement.
    Status,
}

/// Accessible metadata for a scene or drawn object.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DSemantic {
    /// Semantic role associated with this object.
    #[serde(default)]
    pub role: Scene2DSemanticRole,
    /// Human-readable accessible name.
    pub label: String,
    /// Optional longer description.
    #[serde(default)]
    pub description: Option<String>,
    /// Optional textual value.
    #[serde(default)]
    pub value_text: Option<String>,
    /// Optional selected state.
    #[serde(default)]
    pub selected: Option<bool>,
    /// Optional disabled state.
    #[serde(default)]
    pub disabled: Option<bool>,
}

/// A stable scene object with geometry, styling, hit metadata, and animation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DNode {
    /// Stable object identity within the scene.
    pub id: String,
    /// Optional semantic pointer target.
    #[serde(default)]
    pub hit_id: Option<String>,
    /// Optional explicit bounds used when hit testing this object.
    #[serde(default)]
    pub hit_bounds: Option<SceneRect>,
    /// Optional clip in this node's local coordinate space.
    #[serde(default)]
    pub clip: Option<SceneRect>,
    /// Optional primitive shadow. Groups clip children but do not paint shadows.
    #[serde(default)]
    pub shadow: Option<Scene2DShadow>,
    /// Optional accessibility metadata.
    #[serde(default)]
    pub semantic: Option<Scene2DSemantic>,
    /// Local transform.
    #[serde(default)]
    pub transform: Scene2DTransform,
    /// Node opacity, from zero through one.
    #[serde(default = "default_one")]
    pub opacity: f32,
    /// Transition used when this node's motion or opacity changes.
    #[serde(default)]
    pub transition: Option<Scene2DTransition>,
    /// Native drawing instruction.
    pub kind: Scene2DNodeKind,
}

impl Scene2DNode {
    fn validate(&self, path: &str) -> Result<(), Scene2DValidationError> {
        self.validate_at_depth(path, 0, Scene2DTransform::identity())
    }

    fn validate_at_depth(
        &self,
        path: &str,
        group_depth: usize,
        parent_transform: Scene2DTransform,
    ) -> Result<(), Scene2DValidationError> {
        if self.id.is_empty() {
            return Err(Scene2DValidationError::new(
                format!("{path}.id"),
                "node identifiers must be nonempty and unique",
            ));
        }
        if self.hit_id.as_ref().is_some_and(String::is_empty) {
            return Err(Scene2DValidationError::new(
                format!("{path}.hit_id"),
                "hit identifiers cannot be empty",
            ));
        }
        if let Some(bounds) = self.hit_bounds
            && (!bounds.is_finite() || bounds.width < 0.0 || bounds.height < 0.0)
        {
            return Err(Scene2DValidationError::new(
                format!("{path}.hit_bounds"),
                "hit bounds must be finite with nonnegative dimensions",
            ));
        }
        if let Some(clip) = self.clip
            && (!clip.is_finite() || clip.width < 0.0 || clip.height < 0.0)
        {
            return Err(Scene2DValidationError::new(
                format!("{path}.clip"),
                "clip bounds must be finite with nonnegative dimensions",
            ));
        }
        if let Some(shadow) = self.shadow {
            if !matches!(
                self.kind,
                Scene2DNodeKind::Rect { .. } | Scene2DNodeKind::RoundedRect { .. }
            ) {
                return Err(Scene2DValidationError::new(
                    format!("{path}.shadow"),
                    "shadows are supported only for rectangle primitives",
                ));
            }
            shadow.validate(&format!("{path}.shadow"))?;
        }
        if !self.transform.is_valid() {
            return Err(Scene2DValidationError::new(
                format!("{path}.transform"),
                "node transforms must be finite and have nonzero scale",
            ));
        }
        let composed_transform = Scene2DTransform::compose(parent_transform, self.transform);
        if !composed_transform.is_valid() {
            return Err(Scene2DValidationError::new(
                format!("{path}.transform"),
                "composed transform must remain finite with nonzero scale",
            ));
        }
        if let Some(bounds) = self.hit_bounds
            && !transformed_bounds_are_finite(bounds, composed_transform)
        {
            return Err(Scene2DValidationError::new(
                format!("{path}.hit_bounds"),
                "transformed hit bounds must remain finite",
            ));
        }
        if let Some(clip) = self.clip
            && !transformed_bounds_are_finite(clip, composed_transform)
        {
            return Err(Scene2DValidationError::new(
                format!("{path}.clip"),
                "transformed clip bounds must remain finite",
            ));
        }
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(Scene2DValidationError::new(
                format!("{path}.opacity"),
                "opacity must be between zero and one",
            ));
        }
        if let Some(semantic) = &self.semantic {
            validate_semantic(semantic, &format!("{path}.semantic"))?;
        }
        if let Some(transition) = &self.transition {
            if transition
                .completion_id
                .as_ref()
                .is_some_and(String::is_empty)
            {
                return Err(Scene2DValidationError::new(
                    format!("{path}.transition.completion_id"),
                    "completion identifiers cannot be empty",
                ));
            }
            if transition.reveal_path && !matches!(&self.kind, Scene2DNodeKind::Path { .. }) {
                return Err(Scene2DValidationError::new(
                    format!("{path}.transition.reveal_path"),
                    "path reveal transitions require a path node",
                ));
            }
        }
        self.kind
            .validate(&format!("{path}.kind"), group_depth, composed_transform)?;
        if let Some(bounds) = kind_bounds(&self.kind)
            && !transformed_bounds_are_finite(bounds, composed_transform)
        {
            return Err(Scene2DValidationError::new(
                format!("{path}.kind"),
                "transformed geometry must remain finite",
            ));
        }
        Ok(())
    }
}

fn kind_bounds(kind: &Scene2DNodeKind) -> Option<SceneRect> {
    match kind {
        Scene2DNodeKind::Group { .. } => None,
        Scene2DNodeKind::Rect { rect, .. } | Scene2DNodeKind::RoundedRect { rect, .. } => {
            Some(*rect)
        }
        Scene2DNodeKind::Circle { center, radius, .. } => Some(SceneRect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        )),
        Scene2DNodeKind::Line { start, end, stroke } => Some(SceneRect::new(
            start.x.min(end.x) - stroke.width * 0.5,
            start.y.min(end.y) - stroke.width * 0.5,
            (start.x - end.x).abs() + stroke.width,
            (start.y - end.y).abs() + stroke.width,
        )),
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
            let padding = stroke.map_or(0.0, |stroke| stroke.width * 0.5);
            Some(SceneRect::new(
                min_x - padding,
                min_y - padding,
                max_x - min_x + padding * 2.0,
                max_y - min_y + padding * 2.0,
            ))
        }
        Scene2DNodeKind::Text {
            origin,
            content,
            size,
            ..
        } => Some(SceneRect::new(
            origin.x,
            origin.y,
            content.chars().count() as f32 * *size * 0.62,
            *size * 1.4,
        )),
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

/// Input capabilities enabled for a scene surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DInputConfig {
    /// Receive mouse or direct pointer down, up, and cancel events.
    #[serde(default = "default_true")]
    pub pointer: bool,
    /// Receive pointer move samples and every crossed grid cell.
    #[serde(default)]
    pub continuous: bool,
    /// Retain an active pointer through moves outside the surface.
    #[serde(default = "default_true")]
    pub capture: bool,
    /// Receive all focused key down and up events.
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

/// Cell identity produced by grid hit testing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scene2DGridCell {
    /// Zero-based row index.
    pub row: u32,
    /// Zero-based column index.
    pub column: u32,
    /// Row-major zero-based cell index.
    pub index: u32,
    /// Stable cell name in the form r{row}c{column}.
    pub id: String,
}

/// Regular grid geometry for board hit testing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DGrid {
    /// Number of rows.
    pub rows: u32,
    /// Number of columns.
    pub columns: u32,
    /// Left grid coordinate.
    pub x: f32,
    /// Top grid coordinate.
    pub y: f32,
    /// Cell width.
    pub cell_width: f32,
    /// Cell height.
    pub cell_height: f32,
    /// Empty space between adjacent cells.
    #[serde(default)]
    pub gap: f32,
    /// Optional row names for accessibility labels.
    #[serde(default)]
    pub row_labels: Vec<String>,
    /// Optional column names for accessibility labels.
    #[serde(default)]
    pub column_labels: Vec<String>,
}

impl Scene2DGrid {
    pub(crate) fn validate(&self) -> Result<(), Scene2DValidationError> {
        if self.rows == 0 || self.columns == 0 {
            return Err(Scene2DValidationError::new(
                "grid",
                "grid rows and columns must both be positive",
            ));
        }
        if ![self.x, self.y, self.cell_width, self.cell_height, self.gap]
            .into_iter()
            .all(f32::is_finite)
            || self.cell_width <= 0.0
            || self.cell_height <= 0.0
            || self.gap < 0.0
        {
            return Err(Scene2DValidationError::new(
                "grid",
                "grid geometry must be finite with positive cells and nonnegative gap",
            ));
        }
        if self.row_labels.len() > self.rows as usize
            || self.column_labels.len() > self.columns as usize
        {
            return Err(Scene2DValidationError::new(
                "grid.labels",
                "grid labels cannot exceed the row or column count",
            ));
        }
        Ok(())
    }

    pub(crate) fn hit_cell(&self, point: ScenePoint) -> Option<Scene2DGridCell> {
        if !point.is_finite() || point.x < self.x || point.y < self.y {
            return None;
        }
        let pitch_x = self.cell_width + self.gap;
        let pitch_y = self.cell_height + self.gap;
        let column = ((point.x - self.x) / pitch_x).floor();
        let row = ((point.y - self.y) / pitch_y).floor();
        if row < 0.0 || column < 0.0 || row >= self.rows as f32 || column >= self.columns as f32 {
            return None;
        }
        let row = row as u32;
        let column = column as u32;
        let cell_x = self.x + column as f32 * pitch_x;
        let cell_y = self.y + row as f32 * pitch_y;
        if point.x > cell_x + self.cell_width || point.y > cell_y + self.cell_height {
            return None;
        }
        let index = row * self.columns + column;
        Some(Scene2DGridCell {
            row,
            column,
            index,
            id: format!("r{row}c{column}"),
        })
    }
}

/// A complete ordered native 2D scene snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene2DScene {
    /// Schema version. Version one is currently supported.
    #[serde(default = "default_version")]
    pub version: u16,
    /// Monotonically increasing source revision.
    #[serde(default)]
    pub revision: u64,
    /// Logical coordinate bounds fitted into the surface.
    pub view_box: SceneRect,
    /// Ordered paint list; later objects appear above earlier ones.
    #[serde(default)]
    pub nodes: Vec<Scene2DNode>,
    /// Optional board grid used for semantic cell picking.
    #[serde(default)]
    pub grid: Option<Scene2DGrid>,
    /// Optional surface-level accessible metadata.
    #[serde(default)]
    pub semantic: Option<Scene2DSemantic>,
    /// Optional scene background.
    #[serde(default)]
    pub background: Option<Scene2DBrush>,
    /// Enabled native input channels.
    #[serde(default)]
    pub input: Scene2DInputConfig,
}

impl Scene2DScene {
    /// Creates an empty version-one scene for the given logical view box.
    ///
    /// # Examples
    ///
    /// ```
    /// use gpui_ui_kit::scene2d::{Scene2DScene, SceneRect};
    /// let scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 100.0, 100.0));
    /// assert!(scene.validate().is_ok());
    /// ```
    pub fn new(view_box: SceneRect) -> Self {
        Self {
            version: SCENE2D_SCHEMA_VERSION,
            revision: 0,
            view_box,
            nodes: Vec::new(),
            grid: None,
            semantic: None,
            background: None,
            input: Scene2DInputConfig::default(),
        }
    }

    /// Validates geometry, identities, colors, and input metadata before use.
    ///
    /// # Errors
    ///
    /// Returns a validation error when scene fields contain unsupported or non-finite values.
    pub fn validate(&self) -> Result<(), Scene2DValidationError> {
        if self.version != SCENE2D_SCHEMA_VERSION {
            return Err(Scene2DValidationError::new(
                "version",
                format!("unsupported scene version {}", self.version),
            ));
        }
        if !self.view_box.is_finite() || self.view_box.width <= 0.0 || self.view_box.height <= 0.0 {
            return Err(Scene2DValidationError::new(
                "view_box",
                "view box must be finite with positive width and height",
            ));
        }
        if let Some(background) = &self.background {
            background.validate("background")?;
        }
        if let Some(grid) = &self.grid {
            grid.validate()?;
        }
        let mut ids = std::collections::HashSet::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            let path = format!("nodes[{index}]");
            node.validate(&path)?;
            insert_node_ids(node, &path, &mut ids)?;
        }
        if let Some(semantic) = &self.semantic {
            validate_semantic(semantic, "semantic")?;
        }
        Ok(())
    }
}

fn insert_node_ids<'a>(
    node: &'a Scene2DNode,
    path: &str,
    ids: &mut std::collections::HashSet<&'a str>,
) -> Result<(), Scene2DValidationError> {
    if !ids.insert(node.id.as_str()) {
        return Err(Scene2DValidationError::new(
            format!("{path}.id"),
            "node identifiers must be nonempty and unique",
        ));
    }
    if let Scene2DNodeKind::Group { children } = &node.kind {
        for (index, child) in children.iter().enumerate() {
            insert_node_ids(child, &format!("{path}.kind.children[{index}]"), ids)?;
        }
    }
    Ok(())
}

/// A scene validation failure with the offending field path.
#[derive(Debug)]
pub struct Scene2DValidationError {
    path: String,
    message: String,
    backtrace: Backtrace,
}

impl Scene2DValidationError {
    pub(crate) fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            backtrace: Backtrace::capture(),
        }
    }

    /// Returns the scene field that failed validation.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the reason validation failed.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Display for Scene2DValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid Scene2D field {}: {}
{}",
            self.path, self.message, self.backtrace
        )
    }
}

impl std::error::Error for Scene2DValidationError {}

fn validate_color(color: Scene2DColor, path: &str) -> Result<(), Scene2DValidationError> {
    if color.is_valid() {
        Ok(())
    } else {
        Err(Scene2DValidationError::new(
            path,
            "color channels must be finite and between zero and one",
        ))
    }
}

fn validate_stroke(stroke: Scene2DStroke, path: &str) -> Result<(), Scene2DValidationError> {
    if !stroke.width.is_finite() || stroke.width <= 0.0 {
        return Err(Scene2DValidationError::new(
            format!("{path}.width"),
            "stroke width must be finite and positive",
        ));
    }
    validate_color(stroke.color, &format!("{path}.color"))
}

fn validate_semantic(semantic: &Scene2DSemantic, path: &str) -> Result<(), Scene2DValidationError> {
    if semantic.label.trim().is_empty() {
        return Err(Scene2DValidationError::new(
            format!("{path}.label"),
            "accessible names cannot be empty",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_serialization_keeps_order_and_nested_kind_tag() {
        let mut scene = Scene2DScene::new(SceneRect::new(1.0, 2.0, 100.0, 80.0));
        scene.revision = 12;
        scene.input = Scene2DInputConfig {
            pointer: true,
            continuous: true,
            capture: true,
            keyboard: true,
        };
        scene.nodes.push(Scene2DNode {
            id: "shape-a".to_owned(),
            hit_id: Some("cell-a".to_owned()),
            hit_bounds: Some(SceneRect::new(1.0, 2.0, 10.0, 10.0)),
            clip: None,
            shadow: None,
            semantic: Some(Scene2DSemantic {
                role: Scene2DSemanticRole::GridCell,
                label: "Row 1, column 1".to_owned(),
                description: None,
                value_text: Some("empty".to_owned()),
                selected: Some(false),
                disabled: None,
            }),
            transform: Scene2DTransform::identity(),
            opacity: 1.0,
            transition: Some(Scene2DTransition {
                duration_ms: 150,
                easing: Scene2DEasing::EaseOutCubic,
                ..Default::default()
            }),
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(1.0, 2.0, 10.0, 10.0),
                fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.1, 0.2, 0.3))),
                stroke: None,
            },
        });

        let wire = serde_json::to_value(&scene).unwrap();
        assert_eq!(wire["version"], 1);
        assert_eq!(wire["revision"], 12);
        assert_eq!(wire["input"]["keyboard"], true);
        assert_eq!(wire["nodes"][0]["id"], "shape-a");
        assert_eq!(wire["nodes"][0]["kind"]["type"], "rect");
        assert_eq!(wire["nodes"][0]["semantic"]["role"], "grid_cell");
        let round_trip: Scene2DScene = serde_json::from_value(wire).unwrap();
        assert_eq!(round_trip, scene);
    }

    #[test]
    fn scene_validation_rejects_duplicate_ids_and_nonfinite_transforms() {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 20.0, 20.0));
        let mut node = Scene2DNode {
            id: "duplicate".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Scene2DTransform::identity(),
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Circle {
                center: ScenePoint::new(5.0, 5.0),
                radius: 3.0,
                fill: Some(Scene2DBrush::solid(Scene2DColor::rgb(0.2, 0.3, 0.4))),
                stroke: None,
            },
        };
        scene.nodes.push(node.clone());
        scene.nodes.push(node.clone());
        assert_eq!(scene.validate().unwrap_err().path(), "nodes[1].id");

        node.transform.translate_x = f32::NAN;
        scene.nodes = vec![node];
        assert_eq!(scene.validate().unwrap_err().path(), "nodes[0].transform");
    }

    #[test]
    fn scene_validation_rejects_excessive_group_depth_before_rendering() {
        let leaf = Scene2DNode {
            id: "leaf".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Scene2DTransform::identity(),
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(0.0, 0.0, 1.0, 1.0),
                fill: None,
                stroke: None,
            },
        };
        let mut nested = leaf;
        for depth in 0..=SCENE2D_MAX_GROUP_DEPTH {
            nested = Scene2DNode {
                id: format!("group-{depth}"),
                hit_id: None,
                hit_bounds: None,
                clip: None,
                shadow: None,
                semantic: None,
                transform: Scene2DTransform::identity(),
                opacity: 1.0,
                transition: None,
                kind: Scene2DNodeKind::Group {
                    children: vec![nested],
                },
            };
        }
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 10.0, 10.0));
        scene.nodes.push(nested);
        let error = scene.validate().unwrap_err();
        assert!(error.message().contains("group nesting"));
    }

    #[test]
    fn scene_validation_rejects_overflowing_composed_group_transforms() {
        let child = Scene2DNode {
            id: "child".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Scene2DTransform::identity(),
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(0.0, 0.0, 1.0, 1.0),
                fill: None,
                stroke: None,
            },
        };
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 10.0, 10.0));
        scene.nodes.push(Scene2DNode {
            id: "outer".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Scene2DTransform {
                scale_x: 1e20,
                ..Scene2DTransform::identity()
            },
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Group {
                children: vec![Scene2DNode {
                    id: "inner".to_owned(),
                    hit_id: None,
                    hit_bounds: None,
                    clip: None,
                    shadow: None,
                    semantic: None,
                    transform: Scene2DTransform {
                        scale_x: 1e20,
                        ..Scene2DTransform::identity()
                    },
                    opacity: 1.0,
                    transition: None,
                    kind: Scene2DNodeKind::Group {
                        children: vec![child],
                    },
                }],
            },
        });
        assert!(
            scene
                .validate()
                .unwrap_err()
                .message()
                .contains("composed transform")
        );
    }

    #[test]
    fn omitted_input_configuration_uses_native_surface_defaults() {
        let scene: Scene2DScene = serde_json::from_value(serde_json::json!({
            "view_box": { "x": 0.0, "y": 0.0, "width": 100.0, "height": 80.0 }
        }))
        .unwrap();

        assert!(scene.input.pointer);
        assert!(!scene.input.continuous);
        assert!(scene.input.capture);
        assert!(scene.input.keyboard);
    }

    #[test]
    fn stroke_only_rectangles_accept_missing_fill() {
        let mut scene = Scene2DScene::new(SceneRect::new(0.0, 0.0, 20.0, 20.0));
        scene.nodes.push(Scene2DNode {
            id: "outline".to_owned(),
            hit_id: None,
            hit_bounds: None,
            clip: None,
            shadow: None,
            semantic: None,
            transform: Scene2DTransform::identity(),
            opacity: 1.0,
            transition: None,
            kind: Scene2DNodeKind::Rect {
                rect: SceneRect::new(1.0, 1.0, 18.0, 18.0),
                fill: None,
                stroke: Some(Scene2DStroke {
                    width: 1.0,
                    color: Scene2DColor::rgb(0.9, 0.9, 0.9),
                }),
            },
        });

        scene.validate().unwrap();
        let wire = serde_json::to_value(&scene).unwrap();
        assert!(wire["nodes"][0]["kind"]["fill"].is_null());
        let round_trip: Scene2DScene = serde_json::from_value(wire).unwrap();
        assert_eq!(round_trip, scene);
    }
}
