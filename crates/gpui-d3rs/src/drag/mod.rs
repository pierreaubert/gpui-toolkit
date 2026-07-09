//! Renderer-independent drag gesture state (d3-drag inspired).
//!
//! This module does not bind to GPUI events directly. Hosts feed pointer
//! coordinates into [`DragState`] and render or dispatch the returned
//! [`DragUpdate`] values in their own event layer.

/// A 2D pointer position in local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragPoint {
    pub x: f64,
    pub y: f64,
}

impl DragPoint {
    /// Create a point after validating both coordinates are finite.
    pub fn try_new(x: f64, y: f64) -> Result<Self, DragError> {
        if !x.is_finite() {
            return Err(DragError::NonFiniteCoordinate {
                axis: "x",
                value: x,
            });
        }
        if !y.is_finite() {
            return Err(DragError::NonFiniteCoordinate {
                axis: "y",
                value: y,
            });
        }
        Ok(Self { x, y })
    }

    /// Return the vector from `other` to this point.
    pub fn delta_from(self, other: Self) -> DragDelta {
        DragDelta {
            dx: self.x - other.x,
            dy: self.y - other.y,
        }
    }
}

/// A 2D movement delta.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragDelta {
    pub dx: f64,
    pub dy: f64,
}

impl DragDelta {
    /// Euclidean length of the delta.
    pub fn length(self) -> f64 {
        self.length_squared().sqrt()
    }

    /// Squared Euclidean length of the delta.
    pub fn length_squared(self) -> f64 {
        self.dx.mul_add(self.dx, self.dy * self.dy)
    }
}

/// Optional local bounds for clamping drag positions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragExtent {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl DragExtent {
    /// Create an extent after validating finite ordered bounds.
    pub fn try_new(x0: f64, y0: f64, x1: f64, y1: f64) -> Result<Self, DragError> {
        let min = DragPoint::try_new(x0, y0)?;
        let max = DragPoint::try_new(x1, y1)?;
        if min.x > max.x {
            return Err(DragError::InvalidExtent {
                reason: "x0 must be <= x1",
            });
        }
        if min.y > max.y {
            return Err(DragError::InvalidExtent {
                reason: "y0 must be <= y1",
            });
        }
        Ok(Self { x0, y0, x1, y1 })
    }

    /// Clamp a point into this extent.
    pub fn clamp(self, point: DragPoint) -> DragPoint {
        DragPoint {
            x: point.x.clamp(self.x0, self.x1),
            y: point.y.clamp(self.y0, self.y1),
        }
    }
}

/// Drag gesture configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragConfig {
    /// Minimum movement distance before a host should treat the gesture as a drag.
    pub click_distance: f64,
    /// Optional local-coordinate clamp extent.
    pub extent: Option<DragExtent>,
}

impl DragConfig {
    /// Validate configuration values.
    pub fn validate(self) -> Result<Self, DragError> {
        if !self.click_distance.is_finite() || self.click_distance < 0.0 {
            return Err(DragError::InvalidClickDistance(self.click_distance));
        }
        Ok(self)
    }

    /// Set the minimum movement distance for drag recognition.
    pub fn with_click_distance(mut self, click_distance: f64) -> Result<Self, DragError> {
        self.click_distance = click_distance;
        self.validate()
    }

    /// Set a local-coordinate clamp extent.
    pub fn with_extent(mut self, extent: DragExtent) -> Self {
        self.extent = Some(extent);
        self
    }

    fn clamp(self, point: DragPoint) -> DragPoint {
        self.extent
            .map(|extent| extent.clamp(point))
            .unwrap_or(point)
    }
}

impl Default for DragConfig {
    fn default() -> Self {
        Self {
            click_distance: 0.0,
            extent: None,
        }
    }
}

/// Phase of a drag update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragPhase {
    Start,
    Drag,
    End,
    Cancel,
}

/// One drag update emitted by [`DragState`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragUpdate {
    pub phase: DragPhase,
    pub pointer_id: u64,
    pub start: DragPoint,
    pub previous: DragPoint,
    pub current: DragPoint,
    pub delta: DragDelta,
    pub total_delta: DragDelta,
    pub distance: f64,
    pub exceeds_click_distance: bool,
}

impl DragUpdate {
    fn new(
        phase: DragPhase,
        pointer_id: u64,
        start: DragPoint,
        previous: DragPoint,
        current: DragPoint,
        click_distance: f64,
    ) -> Self {
        let delta = current.delta_from(previous);
        let total_delta = current.delta_from(start);
        let distance = total_delta.length();
        Self {
            phase,
            pointer_id,
            start,
            previous,
            current,
            delta,
            total_delta,
            distance,
            exceeds_click_distance: distance >= click_distance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ActiveDrag {
    pointer_id: u64,
    start: DragPoint,
    previous: DragPoint,
    current: DragPoint,
}

/// State machine for one active pointer drag.
#[derive(Debug, Clone, PartialEq)]
pub struct DragState {
    config: DragConfig,
    active: Option<ActiveDrag>,
}

impl DragState {
    /// Create a drag state with default configuration.
    pub fn new() -> Self {
        Self::with_config(DragConfig::default()).expect("default drag config is valid")
    }

    /// Create a drag state with validated configuration.
    pub fn with_config(config: DragConfig) -> Result<Self, DragError> {
        Ok(Self {
            config: config.validate()?,
            active: None,
        })
    }

    /// Return the current configuration.
    pub fn config(&self) -> DragConfig {
        self.config
    }

    /// Start a drag gesture for one pointer id.
    pub fn start(&mut self, pointer_id: u64, x: f64, y: f64) -> Result<DragUpdate, DragError> {
        if let Some(active) = self.active {
            return Err(DragError::AlreadyActive {
                pointer_id: active.pointer_id,
            });
        }

        let point = self.config.clamp(DragPoint::try_new(x, y)?);
        let active = ActiveDrag {
            pointer_id,
            start: point,
            previous: point,
            current: point,
        };
        self.active = Some(active);

        Ok(DragUpdate::new(
            DragPhase::Start,
            pointer_id,
            point,
            point,
            point,
            self.config.click_distance,
        ))
    }

    /// Update the active drag gesture.
    pub fn drag(&mut self, pointer_id: u64, x: f64, y: f64) -> Result<DragUpdate, DragError> {
        let point = self.config.clamp(DragPoint::try_new(x, y)?);
        let active = self.active_mut(pointer_id)?;
        let previous = active.current;
        active.previous = previous;
        active.current = point;

        Ok(DragUpdate::new(
            DragPhase::Drag,
            pointer_id,
            active.start,
            previous,
            point,
            self.config.click_distance,
        ))
    }

    /// End the active drag gesture and clear the state.
    pub fn end(&mut self, pointer_id: u64, x: f64, y: f64) -> Result<DragUpdate, DragError> {
        let point = self.config.clamp(DragPoint::try_new(x, y)?);
        let active = self.active_for(pointer_id)?;
        let update = DragUpdate::new(
            DragPhase::End,
            pointer_id,
            active.start,
            active.current,
            point,
            self.config.click_distance,
        );
        self.active = None;
        Ok(update)
    }

    /// Cancel the active drag gesture and clear the state.
    pub fn cancel(&mut self, pointer_id: u64) -> Result<DragUpdate, DragError> {
        let active = self.active_for(pointer_id)?;
        let update = DragUpdate::new(
            DragPhase::Cancel,
            pointer_id,
            active.start,
            active.current,
            active.current,
            self.config.click_distance,
        );
        self.active = None;
        Ok(update)
    }

    /// Return whether a drag is currently active.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Return the active pointer id, if any.
    pub fn active_pointer_id(&self) -> Option<u64> {
        self.active.map(|active| active.pointer_id)
    }

    /// Return a snapshot of the current active drag.
    pub fn current_update(&self) -> Option<DragUpdate> {
        let active = self.active?;
        Some(DragUpdate::new(
            DragPhase::Drag,
            active.pointer_id,
            active.start,
            active.previous,
            active.current,
            self.config.click_distance,
        ))
    }

    fn active_for(&self, pointer_id: u64) -> Result<ActiveDrag, DragError> {
        let active = self.active.ok_or(DragError::Inactive)?;
        if active.pointer_id != pointer_id {
            return Err(DragError::PointerMismatch {
                active: active.pointer_id,
                received: pointer_id,
            });
        }
        Ok(active)
    }

    fn active_mut(&mut self, pointer_id: u64) -> Result<&mut ActiveDrag, DragError> {
        let active = self.active.as_mut().ok_or(DragError::Inactive)?;
        if active.pointer_id != pointer_id {
            return Err(DragError::PointerMismatch {
                active: active.pointer_id,
                received: pointer_id,
            });
        }
        Ok(active)
    }
}

impl Default for DragState {
    fn default() -> Self {
        Self::new()
    }
}

/// Recoverable drag configuration or lifecycle errors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DragError {
    NonFiniteCoordinate { axis: &'static str, value: f64 },
    InvalidExtent { reason: &'static str },
    InvalidClickDistance(f64),
    AlreadyActive { pointer_id: u64 },
    Inactive,
    PointerMismatch { active: u64, received: u64 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_state_tracks_lifecycle_and_deltas() {
        let config = DragConfig::default().with_click_distance(5.0).unwrap();
        let mut drag = DragState::with_config(config).unwrap();

        let start = drag.start(7, 10.0, 20.0).unwrap();
        assert_eq!(start.phase, DragPhase::Start);
        assert_eq!(start.current, DragPoint { x: 10.0, y: 20.0 });
        assert!(!start.exceeds_click_distance);
        assert!(drag.is_active());

        let update = drag.drag(7, 13.0, 24.0).unwrap();
        assert_eq!(update.phase, DragPhase::Drag);
        assert_eq!(update.delta, DragDelta { dx: 3.0, dy: 4.0 });
        assert_eq!(update.total_delta, DragDelta { dx: 3.0, dy: 4.0 });
        assert_eq!(update.distance, 5.0);
        assert!(update.exceeds_click_distance);

        let end = drag.end(7, 15.0, 29.0).unwrap();
        assert_eq!(end.phase, DragPhase::End);
        assert_eq!(end.delta, DragDelta { dx: 2.0, dy: 5.0 });
        assert_eq!(end.total_delta, DragDelta { dx: 5.0, dy: 9.0 });
        assert!(!drag.is_active());
    }

    #[test]
    fn drag_state_clamps_positions_to_extent() {
        let extent = DragExtent::try_new(0.0, 0.0, 100.0, 50.0).unwrap();
        let config = DragConfig::default().with_extent(extent);
        let mut drag = DragState::with_config(config).unwrap();

        let start = drag.start(1, -10.0, 10.0).unwrap();
        assert_eq!(start.current, DragPoint { x: 0.0, y: 10.0 });

        let update = drag.drag(1, 120.0, -5.0).unwrap();
        assert_eq!(update.current, DragPoint { x: 100.0, y: 0.0 });
        assert_eq!(
            update.total_delta,
            DragDelta {
                dx: 100.0,
                dy: -10.0
            }
        );
    }

    #[test]
    fn drag_state_rejects_invalid_inputs() {
        match DragPoint::try_new(f64::NAN, 0.0) {
            Err(DragError::NonFiniteCoordinate { axis: "x", value }) => {
                assert!(value.is_nan());
            }
            result => panic!("unexpected non-finite result: {result:?}"),
        }
        assert_eq!(
            DragExtent::try_new(10.0, 0.0, 0.0, 10.0),
            Err(DragError::InvalidExtent {
                reason: "x0 must be <= x1"
            })
        );
        assert_eq!(
            DragConfig::default().with_click_distance(-1.0),
            Err(DragError::InvalidClickDistance(-1.0))
        );
    }

    #[test]
    fn drag_state_enforces_pointer_identity_without_mutation() {
        let mut drag = DragState::new();
        drag.start(42, 0.0, 0.0).unwrap();

        assert_eq!(
            drag.drag(9, 1.0, 1.0),
            Err(DragError::PointerMismatch {
                active: 42,
                received: 9
            })
        );
        let current = drag.current_update().unwrap();
        assert_eq!(current.current, DragPoint { x: 0.0, y: 0.0 });

        assert_eq!(
            drag.start(43, 0.0, 0.0),
            Err(DragError::AlreadyActive { pointer_id: 42 })
        );
        assert!(drag.cancel(42).is_ok());
        assert!(!drag.is_active());
    }

    #[test]
    fn drag_state_reports_inactive_lifecycle_errors() {
        let mut drag = DragState::new();

        assert_eq!(drag.drag(1, 0.0, 0.0), Err(DragError::Inactive));
        assert_eq!(drag.end(1, 0.0, 0.0), Err(DragError::Inactive));
        assert_eq!(drag.cancel(1), Err(DragError::Inactive));
    }
}
