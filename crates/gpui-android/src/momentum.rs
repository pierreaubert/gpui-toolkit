//! Momentum scrolling (inertia / fling) for touch-based platforms.

use smallvec::SmallVec;
use std::time::Instant;

const DECELERATION_RATE: f32 = 0.998;
const MIN_VELOCITY: f32 = 30.0;
const MAX_VELOCITY: f32 = 16000.0;
const MAX_SAMPLES: usize = 20;
const VELOCITY_WINDOW_SECS: f64 = 0.10;
const MIN_SAMPLES_FOR_VELOCITY: usize = 2;

#[derive(Clone, Copy, Debug)]
struct Sample {
    x: f32,
    y: f32,
    time: Instant,
}

pub struct VelocityTracker {
    samples: [Option<Sample>; MAX_SAMPLES],
    index: usize,
}

impl Default for VelocityTracker {
    fn default() -> Self {
        Self {
            samples: [None; MAX_SAMPLES],
            index: 0,
        }
    }
}

impl VelocityTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, x: f32, y: f32) {
        self.samples[self.index] = Some(Sample {
            x,
            y,
            time: Instant::now(),
        });
        self.index = (self.index + 1) % MAX_SAMPLES;
    }

    pub fn velocity(&self) -> (f32, f32) {
        let now = Instant::now();
        let mut recent: SmallVec<[&Sample; MAX_SAMPLES]> = SmallVec::new();
        for s in self.samples.iter().flatten() {
            let age = now.duration_since(s.time).as_secs_f64();
            if age <= VELOCITY_WINDOW_SECS {
                recent.push(s);
            }
        }
        if recent.len() < MIN_SAMPLES_FOR_VELOCITY {
            return (0.0, 0.0);
        }
        recent.sort_by_key(|a| a.time);
        if recent.len() >= 3 {
            let (wvx, wvy) = weighted_velocity(&recent);
            return (clamp_velocity(wvx), clamp_velocity(wvy));
        }
        let first = recent[0];
        let last = recent[recent.len() - 1];
        let dt = last.time.duration_since(first.time).as_secs_f64();
        if dt < 1e-6 {
            return (0.0, 0.0);
        }
        let vx = ((last.x - first.x) as f64 / dt) as f32;
        let vy = ((last.y - first.y) as f64 / dt) as f32;
        (clamp_velocity(vx), clamp_velocity(vy))
    }

    pub fn reset(&mut self) {
        self.samples = [None; MAX_SAMPLES];
        self.index = 0;
    }
}

fn weighted_velocity(samples: &[&Sample]) -> (f32, f32) {
    if samples.len() < 2 {
        return (0.0, 0.0);
    }
    let t0 = samples[0].time;
    let n = samples.len();
    let mut sum_w = 0.0_f64;
    let mut sum_wt = 0.0_f64;
    let mut sum_wt2 = 0.0_f64;
    let mut sum_wx = 0.0_f64;
    let mut sum_wy = 0.0_f64;
    let mut sum_wtx = 0.0_f64;
    let mut sum_wty = 0.0_f64;
    for (i, s) in samples.iter().enumerate() {
        let t = s.time.duration_since(t0).as_secs_f64();
        let w = (2.0 * i as f64 / n as f64).exp();
        sum_w += w;
        sum_wt += w * t;
        sum_wt2 += w * t * t;
        sum_wx += w * s.x as f64;
        sum_wy += w * s.y as f64;
        sum_wtx += w * t * s.x as f64;
        sum_wty += w * t * s.y as f64;
    }
    let denom = sum_w * sum_wt2 - sum_wt * sum_wt;
    if denom.abs() < 1e-12 {
        let first = samples[0];
        let last = samples[n - 1];
        let dt = last.time.duration_since(first.time).as_secs_f64();
        if dt < 1e-6 {
            return (0.0, 0.0);
        }
        return (
            ((last.x - first.x) as f64 / dt) as f32,
            ((last.y - first.y) as f64 / dt) as f32,
        );
    }
    let vx = (sum_w * sum_wtx - sum_wt * sum_wx) / denom;
    let vy = (sum_w * sum_wty - sum_wt * sum_wy) / denom;
    (vx as f32, vy as f32)
}

fn clamp_velocity(v: f32) -> f32 {
    v.clamp(-MAX_VELOCITY, MAX_VELOCITY)
}

pub struct MomentumScroller {
    vx: f32,
    vy: f32,
    last_x: f32,
    last_y: f32,
    active: bool,
    last_time: Instant,
}

impl Default for MomentumScroller {
    fn default() -> Self {
        Self {
            vx: 0.0,
            vy: 0.0,
            last_x: 0.0,
            last_y: 0.0,
            active: false,
            last_time: Instant::now(),
        }
    }
}

impl MomentumScroller {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn fling(&mut self, vx: f32, vy: f32, last_x: f32, last_y: f32) {
        let speed = (vx * vx + vy * vy).sqrt();
        if speed < MIN_VELOCITY {
            self.active = false;
            return;
        }
        self.vx = vx;
        self.vy = vy;
        self.last_x = last_x;
        self.last_y = last_y;
        self.active = true;
        self.last_time = Instant::now();
    }

    pub fn step(&mut self) -> Option<MomentumDelta> {
        if !self.active {
            return None;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f64() as f32;
        self.last_time = now;
        let dt = dt.min(0.033);
        if dt < 1e-6 {
            return None;
        }
        let dt_ms = dt * 1000.0;
        let decay = DECELERATION_RATE.powf(dt_ms);
        let ln_r = DECELERATION_RATE.ln();
        let displacement_factor = if ln_r.abs() > 1e-9 {
            (decay - 1.0) / (ln_r * 1000.0)
        } else {
            dt
        };
        let dx = self.vx * displacement_factor;
        let dy = self.vy * displacement_factor;
        self.last_x += dx;
        self.last_y += dy;
        self.vx *= decay;
        self.vy *= decay;
        let speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
        if speed < MIN_VELOCITY {
            self.active = false;
            if dx.abs() < 0.1 && dy.abs() < 0.1 {
                return None;
            }
        }
        Some(MomentumDelta {
            dx,
            dy,
            position_x: self.last_x,
            position_y: self.last_y,
        })
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.vx = 0.0;
        self.vy = 0.0;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Returns true if the scroller has naturally decelerated to a stop.
    /// This is only meaningful when called after `step()` returned `None`
    /// while the scroller was previously active.
    pub fn is_finished(&self) -> bool {
        !self.active
    }

    pub fn position_x(&self) -> f32 {
        self.last_x
    }

    pub fn position_y(&self) -> f32 {
        self.last_y
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MomentumDelta {
    pub dx: f32,
    pub dy: f32,
    pub position_x: f32,
    pub position_y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_momentum_scroller_is_finished() {
        let mut scroller = MomentumScroller::new();
        assert!(scroller.is_finished());

        scroller.fling(100.0, 0.0, 0.0, 0.0);
        assert!(!scroller.is_finished());

        // Exhaust the fling
        while scroller.is_active() {
            scroller.last_time = Instant::now() - Duration::from_millis(33);
            let _ = scroller.step();
        }
        assert!(scroller.is_finished());
    }

    #[test]
    fn test_step_may_return_none_without_finishing() {
        let mut scroller = MomentumScroller::new();
        scroller.fling(100.0, 0.0, 0.0, 0.0);

        // First step should produce a delta
        scroller.last_time = Instant::now() - Duration::from_millis(16);
        assert!(scroller.step().is_some());

        // A sub-microsecond step returns None
        // without the scroller being finished.
        scroller.last_time = Instant::now();
        let result = scroller.step();
        assert!(result.is_none());
        assert!(
            scroller.is_active(),
            "scroller should still be active after sub-microsecond dt"
        );
        assert!(
            !scroller.is_finished(),
            "scroller should not be finished after sub-microsecond dt"
        );
    }

    #[test]
    fn momentum_position_advances_with_each_step() {
        let mut scroller = MomentumScroller::new();
        scroller.fling(1_000.0, 500.0, 10.0, 20.0);

        scroller.last_time = Instant::now() - Duration::from_millis(16);
        let first = scroller.step().expect("first momentum step");
        assert!(first.position_x > 10.0);
        assert!(first.position_y > 20.0);

        scroller.last_time = Instant::now() - Duration::from_millis(16);
        let second = scroller.step().expect("second momentum step");
        assert!(second.position_x > first.position_x);
        assert!(second.position_y > first.position_y);
    }

    #[test]
    fn velocity_tracker_computes_release_velocity() {
        let mut tracker = VelocityTracker::new();
        tracker.record(0.0, 0.0);
        thread::sleep(Duration::from_millis(8));
        tracker.record(50.0, 0.0);
        thread::sleep(Duration::from_millis(8));
        tracker.record(100.0, 0.0);

        let (vx, vy) = tracker.velocity();
        assert!(vx > 0.0, "velocity should be positive after rightward drag");
        assert_eq!(vy, 0.0);
        assert!(vx.abs() <= MAX_VELOCITY);
    }
}
