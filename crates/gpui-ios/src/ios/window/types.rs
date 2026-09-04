use gpui::{AtlasKey, AtlasTile};
use std::collections::HashMap;

/// Tracks the current touch gesture state machine.
///
/// This distinguishes taps (short, stationary touches) from scroll gestures
/// (finger drags). The same pattern is used on Android.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum TouchState {
    /// No active touch.
    Idle,
    /// Finger is down but hasn't moved beyond the slop threshold.
    Pending { start_x: f32, start_y: f32 },
    /// Finger has moved beyond the threshold — we are scrolling.
    Scrolling { prev_x: f32, prev_y: f32 },
    /// GPUI consumed the MouseDown (e.g. a drag handler) — only emit MouseMove,
    /// no ScrollWheel events, so the element can drive its own drag logic.
    Dragging,
}

const MAX_TOUCHES: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TouchPointState {
    pub id: usize,
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TouchEntry {
    id: usize,
    state: TouchState,
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct PinchState {
    active: bool,
    last_distance: f32,
}

impl PinchState {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn start(&mut self, distance: f32) {
        self.active = true;
        self.last_distance = distance;
    }

    pub fn update(&mut self, distance: f32) -> Option<f32> {
        if !self.active || self.last_distance <= f32::EPSILON || distance <= f32::EPSILON {
            self.start(distance);
            return Some(0.0);
        }

        let delta = distance / self.last_distance - 1.0;
        self.last_distance = distance;
        delta.is_finite().then_some(delta)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub(super) fn pinch_geometry(
    first: TouchPointState,
    second: TouchPointState,
) -> Option<(f32, f32, f32)> {
    let dx = second.x - first.x;
    let dy = second.y - first.y;
    let distance = (dx * dx + dy * dy).sqrt();
    (distance > f32::EPSILON).then_some((
        (first.x + second.x) * 0.5,
        (first.y + second.y) * 0.5,
        distance,
    ))
}

/// Small fixed-size map for active touches.
///
/// iOS supports at most a handful of simultaneous touches; a linear-scan
/// array avoids the per-event heap traffic of a `HashMap`.
#[derive(Clone, Debug)]
pub(super) struct TouchStateMap {
    entries: [Option<TouchEntry>; MAX_TOUCHES],
}

impl Default for TouchStateMap {
    fn default() -> Self {
        Self {
            entries: [None; MAX_TOUCHES],
        }
    }
}

impl TouchStateMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, touch_id: usize) -> Option<TouchState> {
        self.entries
            .iter()
            .flatten()
            .find(|entry| entry.id == touch_id)
            .map(|entry| entry.state)
    }

    pub fn insert(&mut self, touch_id: usize, state: TouchState, x: f32, y: f32) {
        for existing in self.entries.iter_mut().flatten() {
            if existing.id == touch_id {
                existing.state = state;
                existing.x = x;
                existing.y = y;
                return;
            }
        }
        for entry in &mut self.entries {
            if entry.is_none() {
                *entry = Some(TouchEntry {
                    id: touch_id,
                    state,
                    x,
                    y,
                });
                return;
            }
        }
        // All slots full — overwrite the oldest (first) entry. This should be
        // extremely rare on iOS, which supports at most ~5 simultaneous touches.
        self.entries[0] = Some(TouchEntry {
            id: touch_id,
            state,
            x,
            y,
        });
    }

    pub fn remove(&mut self, touch_id: usize) -> Option<TouchState> {
        for entry in &mut self.entries {
            if entry
                .as_ref()
                .is_some_and(|existing| existing.id == touch_id)
            {
                return entry.take().map(|entry| entry.state);
            }
        }
        None
    }

    pub fn clear_states(&mut self) {
        for entry in self.entries.iter_mut().flatten() {
            entry.state = TouchState::Idle;
        }
    }

    pub fn two_active_points(&self) -> Option<(TouchPointState, TouchPointState)> {
        let mut points = self.entries.iter().flatten().map(|entry| TouchPointState {
            id: entry.id,
            x: entry.x,
            y: entry.y,
        });
        let first = points.next()?;
        let second = points.next()?;
        points.next().is_none().then_some((first, second))
    }
}

pub(super) struct FallbackAtlasState {
    pub(super) next_id: u32,
    pub(super) tiles: HashMap<AtlasKey, AtlasTile>,
    /// Insertion/recency order for LRU eviction, front = oldest.
    pub(super) order: std::collections::VecDeque<AtlasKey>,
}

impl FallbackAtlasState {
    /// Maximum retained tiles; the fallback atlas never uploads to the GPU,
    /// so this only bounds host memory.
    pub(super) const MAX_TILES: usize = 4096;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_state_map_stores_and_removes_touches() {
        let mut map = TouchStateMap::new();
        assert_eq!(map.get(1), None);

        map.insert(
            1,
            TouchState::Pending {
                start_x: 0.0,
                start_y: 0.0,
            },
            0.0,
            0.0,
        );
        assert!(matches!(map.get(1), Some(TouchState::Pending { .. })));

        map.insert(1, TouchState::Dragging, 1.0, 1.0);
        assert_eq!(map.get(1), Some(TouchState::Dragging));

        let removed = map.remove(1);
        assert_eq!(removed, Some(TouchState::Dragging));
        assert_eq!(map.get(1), None);
    }

    #[test]
    fn touch_state_map_overwrites_oldest_when_full() {
        let mut map = TouchStateMap::new();
        for i in 0..MAX_TOUCHES {
            map.insert(i, TouchState::Dragging, i as f32, i as f32);
        }
        // Adding one more should evict the oldest slot (id 0).
        map.insert(
            MAX_TOUCHES,
            TouchState::Pending {
                start_x: 1.0,
                start_y: 2.0,
            },
            1.0,
            2.0,
        );
        assert_eq!(map.get(0), None);
        assert!(matches!(
            map.get(MAX_TOUCHES),
            Some(TouchState::Pending {
                start_x: 1.0,
                start_y: 2.0
            })
        ));
    }

    #[test]
    fn touch_state_map_reports_exactly_two_active_points() {
        let mut map = TouchStateMap::new();
        assert_eq!(map.two_active_points(), None);

        map.insert(
            1,
            TouchState::Pending {
                start_x: 0.0,
                start_y: 0.0,
            },
            10.0,
            20.0,
        );
        assert_eq!(map.two_active_points(), None);

        map.insert(
            2,
            TouchState::Pending {
                start_x: 0.0,
                start_y: 0.0,
            },
            30.0,
            40.0,
        );
        let (first, second) = map.two_active_points().unwrap();
        assert_eq!(first.id, 1);
        assert_eq!(second.id, 2);

        map.insert(3, TouchState::Dragging, 50.0, 60.0);
        assert_eq!(map.two_active_points(), None);
    }

    #[test]
    fn pinch_geometry_returns_centroid_and_distance() {
        let first = TouchPointState {
            id: 1,
            x: 0.0,
            y: 0.0,
        };
        let second = TouchPointState {
            id: 2,
            x: 6.0,
            y: 8.0,
        };
        let (x, y, distance) = pinch_geometry(first, second).unwrap();
        assert_eq!(x, 3.0);
        assert_eq!(y, 4.0);
        assert_eq!(distance, 10.0);
    }

    #[test]
    fn pinch_state_reports_incremental_delta() {
        let mut pinch = PinchState::default();
        pinch.start(100.0);
        assert_eq!(pinch.update(125.0), Some(0.25));
        assert_eq!(pinch.update(100.0), Some(-0.19999999));
        pinch.reset();
        assert!(!pinch.is_active());
    }
}
