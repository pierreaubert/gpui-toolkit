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
        assert!(stream
            .sink
            .events
            .iter()
            .filter(|e| e == &"line_start")
            .count()
            >= 1);
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
