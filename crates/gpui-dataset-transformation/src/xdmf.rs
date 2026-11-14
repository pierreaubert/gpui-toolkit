//! XDMF I/O with binary sidecar data (XDMF `Format="Binary"`).
//!
//! An XDMF document separates the XML description from bulk storage. This
//! module writes the standard `Binary` variant: geometry and attributes
//! live in little-endian binary sidecar files referenced by name, which
//! keeps multi-gigabyte fields out of the XML and streams cleanly on both
//! native and wasm hosts (the host supplies byte storage through a
//! resolver closure, so no filesystem is assumed). HDF5-backed XDMF is a
//! different storage backend and is not silently approximated.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};
use std::fmt::Write as _;

/// A written XDMF document plus its binary sidecar payloads.
#[derive(Debug, Clone, PartialEq)]
pub struct XdmfBundle {
    /// The `.xdmf` XML document text.
    pub xml: String,
    /// `(file name, bytes)` sidecars to store beside the document.
    pub binaries: Vec<(String, Vec<u8>)>,
}

/// XDMF topology codes for mixed topology streams.
fn xdmf_code(kind: CellType) -> u32 {
    match kind {
        CellType::Vertex => 1,
        CellType::Line => 2,
        CellType::Polygon => 3,
        CellType::Triangle => 4,
        CellType::Quad => 5,
        CellType::Tetra => 6,
        CellType::Pyramid => 7,
        CellType::Wedge => 8,
        CellType::Hexahedron => 9,
    }
}

fn from_xdmf_code(code: u32) -> Option<CellType> {
    Some(match code {
        1 => CellType::Vertex,
        2 => CellType::Line,
        3 => CellType::Polygon,
        4 => CellType::Triangle,
        5 => CellType::Quad,
        6 => CellType::Tetra,
        7 => CellType::Pyramid,
        8 => CellType::Wedge,
        9 => CellType::Hexahedron,
        _ => return None,
    })
}

fn f64_bytes(values: &[f64]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 8);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn u32_bytes(values: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for v in values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn data_item(out: &mut String, name: &str, dims: &str, number_type: &str, file: &str) {
    let _ = write!(
        out,
        "<DataItem Name=\"{name}\" Dimensions=\"{dims}\" NumberType=\"{number_type}\" Precision=\"8\" Endian=\"Little\" Format=\"Binary\">{file}</DataItem>"
    );
}

/// Writes a dataset as an XDMF document plus binary sidecars.
///
/// Sidecars are named `<dataset>_<part>.bin`; store them beside the XML
/// under exactly those names for [`read_xdmf`] to find them.
#[must_use]
pub fn write_xdmf(dataset: &DataSet) -> XdmfBundle {
    let base = dataset.id.replace(['/', '\\', ' '], "_");
    let mut binaries: Vec<(String, Vec<u8>)> = Vec::new();
    let points_file = format!("{base}_points.bin");
    let points: Vec<f64> = dataset.positions.iter().flatten().copied().collect();
    binaries.push((points_file.clone(), f64_bytes(&points)));

    // Mixed topology stream: [type, nodes...] per cell.
    let mut stream: Vec<u32> = Vec::new();
    for cell in 0..dataset.cell_count() {
        stream.push(xdmf_code(dataset.cells.types[cell]));
        if dataset.cells.types[cell] == CellType::Polygon {
            // Mixed streams prefix polygons with their node count.
            stream.push(dataset.cells.cell(cell).unwrap_or(&[]).len() as u32);
        }
        if let Some(nodes) = dataset.cells.cell(cell) {
            stream.extend_from_slice(nodes);
        }
    }
    let topology_file = format!("{base}_topology.bin");
    let stream_len = stream.len();
    binaries.push((topology_file.clone(), u32_bytes(&stream)));

    let mut xml = String::new();
    let _ = write!(
        xml,
        "<Xdmf Version=\"3.0\"><Domain><Grid Name=\"{}\" GridType=\"Uniform\">\
         <Geometry GeometryType=\"XYZ\">",
        dataset.id
    );
    data_item(
        &mut xml,
        "points",
        &format!("{} 3", dataset.point_count()),
        "Float",
        &points_file,
    );
    let _ = write!(
        xml,
        "</Geometry><Topology TopologyType=\"Mixed\" NumberOfElements=\"{}\">",
        dataset.cell_count()
    );
    data_item(
        &mut xml,
        "topology",
        &stream_len.to_string(),
        "UInt",
        &topology_file,
    );
    xml.push_str("</Topology>");

    for (association, fields) in [
        (Association::Point, &dataset.point_data),
        (Association::Cell, &dataset.cell_data),
    ] {
        let center = match association {
            Association::Point => "Node",
            Association::Cell => "Cell",
        };
        for field in fields {
            // Masked entries are exported as NaN plus a `<name>__valid`
            // companion attribute (see validity.rs).
            let attribute_type = match &field.data {
                FieldData::Scalar(_) | FieldData::Complex(_) => "Scalar",
                FieldData::Vector(_) => "Vector",
                FieldData::Tensor(_) => "Tensor6",
            };
            let components = field.data.components();
            let flat = crate::validity::canonical_flat(field);
            let file = format!("{base}_{}.bin", field.name.replace(['/', '\\', ' '], "_"));
            let count = flat.len() / components.max(1);
            binaries.push((file.clone(), f64_bytes(&flat)));
            let _ = write!(
                xml,
                "<Attribute Name=\"{}\" AttributeType=\"{attribute_type}\" Center=\"{center}\">",
                field.name
            );
            data_item(
                &mut xml,
                &field.name,
                &format!("{count} {components}"),
                "Float",
                &file,
            );
            xml.push_str("</Attribute>");
            if let Some(mask) = crate::validity::active_mask(field) {
                let companion_name = crate::validity::mask_name(&field.name);
                let companion: Vec<f64> = mask
                    .iter()
                    .map(|valid| if *valid { 1.0 } else { 0.0 })
                    .collect();
                let file = format!(
                    "{base}_{}.bin",
                    companion_name.replace(['/', '\\', ' '], "_")
                );
                binaries.push((file.clone(), f64_bytes(&companion)));
                let _ = write!(
                    xml,
                    "<Attribute Name=\"{companion_name}\" AttributeType=\"Scalar\" Center=\"{center}\">"
                );
                data_item(
                    &mut xml,
                    &companion_name,
                    &format!("{} 1", companion.len()),
                    "Float",
                    &file,
                );
                xml.push_str("</Attribute>");
            }
        }
    }
    xml.push_str("</Grid></Domain></Xdmf>");
    XdmfBundle { xml, binaries }
}

/// Reads an XDMF document, resolving sidecar names through `resolve`.
///
/// # Errors
///
/// Returns a description of the first structural problem encountered.
pub fn read_xdmf(xml: &str, resolve: impl Fn(&str) -> Option<Vec<u8>>) -> Result<DataSet, String> {
    if !xml.contains("<Xdmf") {
        return Err("not an XDMF document".into());
    }
    let items = parse_data_items(xml);
    let item = |name: &str| {
        items
            .iter()
            .find(|(n, _, _)| n == name)
            .ok_or_else(|| format!("XDMF document lacks DataItem '{name}'"))
    };
    let load_f64 = |name: &str| -> Result<Vec<f64>, String> {
        let (_, _, file) = item(name)?;
        let bytes = resolve(file).ok_or_else(|| format!("cannot resolve sidecar '{file}'"))?;
        Ok(bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| f64::from_le_bytes(*c))
            .collect())
    };
    let load_u32 = |name: &str| -> Result<Vec<u32>, String> {
        let (_, _, file) = item(name)?;
        let bytes = resolve(file).ok_or_else(|| format!("cannot resolve sidecar '{file}'"))?;
        Ok(bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect())
    };

    let flat = load_f64("points")?;
    if flat.len() % 3 != 0 {
        return Err("points sidecar length is not a multiple of 3".into());
    }
    let positions: Vec<[f64; 3]> = flat
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    let stream = load_u32("topology")?;
    let mut raw_cells: Vec<(CellType, Vec<u32>)> = Vec::new();
    let mut cursor = 0usize;
    while cursor < stream.len() {
        let kind = from_xdmf_code(stream[cursor])
            .ok_or_else(|| format!("unsupported XDMF topology code {}", stream[cursor]))?;
        cursor += 1;
        let n = if kind == CellType::Polygon {
            if cursor >= stream.len() {
                return Err("topology stream is truncated".into());
            }
            let count = stream[cursor] as usize;
            cursor += 1;
            count
        } else {
            kind.node_count()
        };
        if cursor + n > stream.len() {
            return Err("topology stream is truncated".into());
        }
        raw_cells.push((kind, stream[cursor..cursor + n].to_vec()));
        cursor += n;
    }
    let refs: Vec<(CellType, &[u32])> = raw_cells.iter().map(|(t, n)| (*t, n.as_slice())).collect();
    let mut dataset = DataSet::new("xdmf", positions, Cells::from_cells(&refs));

    // Attributes appear as <Attribute Name=.. Center=..><DataItem Name=../>.
    for (name, center, attribute_type) in parse_attributes(xml) {
        let values = load_f64(&name)?;
        let association = if center == "Cell" {
            Association::Cell
        } else {
            Association::Point
        };
        let mut field = match (attribute_type.as_str(), values.len()) {
            ("Vector", n) if n % 3 == 0 => Field::vector(
                name.clone(),
                values
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|c| [c[0], c[1], c[2]])
                    .collect::<Vec<_>>(),
            ),
            ("Tensor6", n) if n % 6 == 0 => {
                let mut f = Field::scalar(name.clone(), Vec::<f64>::new());
                f.data = FieldData::Tensor(
                    values
                        .as_chunks::<6>()
                        .0
                        .iter()
                        .map(|c| [c[0], c[1], c[2], c[3], c[4], c[5]])
                        .collect::<Vec<_>>()
                        .into(),
                );
                f
            }
            _ => Field::scalar(name.clone(), values),
        };
        field.association = association;
        match association {
            Association::Point => dataset.point_data.push(field),
            Association::Cell => dataset.cell_data.push(field),
        }
    }
    crate::validity::restore_masks(&mut dataset.point_data);
    crate::validity::restore_masks(&mut dataset.cell_data);
    dataset
        .validate()
        .map_err(|e| format!("parsed dataset is invalid: {e}"))?;
    Ok(dataset)
}

/// `(name, dimensions, file)` per `DataItem` element.
fn parse_data_items(xml: &str) -> Vec<(String, String, String)> {
    let mut items = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<DataItem") {
        let after = &rest[start..];
        let Some(tag_end) = after.find('>') else {
            break;
        };
        let tag = &after[..tag_end];
        let attr = |key: &str| -> String {
            let pattern = format!("{key}=\"");
            tag.find(&pattern).map_or_default(|i| {
                let from = i + pattern.len();
                let to = tag[from..].find('"').map_or(tag.len(), |e| from + e);
                tag[from..to].to_string()
            })
        };
        let close = after[tag_end..]
            .find("</DataItem>")
            .map_or(after.len(), |c| tag_end + c);
        let file = after[tag_end + 1..close].trim().to_string();
        items.push((attr("Name"), attr("Dimensions"), file));
        rest = &after[(close + 11).min(after.len())..];
    }
    items
}

/// `(name, center, attribute_type)` per `Attribute` element.
fn parse_attributes(xml: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<Attribute") {
        let after = &rest[start..];
        let Some(tag_end) = after.find('>') else {
            break;
        };
        let tag = &after[..tag_end];
        let attr = |key: &str| -> String {
            let pattern = format!("{key}=\"");
            tag.find(&pattern).map_or_default(|i| {
                let from = i + pattern.len();
                let to = tag[from..].find('"').map_or(tag.len(), |e| from + e);
                tag[from..to].to_string()
            })
        };
        out.push((attr("Name"), attr("Center"), attr("AttributeType")));
        rest = &after[tag_end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_xdmf_with_binary_sidecars() {
        let dataset = DataSet::structured_grid("room", [2, 2, 2], [0.0; 3], [1.0; 3])
            .unwrap()
            .with_field(Field::scalar("pressure", vec![2.5; 8]))
            .with_field(Field::vector("velocity", vec![[0.0, 1.0, 0.0]; 8]));
        let bundle = write_xdmf(&dataset);
        assert!(bundle.xml.contains("Format=\"Binary\""));
        assert_eq!(bundle.binaries.len(), 4);
        let parsed = read_xdmf(&bundle.xml, |name| {
            bundle
                .binaries
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, bytes)| bytes.clone())
        })
        .unwrap();
        assert_eq!(parsed.point_count(), 8);
        assert_eq!(parsed.cell_count(), 1);
        assert_eq!(parsed.cells.types[0], CellType::Hexahedron);
        assert_eq!(
            parsed
                .field(Association::Point, "pressure")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[2.5; 8]
        );
        assert_eq!(parsed.validate(), Ok(()));
    }

    #[test]
    fn missing_sidecar_is_an_error() {
        let dataset = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let bundle = write_xdmf(&dataset);
        let error = read_xdmf(&bundle.xml, |_| None).unwrap_err();
        assert!(error.contains("cannot resolve"), "got: {error}");
    }
}
