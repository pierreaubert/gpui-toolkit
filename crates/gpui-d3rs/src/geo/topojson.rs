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

/// Limits for parsing a TopoJSON land object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopoJsonBudget {
    pub max_input_bytes: usize,
    pub max_arcs: usize,
    pub max_arc_points: usize,
    pub max_output_points: usize,
    pub max_geometries: usize,
}

impl TopoJsonBudget {
    pub const fn new(
        max_input_bytes: usize,
        max_arcs: usize,
        max_arc_points: usize,
        max_output_points: usize,
        max_geometries: usize,
    ) -> Self {
        Self {
            max_input_bytes,
            max_arcs,
            max_arc_points,
            max_output_points,
            max_geometries,
        }
    }
}

impl Default for TopoJsonBudget {
    fn default() -> Self {
        Self::new(
            32 * 1024 * 1024,
            1_000_000,
            1_000_000,
            10_000_000,
            1_000_000,
        )
    }
}

/// Failure while parsing or bounding a TopoJSON input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopoJsonError {
    InvalidJson(String),
    BudgetExceeded {
        resource: &'static str,
        limit: usize,
        actual: usize,
    },
    EmptyLand,
}

impl std::fmt::Display for TopoJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson(error) => write!(f, "invalid TopoJSON: {error}"),
            Self::BudgetExceeded {
                resource,
                limit,
                actual,
            } => write!(f, "TopoJSON {resource} budget exceeded: {actual} > {limit}"),
            Self::EmptyLand => write!(f, "TopoJSON land object is empty"),
        }
    }
}

impl std::error::Error for TopoJsonError {}

/// Parse a TopoJSON `land` object into a single `GeoJsonGeometry::MultiPolygon`.
///
/// All polygon and multipolygon geometries in the land collection are flattened
/// into one multipolygon so that `GeoPath` can render the whole world in a
/// single pass.
pub fn parse_land(json: &str) -> Option<GeoJsonGeometry> {
    parse_land_with_budget(json, &TopoJsonBudget::default()).ok()
}

/// Parse a TopoJSON `land` object under explicit input and output limits.
pub fn parse_land_with_budget(
    json: &str,
    budget: &TopoJsonBudget,
) -> Result<GeoJsonGeometry, TopoJsonError> {
    if json.len() > budget.max_input_bytes {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "input bytes",
            limit: budget.max_input_bytes,
            actual: json.len(),
        });
    }

    let topology: Topology = serde_json::from_str(json)
        .map_err(|error| TopoJsonError::InvalidJson(error.to_string()))?;
    validate_budget(&topology, budget)?;
    decode_land(&topology, budget)
}

fn validate_budget(topology: &Topology, budget: &TopoJsonBudget) -> Result<(), TopoJsonError> {
    if topology.arcs.len() > budget.max_arcs {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "arcs",
            limit: budget.max_arcs,
            actual: topology.arcs.len(),
        });
    }
    let mut decoded_points = 0usize;
    for arc in &topology.arcs {
        if arc.len() > budget.max_arc_points {
            return Err(TopoJsonError::BudgetExceeded {
                resource: "arc points",
                limit: budget.max_arc_points,
                actual: arc.len(),
            });
        }
        decoded_points = decoded_points.saturating_add(arc.len());
    }
    if decoded_points > budget.max_output_points {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "decoded arc points",
            limit: budget.max_output_points,
            actual: decoded_points,
        });
    }

    let geometries = count_geometries(&topology.objects.land);
    if geometries > budget.max_geometries {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "geometries",
            limit: budget.max_geometries,
            actual: geometries,
        });
    }
    Ok(())
}

fn count_geometries(geom: &TopologyGeometry) -> usize {
    match geom {
        TopologyGeometry::MultiPolygon { arcs } => arcs.len(),
        TopologyGeometry::Polygon { .. } => 1,
        TopologyGeometry::GeometryCollection { geometries } => {
            geometries.iter().map(count_geometries).sum()
        }
    }
}

fn decode_land(
    topology: &Topology,
    budget: &TopoJsonBudget,
) -> Result<GeoJsonGeometry, TopoJsonError> {
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

    let mut output_points = 0;
    collect_geometries(
        &topology.objects.land,
        &mut multi_polygon,
        &decoded_arcs,
        budget,
        &mut output_points,
    )?;

    if multi_polygon.is_empty() {
        Err(TopoJsonError::EmptyLand)
    } else {
        Ok(GeoJsonGeometry::MultiPolygon(multi_polygon))
    }
}

fn collect_geometries(
    geom: &TopologyGeometry,
    out: &mut Vec<Vec<Vec<(f64, f64)>>>,
    decoded_arcs: &[Vec<(f64, f64)>],
    budget: &TopoJsonBudget,
    output_points: &mut usize,
) -> Result<(), TopoJsonError> {
    match geom {
        TopologyGeometry::MultiPolygon { arcs } => {
            for polygon_arcs in arcs {
                out.push(decode_polygon(
                    polygon_arcs,
                    decoded_arcs,
                    budget,
                    output_points,
                )?);
            }
        }
        TopologyGeometry::Polygon { arcs } => {
            out.push(decode_polygon(arcs, decoded_arcs, budget, output_points)?);
        }
        TopologyGeometry::GeometryCollection { geometries } => {
            for g in geometries {
                collect_geometries(g, out, decoded_arcs, budget, output_points)?;
            }
        }
    }
    Ok(())
}

fn decode_polygon(
    polygon_arcs: &[Vec<i32>],
    decoded_arcs: &[Vec<(f64, f64)>],
    budget: &TopoJsonBudget,
    output_points: &mut usize,
) -> Result<Vec<Vec<(f64, f64)>>, TopoJsonError> {
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
                        *output_points = (*output_points).saturating_add(1);
                        if *output_points > budget.max_output_points {
                            return Err(TopoJsonError::BudgetExceeded {
                                resource: "output points",
                                limit: budget.max_output_points,
                                actual: *output_points,
                            });
                        }
                        stitched_ring.push(*p);
                    }
                } else {
                    for (j, p) in arc.iter().enumerate() {
                        if i > 0 && j == 0 {
                            continue;
                        }
                        *output_points = (*output_points).saturating_add(1);
                        if *output_points > budget.max_output_points {
                            return Err(TopoJsonError::BudgetExceeded {
                                resource: "output points",
                                limit: budget.max_output_points,
                                actual: *output_points,
                            });
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
    Ok(polygon)
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

    #[test]
    fn budget_rejects_large_topojson_before_deserializing() {
        let budget = TopoJsonBudget::new(1, 10, 10, 10, 10);
        let error = parse_land_with_budget("{}", &budget).unwrap_err();
        assert!(matches!(
            error,
            TopoJsonError::BudgetExceeded {
                resource: "input bytes",
                ..
            }
        ));
    }

    #[test]
    fn budget_rejects_excess_decoded_arc_points() {
        let json = r#"{
            "objects":{"land":{"type":"Polygon","arcs":[[0]]}},
            "arcs":[[[0,0],[1,1],[1,1]]],
            "transform":{"scale":[1,1],"translate":[0,0]}
        }"#;
        let budget = TopoJsonBudget::new(4096, 10, 10, 2, 10);
        let error = parse_land_with_budget(json, &budget).unwrap_err();
        assert!(matches!(
            error,
            TopoJsonError::BudgetExceeded {
                resource: "decoded arc points",
                ..
            }
        ));
    }
}
