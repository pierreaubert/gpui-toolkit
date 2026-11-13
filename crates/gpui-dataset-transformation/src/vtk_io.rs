//! Legacy VTK (`.vtk`) ASCII unstructured-grid read and write.
//!
//! This is the interchange subset: points, cells with VTK type codes, and
//! scalar/vector point and cell data (tensor/complex fields are not
//! written). It covers the files most solvers still export for quick
//! inspection; binary and XML (`.vtu`) formats are separate concerns and
//! are not silently approximated here. Validity masks follow the shared
//! IO convention in `validity.rs`: masked entries are written as NaN with
//! a `<name>__valid` companion, and readers restore masks from it.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};
use std::fmt::Write as _;

/// VTK cell type codes for the supported kinds.
fn vtk_code(kind: CellType) -> u32 {
    match kind {
        CellType::Vertex => 1,
        CellType::Line => 3,
        CellType::Triangle => 5,
        CellType::Quad => 9,
        CellType::Tetra => 10,
        CellType::Hexahedron => 12,
        CellType::Wedge => 13,
        CellType::Pyramid => 14,
        CellType::Polygon => 7,
    }
}

fn from_vtk_code(code: u32) -> Option<CellType> {
    Some(match code {
        1 => CellType::Vertex,
        3 => CellType::Line,
        5 => CellType::Triangle,
        9 => CellType::Quad,
        10 => CellType::Tetra,
        12 => CellType::Hexahedron,
        13 => CellType::Wedge,
        14 => CellType::Pyramid,
        7 => CellType::Polygon,
        _ => return None,
    })
}

/// Serializes a dataset as legacy VTK ASCII `UNSTRUCTURED_GRID` text.
#[must_use]
pub fn write_vtk(dataset: &DataSet) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# vtk DataFile Version 3.0");
    let _ = writeln!(out, "{}", dataset.id);
    let _ = writeln!(out, "ASCII");
    let _ = writeln!(out, "DATASET UNSTRUCTURED_GRID");
    let _ = writeln!(out, "POINTS {} double", dataset.point_count());
    for p in dataset.positions.iter() {
        let _ = writeln!(out, "{} {} {}", p[0], p[1], p[2]);
    }
    let total_connectivity: usize = (0..dataset.cell_count())
        .map(|c| 1 + dataset.cells.cell(c).map_or(0, <[u32]>::len))
        .sum();
    let _ = writeln!(out, "CELLS {} {}", dataset.cell_count(), total_connectivity);
    for cell in 0..dataset.cell_count() {
        let nodes = dataset.cells.cell(cell).unwrap_or(&[]);
        let _ = write!(out, "{}", nodes.len());
        for node in nodes {
            let _ = write!(out, " {node}");
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(out, "CELL_TYPES {}", dataset.cell_count());
    for kind in dataset.cells.types.iter() {
        let _ = writeln!(out, "{}", vtk_code(*kind));
    }
    write_fields(
        &mut out,
        "POINT_DATA",
        dataset.point_count(),
        &dataset.point_data,
    );
    write_fields(
        &mut out,
        "CELL_DATA",
        dataset.cell_count(),
        &dataset.cell_data,
    );
    out
}

fn write_fields(out: &mut String, header: &str, count: usize, fields: &[Field]) {
    let writable: Vec<&Field> = fields
        .iter()
        .filter(|f| matches!(f.data, FieldData::Scalar(_) | FieldData::Vector(_)))
        .collect();
    if writable.is_empty() {
        return;
    }
    let _ = writeln!(out, "{header} {count}");
    for field in writable {
        // Masked entries are exported as NaN plus a `<name>__valid`
        // companion (see validity.rs), never as placeholder measurements.
        let flat = crate::validity::canonical_flat(field);
        match &field.data {
            FieldData::Scalar(_) => {
                let _ = writeln!(out, "SCALARS {} double 1", field.name);
                let _ = writeln!(out, "LOOKUP_TABLE default");
                for value in &flat {
                    let _ = writeln!(out, "{value}");
                }
            }
            FieldData::Vector(_) => {
                let _ = writeln!(out, "VECTORS {} double", field.name);
                for value in flat.as_chunks::<3>().0 {
                    let _ = writeln!(out, "{} {} {}", value[0], value[1], value[2]);
                }
            }
            _ => {}
        }
        if let Some(mask) = crate::validity::active_mask(field) {
            let _ = writeln!(
                out,
                "SCALARS {} double 1",
                crate::validity::mask_name(&field.name)
            );
            let _ = writeln!(out, "LOOKUP_TABLE default");
            for valid in mask {
                let _ = writeln!(out, "{}", if *valid { 1.0 } else { 0.0 });
            }
        }
    }
}

/// Parses legacy VTK ASCII `UNSTRUCTURED_GRID` text.
///
/// # Errors
///
/// Returns a description of the first structural problem encountered.
pub fn read_vtk(text: &str) -> Result<DataSet, String> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());
    let header = lines.next().ok_or("empty file")?;
    if !header.starts_with("# vtk DataFile") {
        return Err(format!("missing VTK header, found '{header}'"));
    }
    let title = lines.next().unwrap_or("dataset").to_string();
    let mut tokens: Vec<String> = Vec::new();
    let mut section_lines: Vec<String> = Vec::new();
    for line in lines {
        if line == "ASCII" || line.starts_with("DATASET") {
            continue;
        }
        section_lines.push(line.to_string());
    }
    for line in &section_lines {
        tokens.extend(line.split_whitespace().map(str::to_string));
    }
    let mut cursor = TokenCursor {
        tokens: &tokens,
        pos: 0,
    };
    let mut positions: Vec<[f64; 3]> = Vec::new();
    let mut cells = Cells::from_cells(&[]);
    let mut point_data: Vec<Field> = Vec::new();
    let mut cell_data: Vec<Field> = Vec::new();
    while let Some(keyword) = cursor.next() {
        match keyword.as_str() {
            "POINTS" => {
                let count: usize = cursor.parse()?;
                cursor.next(); // dtype
                for _ in 0..count {
                    positions.push([cursor.parse()?, cursor.parse()?, cursor.parse()?]);
                }
            }
            "CELLS" => {
                let count: usize = cursor.parse()?;
                let _total: usize = cursor.parse()?;
                let mut raw: Vec<Vec<u32>> = Vec::with_capacity(count);
                for _ in 0..count {
                    let n: usize = cursor.parse()?;
                    let mut nodes = Vec::with_capacity(n);
                    for _ in 0..n {
                        nodes.push(cursor.parse()?);
                    }
                    raw.push(nodes);
                }
                // Types are patched in when CELL_TYPES is parsed below.
                cells = Cells {
                    types: vec![CellType::Triangle; count].into(),
                    offsets: offsets_of(&raw),
                    connectivity: raw.iter().flatten().copied().collect::<Vec<_>>().into(),
                };
            }
            "CELL_TYPES" => {
                let count: usize = cursor.parse()?;
                let mut types = Vec::with_capacity(count);
                for _ in 0..count {
                    let code: u32 = cursor.parse()?;
                    types.push(
                        from_vtk_code(code)
                            .ok_or_else(|| format!("unsupported VTK cell type {code}"))?,
                    );
                }
                cells.types = types.into();
            }
            "POINT_DATA" | "CELL_DATA" => {
                let is_point = keyword == "POINT_DATA";
                let _count: usize = cursor.parse()?;
                let target = if is_point {
                    &mut point_data
                } else {
                    &mut cell_data
                };
                parse_attributes(&mut cursor, target, is_point)?;
            }
            other => {
                return Err(format!("unsupported VTK keyword '{other}'"));
            }
        }
    }
    crate::validity::restore_masks(&mut point_data);
    crate::validity::restore_masks(&mut cell_data);
    let mut dataset = DataSet::new(title, positions, cells);
    dataset.point_data = point_data;
    dataset.cell_data = cell_data;
    dataset
        .validate()
        .map_err(|e| format!("parsed dataset is invalid: {e}"))?;
    Ok(dataset)
}

fn offsets_of(raw: &[Vec<u32>]) -> std::sync::Arc<[u64]> {
    let mut offsets = Vec::with_capacity(raw.len() + 1);
    offsets.push(0u64);
    for cell in raw {
        offsets.push(offsets.last().copied().unwrap_or(0) + cell.len() as u64);
    }
    offsets.into()
}

fn parse_attributes(
    cursor: &mut TokenCursor<'_>,
    target: &mut Vec<Field>,
    is_point: bool,
) -> Result<(), String> {
    // Attribute blocks run until the next top-level keyword, which the
    // token stream cannot delimit; scalar/vector blocks have known
    // lengths only from the dataset, so parse greedily by declared counts
    // recorded in the field headers themselves. The caller guarantees the
    // file orders POINT_DATA/CELL_DATA last, as writers emit them.
    while let Some(keyword) = cursor.peek() {
        match keyword.as_str() {
            "SCALARS" => {
                cursor.next();
                let name = cursor.next().ok_or("SCALARS without a name")?;
                cursor.next(); // dtype
                let _components: usize = cursor.parse().unwrap_or(1);
                if cursor.peek().as_deref() == Some("LOOKUP_TABLE") {
                    cursor.next();
                    cursor.next();
                }
                // The value count is the remaining scalar run; it is
                // bounded by the next non-numeric token, handled by the
                // caller through dataset validation. Here we need the
                // count: it equals the association length, which the
                // reader validates after parsing, so consume numbers
                // until a known keyword appears.
                let mut values = Vec::new();
                while let Some(token) = cursor.peek() {
                    if is_keyword(&token) {
                        break;
                    }
                    values.push(cursor.parse::<f64>()?);
                }
                let mut field = Field::scalar(name, values);
                if !is_point {
                    field = field.on_cells();
                }
                target.push(field);
            }
            "VECTORS" => {
                cursor.next();
                let name = cursor.next().ok_or("VECTORS without a name")?;
                cursor.next(); // dtype
                let mut values = Vec::new();
                while let Some(token) = cursor.peek() {
                    if is_keyword(&token) {
                        break;
                    }
                    values.push([
                        cursor.parse::<f64>()?,
                        cursor.parse::<f64>()?,
                        cursor.parse::<f64>()?,
                    ]);
                }
                let mut field = Field::vector(name, values);
                if !is_point {
                    field.association = Association::Cell;
                }
                target.push(field);
            }
            _ => break,
        }
    }
    Ok(())
}

fn is_keyword(token: &str) -> bool {
    matches!(
        token,
        "POINTS"
            | "CELLS"
            | "CELL_TYPES"
            | "POINT_DATA"
            | "CELL_DATA"
            | "SCALARS"
            | "VECTORS"
            | "LOOKUP_TABLE"
    )
}

struct TokenCursor<'a> {
    tokens: &'a [String],
    pos: usize,
}

impl TokenCursor<'_> {
    fn next(&mut self) -> Option<String> {
        let token = self.tokens.get(self.pos)?.clone();
        self.pos += 1;
        Some(token)
    }

    fn peek(&self) -> Option<String> {
        self.tokens.get(self.pos).cloned()
    }

    fn parse<T: std::str::FromStr>(&mut self) -> Result<T, String> {
        let token = self
            .next()
            .ok_or_else(|| "unexpected end of file".to_string())?;
        token
            .parse::<T>()
            .map_err(|_| format!("expected a number, found '{token}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_an_unstructured_dataset() {
        let dataset = DataSet::structured_grid("room", [2, 2, 2], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("pressure", vec![0.5; 8]).labelled("Pressure", "Pa"))
            .with_field(Field::vector("velocity", vec![[1.0, 0.0, 0.0]; 8]));
        let text = write_vtk(&dataset);
        let parsed = read_vtk(&text).unwrap();
        assert_eq!(parsed.point_count(), 8);
        assert_eq!(parsed.cell_count(), 1);
        assert_eq!(parsed.cells.types[0], CellType::Hexahedron);
        assert_eq!(
            parsed
                .field(Association::Point, "pressure")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[0.5; 8]
        );
        assert_eq!(parsed.validate(), Ok(()));
    }

    #[test]
    fn rejects_a_non_vtk_file() {
        read_vtk("hello\nworld").unwrap_err();
    }
}
