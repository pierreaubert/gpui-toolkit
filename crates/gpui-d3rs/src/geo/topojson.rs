use serde::Deserialize;

use super::GeoJsonGeometry;

#[derive(Debug, Deserialize)]
pub struct Transform {
    pub scale: [f64; 2],
    pub translate: [f64; 2],
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TopologyGeometry {
    MultiPolygon { arcs: Vec<Vec<Vec<i32>>> },
    Polygon { arcs: Vec<Vec<i32>> },
    GeometryCollection { geometries: Vec<TopologyGeometry> },
}

#[derive(Debug, Deserialize)]
pub struct TopologyObjects {
    pub land: TopologyGeometry,
}

#[derive(Debug, Deserialize)]
pub struct Topology {
    pub objects: TopologyObjects,
    pub arcs: Vec<Vec<[i32; 2]>>,
    pub transform: Transform,
}

/// Parse a TopoJSON `land` object into a single `GeoJsonGeometry::MultiPolygon`.
///
/// All polygon and multipolygon geometries in the land collection are flattened
/// into one multipolygon so that `GeoPath` can render the whole world in a
/// single pass.
pub fn parse_land(json: &str) -> Option<GeoJsonGeometry> {
    let topology: Topology = serde_json::from_str(json).ok()?;
    decode_land(&topology)
}

fn decode_land(topology: &Topology) -> Option<GeoJsonGeometry> {
    let decoded_arcs: Vec<Vec<(f64, f64)>> = topology
        .arcs
        .iter()
        .map(|arc| {
            let mut x = 0i32;
            let mut y = 0i32;
            let scale = topology.transform.scale;
            let translate = topology.transform.translate;
            arc.iter()
                .map(|point| {
                    x += point[0];
                    y += point[1];
                    (
                        x as f64 * scale[0] + translate[0],
                        y as f64 * scale[1] + translate[1],
                    )
                })
                .collect()
        })
        .collect();

    let mut multi_polygon: Vec<Vec<Vec<(f64, f64)>>> = Vec::new();

    collect_geometries(&topology.objects.land, &mut multi_polygon, &decoded_arcs);

    if multi_polygon.is_empty() {
        None
    } else {
        Some(GeoJsonGeometry::MultiPolygon(multi_polygon))
    }
}

fn collect_geometries(
    geom: &TopologyGeometry,
    out: &mut Vec<Vec<Vec<(f64, f64)>>>,
    decoded_arcs: &[Vec<(f64, f64)>],
) {
    match geom {
        TopologyGeometry::MultiPolygon { arcs } => {
            for polygon_arcs in arcs {
                out.push(decode_polygon(polygon_arcs, decoded_arcs));
            }
        }
        TopologyGeometry::Polygon { arcs } => {
            out.push(decode_polygon(arcs, decoded_arcs));
        }
        TopologyGeometry::GeometryCollection { geometries } => {
            for g in geometries {
                collect_geometries(g, out, decoded_arcs);
            }
        }
    }
}

fn decode_polygon(
    polygon_arcs: &[Vec<i32>],
    decoded_arcs: &[Vec<(f64, f64)>],
) -> Vec<Vec<(f64, f64)>> {
    let mut polygon = Vec::with_capacity(polygon_arcs.len());
    for ring_arcs in polygon_arcs {
        let mut stitched_ring = Vec::new();
        for (i, &arc_idx) in ring_arcs.iter().enumerate() {
            let arc_opt = if arc_idx >= 0 {
                decoded_arcs.get(arc_idx as usize)
            } else {
                decoded_arcs.get((!arc_idx) as usize)
            };

            if let Some(arc) = arc_opt {
                if arc_idx < 0 {
                    for (j, p) in arc.iter().rev().enumerate() {
                        if i > 0 && j == 0 {
                            continue;
                        }
                        stitched_ring.push(*p);
                    }
                } else {
                    for (j, p) in arc.iter().enumerate() {
                        if i > 0 && j == 0 {
                            continue;
                        }
                        stitched_ring.push(*p);
                    }
                }
            } else {
                eprintln!("Warning: invalid arc index {arc_idx} in topology");
            }
        }
        polygon.push(stitched_ring);
    }
    polygon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_land_50m() {
        let json = include_str!("../../bin/showcase/data/land-50m.json");
        let geom = parse_land(json).expect("failed to parse land-50m.json");
        match geom {
            GeoJsonGeometry::MultiPolygon(mp) => {
                assert!(!mp.is_empty(), "land should contain at least one polygon");
            }
            _ => panic!("expected MultiPolygon"),
        }
    }
}
