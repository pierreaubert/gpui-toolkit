/// A GeoJSON geometry type.
#[derive(Clone, Debug)]
pub enum GeoJsonGeometry {
    /// A single point
    Point(f64, f64),
    /// Multiple points
    MultiPoint(Vec<(f64, f64)>),
    /// A line string (series of connected points)
    LineString(Vec<(f64, f64)>),
    /// Multiple line strings
    MultiLineString(Vec<Vec<(f64, f64)>>),
    /// A polygon (closed ring with optional holes)
    Polygon(Vec<Vec<(f64, f64)>>),
    /// Multiple polygons
    MultiPolygon(Vec<Vec<Vec<(f64, f64)>>>),
}
