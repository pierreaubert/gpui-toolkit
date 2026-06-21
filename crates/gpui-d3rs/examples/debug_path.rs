use d3rs::geo::{GeoJsonGeometry, GeoPath, Mercator};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
struct Transform {
    scale: [f64; 2],
    translate: [f64; 2],
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum GeometryObject {
    MultiPolygon { arcs: Vec<Vec<Vec<i32>>> },
    Polygon { arcs: Vec<Vec<i32>> },
    GeometryCollection { geometries: Vec<GeometryObject> },
}

#[derive(Deserialize, Debug)]
struct Objects {
    land: GeometryObject,
}

#[derive(Deserialize, Debug)]
struct Topology {
    objects: Objects,
    arcs: Vec<Vec<[i32; 2]>>,
    transform: Transform,
}

fn parse_topojson(json_str: &str) -> Option<GeoJsonGeometry> {
    let topology: Topology = serde_json::from_str(json_str).ok()?;
    let scale = topology.transform.scale;
    let translate = topology.transform.translate;

    let decoded_arcs: Vec<Vec<(f64, f64)>> = topology
        .arcs
        .iter()
        .map(|arc| {
            let mut x = 0;
            let mut y = 0;
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

    let geometry = match &topology.objects.land {
        GeometryObject::GeometryCollection { geometries } => geometries.first()?,
        other => other,
    };

    match geometry {
        GeometryObject::MultiPolygon { arcs } => {
            let mut multi_polygon = Vec::new();
            for polygon_arcs in arcs {
                let mut polygon = Vec::new();
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
                        }
                    }
                    if !stitched_ring.is_empty() {
                        polygon.push(stitched_ring);
                    }
                }
                if !polygon.is_empty() {
                    multi_polygon.push(polygon);
                }
            }
            Some(GeoJsonGeometry::MultiPolygon(multi_polygon))
        }
        GeometryObject::Polygon { arcs } => {
            let mut polygon = Vec::new();
            for ring_arcs in arcs {
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
                    }
                }
                polygon.push(stitched_ring);
            }
            Some(GeoJsonGeometry::Polygon(polygon))
        }
        _ => None,
    }
}

fn main() {
    let json =
        fs::read_to_string("crates/gpui-d3rs/bin/showcase/data/land-50m.json").expect("land data");
    let geom = parse_topojson(&json).expect("parse");

    let config = [
        ("0,0", (0.0, 0.0)),
        ("30,-15", (30.0, -15.0)),
        ("270,-15", (270.0, -15.0)),
    ];
    for (label, (lon, lat)) in config {
        let proj = Mercator::new()
            .scale(100.0)
            .translate(400.0, 300.0)
            .rotate(lon, lat, 0.0);
        let path = GeoPath::new(proj).render(&geom);
        let out = format!("debug_path_{}.svg", label);
        let svg = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"800\" height=\"600\"><path d=\"{}\" fill=\"#ccddff\" stroke=\"#888\" stroke-width=\"0.5\"/></svg>",
            path
        );
        fs::write(&out, &svg).unwrap();
        println!("{} -> {} ({} bytes)", label, out, svg.len());

        // Find longest straight segment in path (diagonal artifact indicator)
        let mut max_diag = 0.0f64;
        let mut max_from = (0.0, 0.0);
        let mut max_to = (0.0, 0.0);
        let mut prev: Option<(f64, f64)> = None;
        let mut cur_cmd = 'M';
        // naive parse: M and L commands alternate with coordinates
        // We'll do a simpler scan for coordinate pairs
        let chars: Vec<char> = path.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            match chars[i] {
                'M' | 'L' => {
                    cur_cmd = chars[i];
                    i += 1;
                }
                'Z' => {
                    prev = None;
                    i += 1;
                }
                _ => {
                    // read number
                    let mut j = i;
                    while j < chars.len()
                        && (chars[j] == '-' || chars[j] == '.' || chars[j].is_ascii_digit())
                    {
                        j += 1;
                    }
                    if j > i {
                        let x: f64 = chars[i..j]
                            .iter()
                            .collect::<String>()
                            .parse()
                            .unwrap_or(0.0);
                        i = j;
                        if i < chars.len() && chars[i] == ',' {
                            i += 1;
                        }
                        let mut k = i;
                        while k < chars.len()
                            && (chars[k] == '-' || chars[k] == '.' || chars[k].is_ascii_digit())
                        {
                            k += 1;
                        }
                        if k > i {
                            let y: f64 = chars[i..k]
                                .iter()
                                .collect::<String>()
                                .parse()
                                .unwrap_or(0.0);
                            i = k;
                            if let Some((px, py)) = prev
                                && cur_cmd == 'L' {
                                    let dx = x - px;
                                    let dy = y - py;
                                    let d = (dx * dx + dy * dy).sqrt();
                                    if d > max_diag {
                                        max_diag = d;
                                        max_from = (px, py);
                                        max_to = (x, y);
                                    }
                                }
                            prev = Some((x, y));
                        } else {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
        println!(
            "  max straight segment: {:.1} from {:?} to {:?}",
            max_diag, max_from, max_to
        );
    }
}

fn _unused() {}
