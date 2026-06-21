//! D3-style geometry streaming.
//!
//! This module mirrors `d3-geo/src/stream.js`. A `Stream` receives geographic
//! features as a sequence of `point`, `lineStart`, `lineEnd`, `polygonStart`,
//! `polygonEnd`, and `sphere` calls. Higher-level operations (path rendering,
//! bounds) are implemented as stream sinks.

use super::path::GeoJsonGeometry;

/// A D3-style geometry stream.
///
/// Coordinates are passed in degrees (matching GeoJSON), the same convention
/// used by `d3-geo/src/stream.js` before `transformRadians` converts them.
pub trait Stream {
    /// Emit a point.
    ///
    /// `m` is a visibility marker used by clipping stages (0 = visible,
    /// 1 = invisible boundary point).
    fn point(&mut self, x: f64, y: f64, m: i32);

    /// Start a line (or polygon ring).
    fn line_start(&mut self);

    /// End a line (or polygon ring).
    fn line_end(&mut self);

    /// Start a polygon.
    fn polygon_start(&mut self);

    /// End a polygon.
    fn polygon_end(&mut self);

    /// Emit a sphere.
    fn sphere(&mut self);
}

/// Stream a GeoJSON geometry into the given sink.
pub fn stream_geojson(geometry: &GeoJsonGeometry, stream: &mut dyn Stream) {
    match geometry {
        GeoJsonGeometry::Point(lon, lat) => {
            stream.point(*lon, *lat, 0);
        }
        GeoJsonGeometry::MultiPoint(points) => {
            for &(lon, lat) in points {
                stream.point(lon, lat, 0);
            }
        }
        GeoJsonGeometry::LineString(coords) => {
            stream_line(coords, stream, false);
        }
        GeoJsonGeometry::MultiLineString(lines) => {
            for line in lines {
                stream_line(line, stream, false);
            }
        }
        GeoJsonGeometry::Polygon(rings) => {
            stream_polygon(rings, stream);
        }
        GeoJsonGeometry::MultiPolygon(polygons) => {
            for rings in polygons {
                stream_polygon(rings, stream);
            }
        }
    }
}

fn stream_line(coords: &[(f64, f64)], stream: &mut dyn Stream, closed: bool) {
    stream.line_start();
    let n = if closed {
        coords.len().saturating_sub(1)
    } else {
        coords.len()
    };
    for &(lon, lat) in &coords[..n] {
        stream.point(lon, lat, 0);
    }
    stream.line_end();
}

fn stream_polygon(rings: &[Vec<(f64, f64)>], stream: &mut dyn Stream) {
    stream.polygon_start();
    for ring in rings {
        stream_line(ring, stream, true);
    }
    stream.polygon_end();
}

#[cfg(test)]
mod tests {
    use super::stream_geojson;
    use super::Stream;
    use crate::geo::path::GeoJsonGeometry;

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
    fn test_stream_geojson_point() {
        let mut stream = RecordingStream::new();
        stream_geojson(&GeoJsonGeometry::Point(10.0, 20.0), &mut stream);
        assert_eq!(stream.events, vec!["point 10 20 0"]);
    }

    #[test]
    fn test_stream_geojson_multi_point() {
        let mut stream = RecordingStream::new();
        stream_geojson(
            &GeoJsonGeometry::MultiPoint(vec![(0.0, 1.0), (2.0, 3.0)]),
            &mut stream,
        );
        assert_eq!(stream.events, vec!["point 0 1 0", "point 2 3 0"]);
    }

    #[test]
    fn test_stream_geojson_line_string() {
        let mut stream = RecordingStream::new();
        stream_geojson(
            &GeoJsonGeometry::LineString(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]),
            &mut stream,
        );
        assert_eq!(
            stream.events,
            vec![
                "line_start",
                "point 0 0 0",
                "point 1 1 0",
                "point 2 2 0",
                "line_end"
            ]
        );
    }

    #[test]
    fn test_stream_geojson_multi_line_string() {
        let mut stream = RecordingStream::new();
        stream_geojson(
            &GeoJsonGeometry::MultiLineString(vec![
                vec![(0.0, 0.0), (1.0, 1.0)],
                vec![(2.0, 2.0)],
            ]),
            &mut stream,
        );
        assert_eq!(
            stream.events,
            vec![
                "line_start",
                "point 0 0 0",
                "point 1 1 0",
                "line_end",
                "line_start",
                "point 2 2 0",
                "line_end"
            ]
        );
    }

    #[test]
    fn test_stream_geojson_polygon() {
        let mut stream = RecordingStream::new();
        stream_geojson(
            &GeoJsonGeometry::Polygon(vec![vec![
                (0.0, 0.0),
                (1.0, 0.0),
                (1.0, 1.0),
                (0.0, 0.0),
            ]]),
            &mut stream,
        );
        assert_eq!(
            stream.events,
            vec![
                "polygon_start",
                "line_start",
                "point 0 0 0",
                "point 1 0 0",
                "point 1 1 0",
                "line_end",
                "polygon_end"
            ]
        );
    }

    #[test]
    fn test_stream_geojson_multi_polygon() {
        let mut stream = RecordingStream::new();
        stream_geojson(
            &GeoJsonGeometry::MultiPolygon(vec![
                vec![vec![(0.0, 0.0), (1.0, 0.0), (0.0, 0.0)]],
                vec![vec![(2.0, 2.0), (3.0, 3.0), (2.0, 2.0)]],
            ]),
            &mut stream,
        );
        assert!(stream.events.iter().filter(|e| e == &"polygon_start").count() == 2);
        assert!(stream.events.iter().filter(|e| e == &"polygon_end").count() == 2);
    }

    #[test]
    fn test_stream_geojson_closed_ring_skips_last_point() {
        let mut stream = RecordingStream::new();
        stream_geojson(
            &GeoJsonGeometry::Polygon(vec![vec![
                (0.0, 0.0),
                (1.0, 0.0),
                (1.0, 1.0),
                (0.0, 0.0),
            ]]),
            &mut stream,
        );
        let point_count = stream.events.iter().filter(|e| e.starts_with("point")).count();
        assert_eq!(point_count, 3);
    }
}
