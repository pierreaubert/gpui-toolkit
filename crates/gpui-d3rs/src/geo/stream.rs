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
