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
    EmptyCollection,
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
            Self::EmptyCollection => write!(f, "TopoJSON collection object is empty"),
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

/// Delta-decode quantized topology arcs into absolute positions.
fn decode_arcs(arcs: &[Vec<[i32; 2]>], transform: &Transform) -> Vec<Vec<(f64, f64)>> {
    arcs.iter()
        .map(|arc| {
            let mut x = 0i32;
            let mut y = 0i32;
            let scale = transform.scale;
            let translate = transform.translate;
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
        .collect()
}

fn decode_land(
    topology: &Topology,
    budget: &TopoJsonBudget,
) -> Result<GeoJsonGeometry, TopoJsonError> {
    let decoded_arcs = decode_arcs(&topology.arcs, &topology.transform);

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

/// A TopoJSON polygon geometry carrying a feature id (e.g. a county FIPS
/// code), as found in `us-atlas` county topologies.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum IdGeometry {
    Polygon {
        arcs: Vec<Vec<i32>>,
        #[serde(default)]
        id: Option<serde_json::Value>,
    },
    MultiPolygon {
        arcs: Vec<Vec<Vec<i32>>>,
        #[serde(default)]
        id: Option<serde_json::Value>,
    },
    GeometryCollection {
        geometries: Vec<IdGeometry>,
    },
}

#[derive(Debug, Deserialize)]
pub struct CountyObjects {
    pub counties: IdGeometry,
    #[serde(default)]
    pub states: Option<IdGeometry>,
}

#[derive(Debug, Deserialize)]
pub struct CountyTopology {
    pub objects: CountyObjects,
    pub arcs: Vec<Vec<[i32; 2]>>,
    pub transform: Transform,
}

/// A decoded county polygon with its feature id.
#[derive(Debug, Clone)]
pub struct CountyFeature {
    /// Feature id as a string (numeric FIPS ids are zero-padded to 5 digits).
    pub id: String,
    pub geometry: GeoJsonGeometry,
}

fn feature_id(value: &Option<serde_json::Value>) -> String {
    match value {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                format!("{u:05}")
            } else {
                n.to_string()
            }
        }
        _ => String::new(),
    }
}

fn collect_id_geometries(
    geom: &IdGeometry,
    out: &mut Vec<(String, Vec<Vec<(f64, f64)>>)>,
    decoded_arcs: &[Vec<(f64, f64)>],
    budget: &TopoJsonBudget,
    output_points: &mut usize,
) -> Result<(), TopoJsonError> {
    match geom {
        IdGeometry::Polygon { arcs, id } => {
            out.push((
                feature_id(id),
                decode_polygon(arcs, decoded_arcs, budget, output_points)?,
            ));
        }
        IdGeometry::MultiPolygon { arcs, id } => {
            let id = feature_id(id);
            for polygon_arcs in arcs {
                out.push((
                    id.clone(),
                    decode_polygon(polygon_arcs, decoded_arcs, budget, output_points)?,
                ));
            }
        }
        IdGeometry::GeometryCollection { geometries } => {
            for g in geometries {
                collect_id_geometries(g, out, decoded_arcs, budget, output_points)?;
            }
        }
    }
    Ok(())
}

fn count_id_geometries(geom: &IdGeometry) -> usize {
    match geom {
        IdGeometry::Polygon { .. } => 1,
        IdGeometry::MultiPolygon { arcs, .. } => arcs.len(),
        IdGeometry::GeometryCollection { geometries } => {
            geometries.iter().map(count_id_geometries).sum()
        }
    }
}

/// Parse a `us-atlas`-style counties topology into one feature per polygon.
///
/// Geometries keep their `id` (FIPS code); multi-polygon counties yield one
/// feature per polygon part sharing the id.
pub fn parse_counties(json: &str) -> Result<Vec<CountyFeature>, TopoJsonError> {
    parse_counties_with_budget(json, &TopoJsonBudget::default())
}

/// Parse counties under explicit input and output limits.
pub fn parse_counties_with_budget(
    json: &str,
    budget: &TopoJsonBudget,
) -> Result<Vec<CountyFeature>, TopoJsonError> {
    if json.len() > budget.max_input_bytes {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "input bytes",
            limit: budget.max_input_bytes,
            actual: json.len(),
        });
    }

    let topology: CountyTopology = serde_json::from_str(json)
        .map_err(|error| TopoJsonError::InvalidJson(error.to_string()))?;
    if topology.arcs.len() > budget.max_arcs {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "arcs",
            limit: budget.max_arcs,
            actual: topology.arcs.len(),
        });
    }
    if count_id_geometries(&topology.objects.counties) > budget.max_geometries {
        return Err(TopoJsonError::BudgetExceeded {
            resource: "geometries",
            limit: budget.max_geometries,
            actual: count_id_geometries(&topology.objects.counties),
        });
    }

    let decoded_arcs = decode_arcs(&topology.arcs, &topology.transform);
    let mut raw: Vec<(String, Vec<Vec<(f64, f64)>>)> = Vec::new();
    let mut output_points = 0usize;
    collect_id_geometries(
        &topology.objects.counties,
        &mut raw,
        &decoded_arcs,
        budget,
        &mut output_points,
    )?;

    if raw.is_empty() {
        return Err(TopoJsonError::EmptyCollection);
    }
    Ok(raw
        .into_iter()
        .map(|(id, rings)| CountyFeature {
            id,
            geometry: GeoJsonGeometry::Polygon(rings),
        })
        .collect())
}

/// Parse the `states` object of a counties topology into a single
/// multipolygon for state border overlays. Returns `None` when the topology
/// carries no states object.
pub fn parse_county_states(json: &str) -> Result<Option<GeoJsonGeometry>, TopoJsonError> {
    let topology: CountyTopology = serde_json::from_str(json)
        .map_err(|error| TopoJsonError::InvalidJson(error.to_string()))?;
    let states = match &topology.objects.states {
        Some(states) => states,
        None => return Ok(None),
    };
    let decoded_arcs = decode_arcs(&topology.arcs, &topology.transform);
    let mut raw: Vec<(String, Vec<Vec<(f64, f64)>>)> = Vec::new();
    let mut output_points = 0usize;
    let budget = TopoJsonBudget::default();
    collect_id_geometries(states, &mut raw, &decoded_arcs, &budget, &mut output_points)?;
    Ok(Some(GeoJsonGeometry::MultiPolygon(
        raw.into_iter().map(|(_, rings)| rings).collect(),
    )))
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
    fn test_parse_counties_albers() {
        let json = include_str!("../../bin/showcase/data/counties-albers-10m.json");
        let counties = parse_counties(json).expect("failed to parse counties-albers-10m.json");
        // ~3.1k county geometries, each with a 5-digit FIPS id.
        assert!(counties.len() > 3000, "got {} counties", counties.len());
        assert!(
            counties.iter().all(|c| c.id.len() == 5),
            "all county ids are zero-padded FIPS codes"
        );
        // Spot-check a known FIPS code keeps its geometry.
        assert!(counties.iter().any(|c| c.id == "04015"));
        let states =
            parse_county_states(json).expect("failed to parse states object");
        assert!(states.is_some(), "states object should decode");
    }

    #[test]
    fn test_parse_counties_ids_and_reversed_arcs() {
        // Minimal topology: one polygon with a numeric id and a reversed arc.
        let json = r#"{
            "type": "Topology",
            "transform": {"scale": [1.0, 1.0], "translate": [0.0, 0.0]},
            "objects": {
                "counties": {
                    "type": "GeometryCollection",
                    "geometries": [
                        {"type": "Polygon", "arcs": [[0]], "id": 1001},
                        {"type": "Polygon", "arcs": [[-1]], "id": "01003"}
                    ]
                }
            },
            "arcs": [[[0, 0], [1, 0], [1, 1], [-1, 0], [-1, -1]]]
        }"#;
        let counties = parse_counties(json).expect("fixture should parse");
        assert_eq!(counties.len(), 2);
        assert_eq!(counties[0].id, "01001");
        assert_eq!(counties[1].id, "01003");
        for county in &counties {
            match &county.geometry {
                GeoJsonGeometry::Polygon(rings) => {
                    assert_eq!(rings.len(), 1);
                    // Closed square, forward or reversed.
                    assert_eq!(rings[0].len(), 5);
                }
                _ => panic!("expected Polygon"),
            }
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
