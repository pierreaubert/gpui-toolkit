//! Antimeridian pre-clip stream stage.
//!
//! Wraps the existing antimeridian cutter (`geo_path::antimeridian_clip_*`) as a
//! `Stream` transformer so it can sit in front of the projection stage. This is a
//! temporary bridge: once the D3-style stream clipper is fully debugged it will
//! replace this stage, but for now it restores the cylindrical golden pass while
//! we build out the new pipeline.

use crate::geo::path::geo_path::{antimeridian_clip_line, antimeridian_clip_ring};
use crate::geo::projection::SphereRotation;
use crate::geo::stream::Stream;

/// Pre-clip stream stage that cuts geometry at the antimeridian in the
/// projection's rotated frame.
///
/// This currently uses the existing per-ring antimeridian cutter as a bridge
/// while the D3-style polygon-level clipper is being debugged.
pub struct PreclipAntimeridianStream<S: Stream> {
    rotation: SphereRotation,
    sink: S,
    buffer: Vec<(f64, f64)>,
    in_line: bool,
    is_ring: bool,
}

impl<S: Stream> PreclipAntimeridianStream<S> {
    pub fn new(rotation: SphereRotation, sink: S) -> Self {
        Self {
            rotation,
            sink,
            buffer: Vec::new(),
            in_line: false,
            is_ring: false,
        }
    }

    pub fn into_sink(self) -> S {
        self.sink
    }
}

impl<S: Stream> Stream for PreclipAntimeridianStream<S> {
    fn point(&mut self, lon: f64, lat: f64, m: i32) {
        if self.in_line {
            self.buffer.push((lon, lat));
        } else {
            // Point or MultiPoint feature: forward without antimeridian clipping.
            self.sink.point(lon, lat, m);
        }
    }

    fn line_start(&mut self) {
        self.in_line = true;
        self.is_ring = false;
        self.buffer.clear();
    }

    fn line_end(&mut self) {
        self.in_line = false;
        let pieces = if self.is_ring {
            antimeridian_clip_ring(&self.buffer, &self.rotation)
        } else {
            antimeridian_clip_line(&self.buffer, &self.rotation)
        };

        for piece in pieces {
            if piece.is_empty() {
                continue;
            }
            self.sink.line_start();
            for &(lon, lat) in &piece {
                self.sink.point(lon, lat, 0);
            }
            self.sink.line_end();
        }
    }

    fn polygon_start(&mut self) {
        self.is_ring = true;
        self.sink.polygon_start();
    }

    fn polygon_end(&mut self) {
        self.is_ring = false;
        self.sink.polygon_end();
    }

    fn sphere(&mut self) {
        self.sink.sphere();
    }
}
