//! Antimeridian pre-clip stream stage.
//!
//! Buffers polygon rings until `polygon_end`, allowing the polygon-level clipper
//! to reconnect antimeridian segments while preserving holes and winding.

use crate::geo::path::clip::clip_antimeridian_polygon;
use crate::geo::path::geo_path::antimeridian_clip_line;
use crate::geo::projection::SphereRotation;
use crate::geo::stream::Stream;

/// Pre-clip stream stage that cuts geometry at the antimeridian in the
/// projection's rotated frame.
///
pub struct PreclipAntimeridianStream<S: Stream> {
    rotation: SphereRotation,
    sink: S,
    buffer: Vec<(f64, f64)>,
    polygon_rings: Vec<Vec<(f64, f64)>>,
    in_line: bool,
    in_polygon: bool,
}

impl<S: Stream> PreclipAntimeridianStream<S> {
    pub fn new(rotation: SphereRotation, sink: S) -> Self {
        Self {
            rotation,
            sink,
            buffer: Vec::new(),
            polygon_rings: Vec::new(),
            in_line: false,
            in_polygon: false,
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
        self.buffer.clear();
    }

    fn line_end(&mut self) {
        self.in_line = false;
        if self.in_polygon {
            if !self.buffer.is_empty() {
                self.polygon_rings.push(std::mem::take(&mut self.buffer));
            }
        } else {
            for piece in antimeridian_clip_line(&self.buffer, &self.rotation) {
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
    }

    fn polygon_start(&mut self) {
        self.in_polygon = true;
        self.polygon_rings.clear();
        self.sink.polygon_start();
    }

    fn polygon_end(&mut self) {
        for ring in clip_antimeridian_polygon(&self.polygon_rings, &self.rotation) {
            if ring.is_empty() {
                continue;
            }
            self.sink.line_start();
            for (lon, lat) in ring {
                self.sink.point(lon, lat, 0);
            }
            self.sink.line_end();
        }
        self.polygon_rings.clear();
        self.in_polygon = false;
        self.sink.polygon_end();
    }

    fn sphere(&mut self) {
        self.sink.sphere();
    }
}

#[cfg(test)]
mod tests {
    use super::PreclipAntimeridianStream;
    use crate::geo::projection::SphereRotation;
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
    fn test_preclip_point_forwards_directly() {
        let rotation = SphereRotation::identity();
        let sink = RecordingStream::new();
        let mut stream = PreclipAntimeridianStream::new(rotation, sink);

        stream.point(10.0, 20.0, 0);
        assert_eq!(stream.sink.events, vec!["point 10 20 0"]);
    }

    #[test]
    fn test_preclip_line_no_crossing() {
        let rotation = SphereRotation::identity();
        let sink = RecordingStream::new();
        let mut stream = PreclipAntimeridianStream::new(rotation, sink);

        stream.line_start();
        stream.point(0.0, 0.0, 0);
        stream.point(10.0, 10.0, 0);
        stream.line_end();

        assert!(stream.sink.events.contains(&"line_start".to_string()));
        assert!(stream.sink.events.contains(&"line_end".to_string()));
        assert_eq!(
            stream
                .sink
                .events
                .iter()
                .filter(|e| e.starts_with("point"))
                .count(),
            2
        );
    }

    #[test]
    fn test_preclip_polygon_ring() {
        let rotation = SphereRotation::identity();
        let sink = RecordingStream::new();
        let mut stream = PreclipAntimeridianStream::new(rotation, sink);

        stream.polygon_start();
        stream.line_start();
        stream.point(0.0, 0.0, 0);
        stream.point(10.0, 0.0, 0);
        stream.point(10.0, 10.0, 0);
        stream.line_end();
        stream.polygon_end();

        assert!(stream.sink.events.contains(&"polygon_start".to_string()));
        assert!(stream.sink.events.contains(&"polygon_end".to_string()));
        assert!(
            stream
                .sink
                .events
                .iter()
                .filter(|e| e == &"line_start")
                .count()
                >= 1
        );
    }

    #[test]
    fn preclip_buffers_all_polygon_rings_until_polygon_end() {
        let rotation = SphereRotation::identity();
        let sink = RecordingStream::new();
        let mut stream = PreclipAntimeridianStream::new(rotation, sink);

        stream.polygon_start();
        stream.line_start();
        stream.point(170.0, -20.0, 0);
        stream.point(-170.0, -20.0, 0);
        stream.point(-170.0, 20.0, 0);
        stream.point(170.0, 20.0, 0);
        stream.line_end();
        assert_eq!(
            stream
                .sink
                .events
                .iter()
                .filter(|event| event.as_str() == "line_start")
                .count(),
            0,
            "polygon fragments must not be emitted before all rings are known"
        );
        stream.polygon_end();

        assert!(
            stream
                .sink
                .events
                .iter()
                .filter(|event| event.as_str() == "line_start")
                .count()
                >= 1
        );
    }

    #[test]
    fn test_preclip_sphere_forwards() {
        let rotation = SphereRotation::identity();
        let sink = RecordingStream::new();
        let mut stream = PreclipAntimeridianStream::new(rotation, sink);

        stream.sphere();
        assert_eq!(stream.sink.events, vec!["sphere"]);
    }

    #[test]
    fn test_preclip_into_sink() {
        let rotation = SphereRotation::identity();
        let sink = RecordingStream::new();
        let stream = PreclipAntimeridianStream::new(rotation, sink);
        let recovered = stream.into_sink();
        assert!(recovered.events.is_empty());
    }
}
