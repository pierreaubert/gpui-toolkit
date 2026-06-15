//! Projection stream stage.
//!
//! Mirrors the project + scale/translate stage of `d3-geo/src/projection/index.js`.
//! This stage receives geographic coordinates in degrees (the convention used by
//! the GeoJSON stream) and forwards projected planar coordinates to the next
//! stream stage.

use crate::geo::projection::Projection;
use crate::geo::stream::Stream;

/// Stream stage that projects incoming (lon, lat) points using a `Projection`
/// and forwards the planar result to the downstream sink.
pub struct ProjectStream<P: Projection, S: Stream> {
    projection: P,
    sink: S,
}

impl<P: Projection, S: Stream> ProjectStream<P, S> {
    pub fn new(projection: P, sink: S) -> Self {
        Self { projection, sink }
    }

    pub fn into_sink(self) -> S {
        self.sink
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }
}

impl<P: Projection, S: Stream> Stream for ProjectStream<P, S> {
    fn point(&mut self, lon: f64, lat: f64, m: i32) {
        let (x, y) = self.projection.project(lon, lat);
        self.sink.point(x, y, m);
    }

    fn line_start(&mut self) {
        self.sink.line_start();
    }

    fn line_end(&mut self) {
        self.sink.line_end();
    }

    fn polygon_start(&mut self) {
        self.sink.polygon_start();
    }

    fn polygon_end(&mut self) {
        self.sink.polygon_end();
    }

    fn sphere(&mut self) {
        self.sink.sphere();
    }
}
