//! Surface-mesh file formats: STL, OBJ, and PLY.
//!
//! Format glue between files and [`DataSet`] triangle surfaces (Sonium's
//! mesher remains the tool for generating and repairing such meshes;
//! these readers/writers move them in and out of the toolkit). STL facet
//! normals are recomputed on read; OBJ/PLY keep indexed vertices.
//!
//! These formats are geometry-only here: writers export boundary
//! triangles and positions only, so point/cell fields and their validity
//! masks are not written (use VTK/VTU/XDMF for field-bearing export) and
//! readers produce datasets without fields.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, boundary_triangles};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Reads an STL file (binary or ASCII) as an indexed triangle surface.
///
/// Vertices are deduplicated by exact position, so the result is a
/// welded mesh rather than a triangle soup.
///
/// # Errors
///
/// Returns a description of the first structural problem encountered.
pub fn read_stl(bytes: &[u8]) -> Result<DataSet, String> {
    if bytes.len() >= 84 && !is_ascii_stl(bytes) {
        Ok(weld(read_stl_binary(bytes)?))
    } else {
        Ok(weld(parse_ascii_stl(bytes)?))
    }
}

fn is_ascii_stl(bytes: &[u8]) -> bool {
    bytes.starts_with(b"solid") && bytes.windows(5).any(|w| w == b"facet")
}

fn read_stl_binary(bytes: &[u8]) -> Result<Vec<[[f64; 3]; 3]>, String> {
    let count = u32::from_le_bytes(
        bytes[80..84]
            .try_into()
            .map_err(|_| "bad STL header".to_string())?,
    ) as usize;
    let mut out = Vec::with_capacity(count);
    let mut cursor = 84usize;
    for _ in 0..count {
        if cursor + 50 > bytes.len() {
            return Err("binary STL is truncated".into());
        }
        let mut tri = [[0.0f64; 3]; 3];
        for (vertex_index, vertex) in tri.iter_mut().enumerate() {
            for (axis_index, component) in vertex.iter_mut().enumerate() {
                let at = cursor + 12 + (vertex_index * 3 + axis_index) * 4;
                *component = f64::from(f32::from_le_bytes(
                    bytes[at..at + 4].try_into().unwrap_or([0; 4]),
                ));
            }
        }
        cursor += 50;
        out.push(tri);
    }
    Ok(out)
}

fn parse_ascii_stl(bytes: &[u8]) -> Result<Vec<[[f64; 3]; 3]>, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut triangles = Vec::new();
    let mut current: Vec<[f64; 3]> = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() == 4 && fields[0] == "vertex" {
            let vertex = [
                fields[1].parse::<f64>().map_err(|_| "bad STL vertex")?,
                fields[2].parse::<f64>().map_err(|_| "bad STL vertex")?,
                fields[3].parse::<f64>().map_err(|_| "bad STL vertex")?,
            ];
            current.push(vertex);
            if current.len() == 3 {
                triangles.push([current[0], current[1], current[2]]);
                current.clear();
            }
        }
    }
    if triangles.is_empty() {
        return Err("no STL facets found".into());
    }
    Ok(triangles)
}

/// Welds a triangle soup into an indexed surface by exact positions.
fn weld(triangles: Vec<[[f64; 3]; 3]>) -> DataSet {
    let mut index: HashMap<[u64; 3], u32> = HashMap::new();
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for tri in triangles {
        let mut nodes = Vec::with_capacity(3);
        for vertex in tri {
            let key = [
                vertex[0].to_bits(),
                vertex[1].to_bits(),
                vertex[2].to_bits(),
            ];
            let id = *index.entry(key).or_insert_with(|| {
                positions.push(vertex);
                (positions.len() - 1) as u32
            });
            nodes.push(id);
        }
        cells.push((CellType::Triangle, nodes));
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    DataSet::new("stl", positions, Cells::from_cells(&refs))
}

/// Writes a dataset's boundary surface as binary STL bytes.
///
/// # Errors
///
/// Returns an error when the dataset is invalid.
pub fn write_stl(dataset: &DataSet) -> Result<Vec<u8>, String> {
    dataset.validate().map_err(|e| e.to_string())?;
    let triangles = boundary_triangles(dataset);
    let mut out = Vec::with_capacity(84 + triangles.len() * 50);
    out.extend_from_slice(&[0u8; 80]);
    out.extend_from_slice(&(triangles.len() as u32).to_le_bytes());
    for tri in triangles {
        let p: Vec<[f64; 3]> = tri.iter().map(|&n| dataset.positions[n as usize]).collect();
        // Facet normal (unnormalised is accepted; write unit normal).
        let ab = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
        let ac = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
        let mut n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 {
            n = [n[0] / len, n[1] / len, n[2] / len];
        }
        for value in n {
            out.extend_from_slice(&(value as f32).to_le_bytes());
        }
        for vertex in p {
            for value in vertex {
                out.extend_from_slice(&(value as f32).to_le_bytes());
            }
        }
        out.extend_from_slice(&[0u8; 2]);
    }
    Ok(out)
}

/// Reads a Wavefront OBJ file (`v` and `f` records; slash forms allowed).
///
/// # Errors
///
/// Returns a description of the first structural problem encountered.
pub fn read_obj(text: &str) -> Result<DataSet, String> {
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields.first().copied() {
            Some("v") if fields.len() >= 4 => {
                positions.push([
                    fields[1].parse().map_err(|_| "bad OBJ vertex")?,
                    fields[2].parse().map_err(|_| "bad OBJ vertex")?,
                    fields[3].parse().map_err(|_| "bad OBJ vertex")?,
                ]);
            }
            Some("f") if fields.len() >= 4 => {
                let mut nodes = Vec::new();
                for token in &fields[1..] {
                    let first = token.split('/').next().unwrap_or("");
                    let raw: i64 = first.parse().map_err(|_| "bad OBJ face")?;
                    let index = if raw < 0 {
                        positions.len() as i64 + raw
                    } else {
                        raw - 1
                    };
                    if index < 0 || index as usize >= positions.len() {
                        return Err("OBJ face index out of range".into());
                    }
                    nodes.push(index as u32);
                }
                let kind = match nodes.len() {
                    3 => CellType::Triangle,
                    4 => CellType::Quad,
                    _ => CellType::Polygon,
                };
                cells.push((kind, nodes));
            }
            _ => {}
        }
    }
    if positions.is_empty() {
        return Err("OBJ file has no vertices".into());
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new("obj", positions, Cells::from_cells(&refs)))
}

/// Writes a dataset as OBJ text (boundary triangles as faces).
#[must_use]
pub fn write_obj(dataset: &DataSet) -> String {
    let mut out = String::new();
    for p in dataset.positions.iter() {
        let _ = writeln!(out, "v {} {} {}", p[0], p[1], p[2]);
    }
    for tri in boundary_triangles(dataset) {
        let _ = writeln!(out, "f {} {} {}", tri[0] + 1, tri[1] + 1, tri[2] + 1);
    }
    out
}

/// Reads an ASCII PLY file (vertex positions + triangle/quad faces).
///
/// # Errors
///
/// Returns a description of the first structural problem encountered.
pub fn read_ply(text: &str) -> Result<DataSet, String> {
    let mut vertices = 0usize;
    let mut faces = 0usize;
    let mut body = None;
    let mut lines = text.lines().peekable();
    for line in lines.by_ref() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        match fields.first().copied() {
            Some("element") if fields.len() == 3 && fields[1] == "vertex" => {
                vertices = fields[2].parse().map_err(|_| "bad PLY header")?;
            }
            Some("element") if fields.len() == 3 && fields[1] == "face" => {
                faces = fields[2].parse().map_err(|_| "bad PLY header")?;
            }
            Some("end_header") => {
                body = Some(());
                break;
            }
            _ => {}
        }
    }
    if body.is_none() {
        return Err("PLY file has no end_header".into());
    }
    let mut positions = Vec::with_capacity(vertices);
    for line in lines.by_ref().take(vertices) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            return Err("PLY vertex line too short".into());
        }
        positions.push([
            fields[0].parse().map_err(|_| "bad PLY vertex")?,
            fields[1].parse().map_err(|_| "bad PLY vertex")?,
            fields[2].parse().map_err(|_| "bad PLY vertex")?,
        ]);
    }
    let mut cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    for line in lines.take(faces) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let count: usize = fields[0].parse().map_err(|_| "bad PLY face")?;
        if fields.len() < count + 1 {
            return Err("PLY face line too short".into());
        }
        let nodes: Vec<u32> = fields[1..=count]
            .iter()
            .map(|f| f.parse::<u32>().map_err(|_| "bad PLY face index"))
            .collect::<Result<_, _>>()?;
        let kind = match count {
            3 => CellType::Triangle,
            4 => CellType::Quad,
            _ => CellType::Polygon,
        };
        cells.push((kind, nodes));
    }
    let refs: Vec<(CellType, &[u32])> = cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    Ok(DataSet::new("ply", positions, Cells::from_cells(&refs)))
}

/// Writes a dataset as ASCII PLY (boundary triangles).
#[must_use]
pub fn write_ply(dataset: &DataSet) -> String {
    let triangles = boundary_triangles(dataset);
    let mut out = String::new();
    let _ = writeln!(out, "ply\nformat ascii 1.0");
    let _ = writeln!(out, "element vertex {}", dataset.point_count());
    let _ = writeln!(out, "property float x\nproperty float y\nproperty float z");
    let _ = writeln!(out, "element face {}", triangles.len());
    let _ = writeln!(out, "property list uchar int vertex_indices\nend_header");
    for p in dataset.positions.iter() {
        let _ = writeln!(out, "{} {} {}", p[0], p[1], p[2]);
    }
    for tri in triangles {
        let _ = writeln!(out, "3 {} {} {}", tri[0], tri[1], tri[2]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tetra_surface() -> DataSet {
        DataSet::surface(
            "t",
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            &[[0, 2, 1], [0, 1, 3], [1, 2, 3], [0, 3, 2]],
        )
    }

    #[test]
    fn stl_obj_ply_round_trip() {
        let ds = tetra_surface();
        let back = read_stl(&write_stl(&ds).unwrap()).unwrap();
        assert_eq!(back.point_count(), 4);
        assert_eq!(back.cell_count(), 4);

        let back = read_obj(&write_obj(&ds)).unwrap();
        assert_eq!(back.cell_count(), 4);
        assert_eq!(back.positions.len(), 4);

        let back = read_ply(&write_ply(&ds)).unwrap();
        assert_eq!(back.cell_count(), 4);
        assert_eq!(back.validate(), Ok(()));
    }

    #[test]
    fn ascii_stl_and_slash_obj_faces_parse() {
        let stl = "solid t\n facet normal 0 0 1\n  outer loop\n   vertex 0 0 0\n   vertex 1 0 0\n   vertex 0 1 0\n  endloop\n endfacet\nendsolid t\n";
        let ds = read_stl(stl.as_bytes()).unwrap();
        assert_eq!(ds.cell_count(), 1);
        let obj = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1/1/1 2/2/2 3/3/3\n";
        let ds = read_obj(obj).unwrap();
        assert_eq!(ds.cell_count(), 1);
    }
}
