use dwrote::OutlineBuilder as DWriteOutlineBuilder;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry :: vector :: { Vector2F } ;
use std::sync::{Arc, Mutex};
use crate :: outline :: { OutlineBuilder } ;
use super::misc::ERROR_BOUND;
use super::types::OutlineCanonicalizerInfo;

#[derive(Clone)]
pub(super) struct OutlineCanonicalizer(pub(super) Arc<Mutex<OutlineCanonicalizerInfo>>);

impl OutlineCanonicalizer {
    pub(super) fn new() -> OutlineCanonicalizer {
        OutlineCanonicalizer(Arc::new(Mutex::new(OutlineCanonicalizerInfo {
            builder: OutlineBuilder::new(),
            last_position: Vector2F::default(),
        })))
    }
}

impl DWriteOutlineBuilder for OutlineCanonicalizer {
    fn move_to(&mut self, to_x: f32, to_y: f32) {
        let to = Vector2F::new(to_x, -to_y);

        let mut this = self.0.lock().unwrap();
        this.last_position = to;
        this.builder.move_to(to);
    }

    fn line_to(&mut self, to_x: f32, to_y: f32) {
        let to = Vector2F::new(to_x, -to_y);

        let mut this = self.0.lock().unwrap();
        this.last_position = to;
        this.builder.line_to(to);
    }

    fn close(&mut self) {
        let mut this = self.0.lock().unwrap();
        this.builder.close();
    }

    fn curve_to(
        &mut self,
        ctrl0_x: f32,
        ctrl0_y: f32,
        ctrl1_x: f32,
        ctrl1_y: f32,
        to_x: f32,
        to_y: f32,
    ) {
        let ctrl = LineSegment2F::new(
            Vector2F::new(ctrl0_x, -ctrl0_y),
            Vector2F::new(ctrl1_x, -ctrl1_y),
        );
        let to = Vector2F::new(to_x, -to_y);

        // This might be a degree-elevated quadratic curve. Try to detect that.
        // See Sederberg § 2.6, "Distance Between Two Bézier Curves".
        let mut this = self.0.lock().unwrap();
        let baseline = LineSegment2F::new(this.last_position, to);
        let approx_ctrl = LineSegment2F((ctrl * 3.0).0 - baseline.0) * 0.5;
        let delta_ctrl = (approx_ctrl.to() - approx_ctrl.from()) * 2.0;
        let max_error = delta_ctrl.length() / 6.0;

        if max_error < ERROR_BOUND {
            // Round to nearest 0.5.
            let approx_ctrl = (approx_ctrl.midpoint() * 2.0).round() * 0.5;
            this.builder.quadratic_curve_to(approx_ctrl, to);
        } else {
            this.builder.cubic_curve_to(ctrl, to);
        }

        this.last_position = to;
    }
}

