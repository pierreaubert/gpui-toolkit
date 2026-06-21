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

#[cfg(test)]
mod tests {
    use super::ProjectStream;
    use crate::geo::projection::Equirectangular;
    use crate::geo::stream::Stream;

    struct RecordingStream {
        events: Vec<String>,
    }

    impl RecordingStream {
        fn new() -> Self {
            Self { events: Vec::new() }
        }
    }

    impl Stream for RecordingStream {
        fn point(&mut self, x: f64, y: f64, m: i32) {
            self.events.push(format!("point {x} {y} {m}"));
        }

        fn line_start(&mut self) {
            self.events.push("line_start".to_string());
        }

        fn line_end(&mut self) {
            self.events.push("line_end".to_string());
        }

        fn polygon_start(&mut self) {
            self.events.push("polygon_start".to_string());
        }

        fn polygon_end(&mut self) {
            self.events.push("polygon_end".to_string());
        }

        fn sphere(&mut self) {
            self.events.push("sphere".to_string());
        }
    }

    #[test]
    fn test_project_stream_point() {
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let sink = RecordingStream::new();
        let mut stream = ProjectStream::new(proj, sink);

        stream.point(0.0, 0.0, 0);
        assert_eq!(stream.sink.events, vec!["point 0 0 0"]);
    }

    #[test]
    fn test_project_stream_forwards_geometry() {
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let sink = RecordingStream::new();
        let mut stream = ProjectStream::new(proj, sink);

        stream.line_start();
        stream.point(10.0, 20.0, 0);
        stream.line_end();
        stream.polygon_start();
        stream.polygon_end();
        stream.sphere();

        assert_eq!(
            stream.sink.events,
            vec![
                "line_start".to_string(),
                format!("point {} {} 0", 10.0f64.to_radians(), -20.0f64.to_radians()),
                "line_end".to_string(),
                "polygon_start".to_string(),
                "polygon_end".to_string(),
                "sphere".to_string(),
            ]
        );
    }

    #[test]
    fn test_project_stream_sinks() {
        let proj = Equirectangular::new();
        let sink = RecordingStream::new();
        let mut stream = ProjectStream::new(proj, sink);

        assert!(stream.sink().events.is_empty());
        stream.sink_mut().events.push("mutated".to_string());
        assert_eq!(stream.sink().events, vec!["mutated"]);
    }

    #[test]
    fn test_project_stream_into_sink() {
        let proj = Equirectangular::new();
        let sink = RecordingStream::new();
        let stream = ProjectStream::new(proj, sink);
        let recovered = stream.into_sink();
        assert!(recovered.events.is_empty());
    }
}
