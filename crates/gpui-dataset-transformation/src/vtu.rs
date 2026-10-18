//! VTU (XML VTK unstructured grid) binary I/O.
//!
//! Arrays use inline `format="binary"`: base64 of a little-endian `UInt64`
//! byte-count header followed by the raw array bytes. That is the layout
//! `ParaView` and VTK itself write when compression is off, so files are
//! interchangeable in both directions for the supported array types.

// Rust guideline compliant 2026-02-21

use crate::base64;
use gpui_mesh::{Association, CellType, Cells, DataSet, Field, FieldData};
use std::fmt::Write as _;

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

/// VTK zlib block size for compressed arrays.
const COMPRESS_BLOCK: usize = 32_768;

/// Limits checked before parsing arrays or expanding compressed input.
///
/// Decoded bytes include raw arrays and their expanded numeric storage. XML
/// input and bounded parser metadata are accounted for separately. Hosts with
/// tighter working sets should lower these limits before accepting a file.
#[derive(Debug, Clone, Copy)]
pub struct VtuReadLimits {
    /// Maximum complete input bytes, including appended data.
    pub max_input_bytes: usize,
    /// Maximum uncompressed bytes in one array.
    pub max_array_bytes: usize,
    /// Maximum cumulative decoded raw and numeric-array bytes.
    pub max_decoded_bytes: usize,
    /// Maximum array descriptors in one file.
    pub max_arrays: usize,
    /// Maximum compressed blocks in one array.
    pub max_blocks: usize,
}

impl Default for VtuReadLimits {
    fn default() -> Self {
        // Bounded desktop defaults; mobile hosts can supply smaller budgets.
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_array_bytes: 32 * 1024 * 1024,
            max_decoded_bytes: 128 * 1024 * 1024,
            max_arrays: 4096,
            max_blocks: 65_536,
        }
    }
}

struct VtuBudget {
    limits: VtuReadLimits,
    decoded: usize,
    arrays: usize,
}

impl VtuBudget {
    fn reserve(&mut self, bytes: usize) -> Result<(), String> {
        let total = self
            .decoded
            .checked_add(bytes)
            .ok_or("decoded VTU size overflow")?;
        if total > self.limits.max_decoded_bytes {
            return Err("decoded VTU byte budget exceeded".into());
        }
        self.decoded = total;
        Ok(())
    }

    fn array(&mut self, bytes: usize) -> Result<(), String> {
        if bytes > self.limits.max_array_bytes {
            return Err("VTU array byte budget exceeded".into());
        }
        self.reserve(bytes)
    }
}

/// Compresses `raw` in the VTK compressed-binary layout: a `UInt64` header
/// `[num_blocks, block_size, last_block_size, compressed sizes...]`
/// followed by one zlib stream per block.
fn compress_block(raw: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let blocks: Vec<&[u8]> = raw.chunks(COMPRESS_BLOCK).collect();
    let num_blocks = blocks.len().max(1);
    let mut compressed: Vec<Vec<u8>> = Vec::with_capacity(num_blocks);
    for block in &blocks {
        let mut encoder =
            flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        let _ = encoder.write_all(block);
        compressed.push(encoder.finish().unwrap_or_default());
    }
    if blocks.is_empty() {
        compressed.push(Vec::new());
    }
    let last_size = blocks.last().map_or(0, |b| b.len());
    let mut out = Vec::new();
    for value in [num_blocks as u64, COMPRESS_BLOCK as u64, last_size as u64] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    for block in &compressed {
        out.extend_from_slice(&(block.len() as u64).to_le_bytes());
    }
    for block in compressed {
        out.extend_from_slice(&block);
    }
    out
}

/// Encodes `raw` as uncompressed inline binary: base64 of a `UInt64`
/// byte-count header followed by the raw array bytes.
fn binary_block(raw: &[u8]) -> String {
    let mut payload = Vec::with_capacity(8 + raw.len());
    payload.extend_from_slice(&(raw.len() as u64).to_le_bytes());
    payload.extend_from_slice(raw);
    base64::encode(&payload)
}

/// Reverses [`compress_block`]: parses the `UInt64` block header and
/// inflates each zlib block in order.
///
/// # Errors
///
/// Returns a description when the header is truncated, a block size is
/// inconsistent, or a zlib stream fails to inflate.
fn decompress_block(payload: &[u8], budget: &mut VtuBudget) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    if payload.len() < 24 {
        return Err("compressed DataArray shorter than its header".into());
    }
    let header = |index: usize| -> u64 {
        let at = index * 8;
        u64::from_le_bytes(payload[at..at + 8].try_into().unwrap_or([0; 8]))
    };
    let count = |index| {
        usize::try_from(header(index))
            .map_err(|_| "compressed VTU count exceeds address space".to_string())
    };
    let num_blocks = count(0)?;
    let block_size = count(1)?;
    let last_size = count(2)?;
    if num_blocks > budget.limits.max_blocks {
        return Err("compressed VTU block budget exceeded".into());
    }
    let header_end = num_blocks
        .checked_mul(8)
        .and_then(|bytes| bytes.checked_add(24))
        .ok_or("compressed VTU header size overflow")?;
    if payload.len() < header_end {
        return Err("compressed DataArray header is truncated".into());
    }
    if (num_blocks > 0 && (block_size == 0 || last_size > block_size))
        || (num_blocks == 0 && last_size != 0)
    {
        return Err("compressed VTU block dimensions are inconsistent".into());
    }
    let expected = num_blocks
        .saturating_sub(1)
        .checked_mul(block_size)
        .and_then(|bytes| bytes.checked_add(last_size))
        .ok_or("decompressed VTU size overflow")?;
    // Validate every compressed span before trusting the declared output size.
    let mut cursor = header_end;
    for block in 0..num_blocks {
        cursor = cursor
            .checked_add(count(3 + block)?)
            .ok_or("compressed VTU span overflow")?;
        if cursor > payload.len() {
            return Err("compressed DataArray block is truncated".into());
        }
    }
    budget.array(expected)?;
    let mut out = Vec::new();
    out.try_reserve_exact(expected)
        .map_err(|_| "cannot allocate decompressed VTU array")?;
    let mut cursor = header_end;
    // Fixed scratch avoids a second growable output vector per block.
    let mut scratch = [0u8; 8192];
    for block in 0..num_blocks {
        let size = count(3 + block)?;
        let stream = payload
            .get(cursor..cursor + size)
            .ok_or("compressed DataArray block is truncated")?;
        let mut decoder = flate2::read::ZlibDecoder::new(stream);
        let mut remaining = if block + 1 == num_blocks {
            last_size
        } else {
            block_size
        };
        while remaining > 0 {
            let length = remaining.min(scratch.len());
            decoder
                .read_exact(&mut scratch[..length])
                .map_err(|e| format!("zlib decompression failed: {e}"))?;
            out.extend_from_slice(&scratch[..length]);
            remaining -= length;
        }
        let mut extra = [0u8; 1];
        if decoder
            .read(&mut extra)
            .map_err(|e| format!("zlib decompression failed: {e}"))?
            != 0
        {
            return Err("decompressed block exceeds its declared size".into());
        }
        cursor += size;
    }
    if out.len() != expected {
        return Err(format!(
            "decompressed size {} does not match header ({expected})",
            out.len()
        ));
    }
    Ok(out)
}

fn encode_block(raw: &[u8], compress: bool) -> String {
    if compress {
        base64::encode(&compress_block(raw))
    } else {
        binary_block(raw)
    }
}

fn f64_block_mode(values: &[f64], compress: bool) -> String {
    let mut raw = Vec::with_capacity(values.len() * 8);
    for v in values {
        raw.extend_from_slice(&v.to_le_bytes());
    }
    encode_block(&raw, compress)
}

fn data_array(out: &mut String, ty: &str, name: Option<&str>, components: usize, encoded: &str) {
    let name_attr = name.map_or(String::new(), |n| format!(" Name=\"{n}\""));
    let _ = write!(
        out,
        "<DataArray type=\"{ty}\"{name_attr} NumberOfComponents=\"{components}\" format=\"binary\">{encoded}</DataArray>"
    );
}

fn write_field_arrays(out: &mut String, fields: &[Field], compress: bool) {
    for field in fields {
        match &field.data {
            FieldData::Scalar(values) => {
                data_array(
                    out,
                    "Float64",
                    Some(&field.name),
                    1,
                    &f64_block_mode(values, compress),
                );
            }
            FieldData::Vector(values) => {
                let flat: Vec<f64> = values.iter().flatten().copied().collect();
                data_array(
                    out,
                    "Float64",
                    Some(&field.name),
                    3,
                    &f64_block_mode(&flat, compress),
                );
            }
            FieldData::Tensor(values) => {
                let flat: Vec<f64> = values.iter().flatten().copied().collect();
                data_array(
                    out,
                    "Float64",
                    Some(&field.name),
                    6,
                    &f64_block_mode(&flat, compress),
                );
            }
            FieldData::Complex(values) => {
                let flat: Vec<f64> = values.iter().flatten().copied().collect();
                data_array(
                    out,
                    "Float64",
                    Some(&field.name),
                    2,
                    &f64_block_mode(&flat, compress),
                );
            }
        }
    }
}

/// Serializes a dataset as VTU XML text with inline binary arrays.
#[must_use]
pub fn write_vtu(dataset: &DataSet) -> String {
    write_vtu_inner(dataset, false)
}

/// Serializes a dataset as VTU with zlib-compressed binary arrays
/// (`compressor="vtkZLibDataCompressor"`, the VTK compressed layout).
#[must_use]
pub fn write_vtu_compressed(dataset: &DataSet) -> String {
    write_vtu_inner(dataset, true)
}

fn write_vtu_inner(dataset: &DataSet, compress: bool) -> String {
    let mut out = String::new();
    let compressor = if compress {
        " compressor=\"vtkZLibDataCompressor\""
    } else {
        ""
    };
    let _ = write!(
        out,
        "<VTKFile type=\"UnstructuredGrid\" version=\"1.0\" byte_order=\"LittleEndian\" header_type=\"UInt64\"{compressor}>\
         <UnstructuredGrid><Piece NumberOfPoints=\"{}\" NumberOfCells=\"{}\">",
        dataset.point_count(),
        dataset.cell_count()
    );
    out.push_str("<Points>");
    let points: Vec<f64> = dataset.positions.iter().flatten().copied().collect();
    data_array(
        &mut out,
        "Float64",
        None,
        3,
        &f64_block_mode(&points, compress),
    );
    out.push_str("</Points><Cells>");
    let mut conn_raw = Vec::with_capacity(dataset.cells.connectivity.len() * 4);
    for index in dataset.cells.connectivity.iter() {
        conn_raw.extend_from_slice(&index.to_le_bytes());
    }
    data_array(
        &mut out,
        "Int32",
        Some("connectivity"),
        1,
        &encode_block(&conn_raw, compress),
    );
    let mut offsets_raw = Vec::with_capacity(dataset.cells.offsets.len() * 8);
    for offset in dataset.cells.offsets.iter().skip(1) {
        offsets_raw.extend_from_slice(&offset.to_le_bytes());
    }
    data_array(
        &mut out,
        "Int64",
        Some("offsets"),
        1,
        &encode_block(&offsets_raw, compress),
    );
    let types: Vec<u8> = dataset
        .cells
        .types
        .iter()
        .map(|t| vtk_code(*t) as u8)
        .collect();
    data_array(
        &mut out,
        "UInt8",
        Some("types"),
        1,
        &encode_block(&types, compress),
    );
    out.push_str("</Cells><PointData>");
    write_field_arrays(&mut out, &dataset.point_data, compress);
    out.push_str("</PointData><CellData>");
    write_field_arrays(&mut out, &dataset.cell_data, compress);
    out.push_str("</CellData></Piece></UnstructuredGrid></VTKFile>");
    out
}

/// One parsed `DataArray` element.
struct RawArray<'a> {
    name: Option<String>,
    ty: String,
    components: usize,
    format: String,
    body: &'a str,
    offset: Option<usize>,
}

fn parse_arrays<'a>(section: &'a str, budget: &mut VtuBudget) -> Result<Vec<RawArray<'a>>, String> {
    let mut arrays = Vec::new();
    let mut rest = section;
    while let Some(start) = rest.find("<DataArray") {
        if budget.arrays >= budget.limits.max_arrays {
            return Err("VTU array count budget exceeded".into());
        }
        budget.arrays += 1;
        let after = &rest[start..];
        let tag_end = after.find('>').ok_or("unterminated VTU DataArray tag")?;
        let tag = &after[..tag_end];
        let attr = |key: &str| -> Option<String> {
            let pattern = format!("{key}=\"");
            let idx = tag.find(&pattern)? + pattern.len();
            let end = tag[idx..].find('"')? + idx;
            Some(tag[idx..end].to_string())
        };
        let self_closing = tag.ends_with('/');
        let (body, advance) = if self_closing {
            ("", tag_end + 1)
        } else {
            let close = after[tag_end..]
                .find("</DataArray>")
                .map(|c| tag_end + c)
                .ok_or("unterminated VTU DataArray body")?;
            (
                after[tag_end + 1..close].trim(),
                close + "</DataArray>".len().min(after.len() - close),
            )
        };
        arrays.push(RawArray {
            name: attr("Name"),
            ty: attr("type").unwrap_or_default(),
            components: attr("NumberOfComponents")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            format: attr("format").unwrap_or_else(|| "ascii".into()),
            body,
            offset: attr("offset").and_then(|v| v.parse().ok()),
        });
        rest = &after[advance.min(after.len())..];
        if advance == 0 {
            break;
        }
    }
    Ok(arrays)
}

fn section<'a>(xml: &'a str, open: &str, close: &str) -> &'a str {
    let Some(start) = xml.find(open).map(|i| i + open.len()) else {
        return "";
    };
    let end = xml[start..].find(close).map_or(xml.len(), |i| start + i);
    &xml[start..end]
}

fn decode_array(
    array: &RawArray<'_>,
    appended: Option<&[u8]>,
    compressed: bool,
    budget: &mut VtuBudget,
) -> Result<Vec<u8>, String> {
    match array.format.as_str() {
        "binary" => {
            let payload = base64::decode(array.body)?;
            if compressed {
                return decompress_block(&payload, budget);
            }
            if payload.len() < 8 {
                return Err("binary DataArray shorter than its header".into());
            }
            let declared = usize::try_from(u64::from_le_bytes(
                payload[..8]
                    .try_into()
                    .map_err(|_| "bad header".to_string())?,
            ))
            .map_err(|_| "VTU byte count exceeds address space")?;
            let raw = &payload[8..];
            if raw.len() < declared {
                return Err(format!(
                    "binary DataArray truncated: header says {declared} bytes, found {}",
                    raw.len()
                ));
            }
            budget.array(declared)?;
            Ok(raw[..declared].to_vec())
        }
        "appended" => {
            let blob = appended.ok_or("appended DataArray without AppendedData section")?;
            let offset = array
                .offset
                .ok_or("appended DataArray without offset attribute")?;
            let data = blob.get(offset..).ok_or("appended offset out of range")?;
            if compressed {
                return decompress_block(data, budget);
            }
            if data.len() < 8 {
                return Err("appended DataArray shorter than its header".into());
            }
            let declared = usize::try_from(u64::from_le_bytes(
                data[..8].try_into().map_err(|_| "bad header".to_string())?,
            ))
            .map_err(|_| "VTU byte count exceeds address space")?;
            let raw = &data[8..];
            if raw.len() < declared {
                return Err("appended DataArray truncated".into());
            }
            budget.array(declared)?;
            Ok(raw[..declared].to_vec())
        }
        "ascii" => Err("ascii VTU arrays are not supported by this reader".into()),
        other => Err(format!("unsupported DataArray format '{other}'")),
    }
}

/// Extracts the raw `AppendedData` blob from file bytes, when present.
fn appended_blob(bytes: &[u8]) -> Option<&[u8]> {
    let start_marker = b"<AppendedData";
    let start = bytes
        .windows(start_marker.len())
        .position(|w| w == start_marker)?;
    let tag_end = bytes[start..].iter().position(|&b| b == b'>')? + start;
    let mut blob_start = tag_end + 1;
    if bytes.get(blob_start) == Some(&b'_') {
        blob_start += 1;
    }
    let end_marker = b"</AppendedData>";
    let end = bytes[blob_start..]
        .windows(end_marker.len())
        .rposition(|w| w == end_marker)?
        + blob_start;
    Some(&bytes[blob_start..end])
}

fn array_f64(
    array: &RawArray<'_>,
    appended: Option<&[u8]>,
    compressed: bool,
    budget: &mut VtuBudget,
) -> Result<Vec<f64>, String> {
    let raw = decode_array(array, appended, compressed, budget)?;
    let element_size = match array.ty.as_str() {
        "Float64" | "Double" => 8,
        "Float32" | "Float" => 4,
        other => return Err(format!("expected a float array, found type '{other}'")),
    };
    if !raw.len().is_multiple_of(element_size) {
        return Err("partial VTU numeric element".into());
    }
    budget.reserve(
        (raw.len() / element_size)
            .checked_mul(8)
            .ok_or("numeric VTU size overflow")?,
    )?;
    match array.ty.as_str() {
        "Float64" | "Double" => Ok(raw
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| f64::from_le_bytes(*c))
            .collect()),
        "Float32" | "Float" => Ok(raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f64::from(f32::from_le_bytes(*c)))
            .collect()),
        other => Err(format!("expected a float array, found type '{other}'")),
    }
}

fn array_i64(
    array: &RawArray<'_>,
    appended: Option<&[u8]>,
    compressed: bool,
    budget: &mut VtuBudget,
) -> Result<Vec<i64>, String> {
    let raw = decode_array(array, appended, compressed, budget)?;
    let element_size = match array.ty.as_str() {
        "Int64" => 8,
        "Int32" | "UInt32" => 4,
        "UInt8" => 1,
        other => return Err(format!("expected an integer array, found type '{other}'")),
    };
    if !raw.len().is_multiple_of(element_size) {
        return Err("partial VTU numeric element".into());
    }
    budget.reserve(
        (raw.len() / element_size)
            .checked_mul(8)
            .ok_or("numeric VTU size overflow")?,
    )?;
    match array.ty.as_str() {
        "Int64" => Ok(raw
            .as_chunks::<8>()
            .0
            .iter()
            .map(|c| i64::from_le_bytes(*c))
            .collect()),
        "Int32" => Ok(raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i64::from(i32::from_le_bytes(*c)))
            .collect()),
        "UInt32" => Ok(raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| i64::from(u32::from_le_bytes(*c)))
            .collect()),
        "UInt8" => Ok(raw.iter().map(|b| i64::from(*b)).collect()),
        other => Err(format!("expected an integer array, found type '{other}'")),
    }
}

fn field_from_array(
    array: &RawArray<'_>,
    appended: Option<&[u8]>,
    compressed: bool,
    association: Association,
    budget: &mut VtuBudget,
) -> Result<Option<Field>, String> {
    let Some(name) = &array.name else {
        return Ok(None);
    };
    let values = array_f64(array, appended, compressed, budget)?;
    let mut field = match array.components {
        1 => Field::scalar(name.clone(), values),
        3 => {
            if values.len() % 3 != 0 {
                return Err(format!(
                    "vector field '{name}' length is not a multiple of 3"
                ));
            }
            Field::vector(
                name.clone(),
                values
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|c| [c[0], c[1], c[2]])
                    .collect::<Vec<_>>(),
            )
        }
        6 => {
            if values.len() % 6 != 0 {
                return Err(format!(
                    "tensor field '{name}' length is not a multiple of 6"
                ));
            }
            let mut field = Field::scalar(name.clone(), Vec::<f64>::new());
            field.data = FieldData::Tensor(
                values
                    .as_chunks::<6>()
                    .0
                    .iter()
                    .map(|c| [c[0], c[1], c[2], c[3], c[4], c[5]])
                    .collect::<Vec<_>>()
                    .into(),
            );
            field
        }
        2 => {
            if values.len() % 2 != 0 {
                return Err(format!("complex field '{name}' length is not even"));
            }
            let mut field = Field::scalar(name.clone(), Vec::<f64>::new());
            field.data = FieldData::Complex(
                values
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|c| [c[0], c[1]])
                    .collect::<Vec<_>>()
                    .into(),
            );
            field
        }
        other => {
            return Err(format!(
                "field '{name}' has unsupported component count {other}"
            ));
        }
    };
    field.association = association;
    Ok(Some(field))
}

/// Parses VTU XML text with inline binary arrays.
///
/// # Errors
///
/// Returns an error for structural problems or exceeded default read limits.
pub fn read_vtu(xml: &str) -> Result<DataSet, String> {
    read_vtu_with_limits(xml, VtuReadLimits::default())
}

/// Parses VTU text within explicit input and decoded-array budgets.
///
/// # Errors
///
/// Returns an error for malformed input or an exceeded allocation limit.
pub fn read_vtu_with_limits(xml: &str, limits: VtuReadLimits) -> Result<DataSet, String> {
    read_vtu_bytes_with_limits(xml.as_bytes(), limits)
}

/// Reads a VTU file from raw bytes; required for `format="appended"`
/// files whose `AppendedData` blob is arbitrary binary.
///
/// # Errors
///
/// Returns an error for structural problems or exceeded default read limits.
pub fn read_vtu_bytes(bytes: &[u8]) -> Result<DataSet, String> {
    read_vtu_bytes_with_limits(bytes, VtuReadLimits::default())
}

/// Parses inline or appended VTU bytes within explicit allocation limits.
///
/// # Errors
///
/// Returns an error for malformed input, invalid XML encoding, or exceeded limits.
pub fn read_vtu_bytes_with_limits(bytes: &[u8], limits: VtuReadLimits) -> Result<DataSet, String> {
    if bytes.len() > limits.max_input_bytes {
        return Err("VTU input byte budget exceeded".into());
    }
    // Raw appended data is not XML text and must not be copied/UTF-8-expanded.
    let end = bytes
        .windows(b"<AppendedData".len())
        .position(|part| part == b"<AppendedData")
        .unwrap_or(bytes.len());
    let xml = std::str::from_utf8(&bytes[..end]).map_err(|_| "VTU XML is not UTF-8")?;
    let mut budget = VtuBudget {
        limits,
        decoded: 0,
        arrays: 0,
    };
    read_vtu_impl(xml, appended_blob(bytes), &mut budget)
}

fn read_vtu_impl(
    xml: &str,
    appended: Option<&[u8]>,
    budget: &mut VtuBudget,
) -> Result<DataSet, String> {
    if !xml.contains("UnstructuredGrid") {
        return Err("not a VTU unstructured-grid file".into());
    }
    let compressed = xml.contains("vtkZLibDataCompressor");
    // Points.
    let points_section = section(xml, "<Points>", "</Points>");
    let point_arrays = parse_arrays(points_section, budget)?;
    let point_array = point_arrays
        .first()
        .ok_or("VTU file has no Points DataArray")?;
    let flat = array_f64(point_array, appended, compressed, budget)?;
    if flat.len() % 3 != 0 {
        return Err("points array length is not a multiple of 3".into());
    }
    let positions: Vec<[f64; 3]> = flat
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| [c[0], c[1], c[2]])
        .collect();

    // Cells.
    let cells_section = section(xml, "<Cells>", "</Cells>");
    let cell_arrays = parse_arrays(cells_section, budget)?;
    let find = |name: &str| {
        cell_arrays
            .iter()
            .find(|a| a.name.as_deref() == Some(name))
            .ok_or_else(|| format!("VTU Cells section lacks '{name}'"))
    };
    let connectivity: Vec<u32> = array_i64(find("connectivity")?, appended, compressed, budget)?
        .into_iter()
        .map(|v| u32::try_from(v).map_err(|_| "VTU point index is outside u32 range".to_string()))
        .collect::<Result<_, _>>()?;
    let mut offsets: Vec<u64> = vec![0];
    for value in array_i64(find("offsets")?, appended, compressed, budget)? {
        offsets.push(u64::try_from(value).map_err(|_| "negative VTU cell offset")?);
    }
    let types: Vec<CellType> = array_i64(find("types")?, appended, compressed, budget)?
        .into_iter()
        .map(|code| {
            u32::try_from(code)
                .ok()
                .and_then(from_vtk_code)
                .ok_or_else(|| format!("unsupported VTK cell type {code}"))
        })
        .collect::<Result<_, _>>()?;
    let cells = Cells {
        types: types.into(),
        offsets: offsets.into(),
        connectivity: connectivity.into(),
    };

    let mut dataset = DataSet::new("vtu", positions, cells);
    for array in parse_arrays(section(xml, "<PointData>", "</PointData>"), budget)? {
        if let Some(field) =
            field_from_array(&array, appended, compressed, Association::Point, budget)?
        {
            dataset.point_data.push(field);
        }
    }
    for array in parse_arrays(section(xml, "<CellData>", "</CellData>"), budget)? {
        if let Some(field) =
            field_from_array(&array, appended, compressed, Association::Cell, budget)?
        {
            dataset.cell_data.push(field);
        }
    }
    dataset
        .validate()
        .map_err(|e| format!("parsed dataset is invalid: {e}"))?;
    Ok(dataset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_binary_vtu() {
        let dataset = DataSet::structured_grid("room", [3, 2, 2], [0.0; 3], [0.5; 3])
            .unwrap()
            .with_field(Field::scalar("pressure", vec![1.5; 12]))
            .with_field(Field::vector("velocity", vec![[1.0, 2.0, 3.0]; 12]))
            .with_field(Field::scalar("cell_id", vec![0.0, 1.0]).on_cells());
        let xml = write_vtu(&dataset);
        assert!(xml.contains("format=\"binary\""));
        let parsed = read_vtu(&xml).unwrap();
        assert_eq!(parsed.point_count(), 12);
        assert_eq!(parsed.cell_count(), 2);
        assert_eq!(parsed.cells.types[0], CellType::Hexahedron);
        assert_eq!(
            parsed
                .field(Association::Point, "pressure")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[1.5; 12]
        );
        assert_eq!(
            parsed
                .field(Association::Point, "velocity")
                .unwrap()
                .as_vector()
                .unwrap()[0],
            [1.0, 2.0, 3.0]
        );
        assert_eq!(
            parsed
                .field(Association::Cell, "cell_id")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[0.0, 1.0]
        );
        assert_eq!(parsed.validate(), Ok(()));
    }

    #[test]
    fn rejects_non_vtu_input() {
        let err = read_vtu("<html></html>").unwrap_err();
        assert!(err.contains("not a VTU"));
    }

    fn budget(limits: VtuReadLimits) -> VtuBudget {
        VtuBudget {
            limits,
            decoded: 0,
            arrays: 0,
        }
    }

    #[test]
    fn forged_compression_headers_fail_before_reserving_output() {
        for words in [[u64::MAX, 1, 1], [1, u64::MAX, u64::MAX], [2, u64::MAX, 1]] {
            let mut payload: Vec<u8> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
            payload.extend_from_slice(&0u64.to_le_bytes());
            let mut allowance = budget(VtuReadLimits::default());
            decompress_block(&payload, &mut allowance).unwrap_err();
            assert_eq!(allowance.decoded, 0);
        }
    }

    #[test]
    fn decompression_stops_at_declared_block_size() {
        let mut payload = compress_block(&[42u8; 4096]);
        payload[16..24].copy_from_slice(&8u64.to_le_bytes());
        let mut allowance = budget(VtuReadLimits::default());
        assert!(
            decompress_block(&payload, &mut allowance)
                .unwrap_err()
                .contains("exceeds its declared size")
        );
        assert_eq!(allowance.decoded, 8);
    }

    #[test]
    fn explicit_budgets_cover_input_arrays_and_numeric_expansion() {
        let dataset = DataSet::surface(
            "s",
            vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        for xml in [write_vtu(&dataset), write_vtu_compressed(&dataset)] {
            for limits in [
                VtuReadLimits {
                    max_input_bytes: xml.len() - 1,
                    ..VtuReadLimits::default()
                },
                VtuReadLimits {
                    max_array_bytes: 8,
                    ..VtuReadLimits::default()
                },
                VtuReadLimits {
                    max_decoded_bytes: 80,
                    ..VtuReadLimits::default()
                },
                VtuReadLimits {
                    max_arrays: 1,
                    ..VtuReadLimits::default()
                },
            ] {
                assert!(
                    read_vtu_with_limits(&xml, limits)
                        .unwrap_err()
                        .contains("budget")
                );
            }
            assert_eq!(read_vtu(&xml).unwrap().point_count(), 3);
        }
        let bytes = write_vtu_appended(&dataset);
        assert_eq!(read_vtu_bytes(&bytes).unwrap().cell_count(), 1);
    }

    #[test]
    fn partial_numeric_elements_are_rejected() {
        let body = binary_block(&[0; 7]);
        let array = RawArray {
            name: None,
            ty: "Float64".into(),
            components: 1,
            format: "binary".into(),
            body: &body,
            offset: None,
        };
        assert!(
            array_f64(&array, None, false, &mut budget(VtuReadLimits::default()))
                .unwrap_err()
                .contains("partial")
        );
    }

    #[test]
    fn absent_sections_do_not_reparse_the_whole_document() {
        assert_eq!(
            section("<Cells>data</Cells>", "<PointData>", "</PointData>"),
            ""
        );
        let mut allowance = budget(VtuReadLimits::default());
        assert!(parse_arrays("<DataArray type=\"Float64\">abc", &mut allowance).is_err());
    }
}

/// Raw bytes of one field array (Float64 payloads, VTK component order).
fn field_raw_bytes(field: &Field) -> (usize, Vec<u8>) {
    let flat: Vec<f64> = match &field.data {
        FieldData::Scalar(values) => values.iter().copied().collect(),
        FieldData::Vector(values) => values.iter().flatten().copied().collect(),
        FieldData::Tensor(values) => values.iter().flatten().copied().collect(),
        FieldData::Complex(values) => values.iter().flatten().copied().collect(),
    };
    let components = match &field.data {
        FieldData::Scalar(_) => 1,
        FieldData::Vector(_) => 3,
        FieldData::Tensor(_) => 6,
        FieldData::Complex(_) => 2,
    };
    let mut raw = Vec::with_capacity(flat.len() * 8);
    for v in flat {
        raw.extend_from_slice(&v.to_le_bytes());
    }
    (components, raw)
}

/// Serializes a dataset as VTU with `format="appended"` raw data.
///
/// Returns the complete file bytes: XML header, then an
/// `<AppendedData encoding="raw">` section holding every array as a
/// `UInt64` length header followed by its raw bytes. Read it back with
/// [`read_vtu_bytes`].
#[must_use]
pub fn write_vtu_appended(dataset: &DataSet) -> Vec<u8> {
    fn add_array(
        blob: &mut Vec<u8>,
        tags: &mut Vec<String>,
        ty: &str,
        name: Option<&str>,
        components: usize,
        raw: &[u8],
    ) {
        let offset = blob.len();
        blob.extend_from_slice(&(raw.len() as u64).to_le_bytes());
        blob.extend_from_slice(raw);
        let name_attr = name.map_or(String::new(), |n| format!(" Name=\"{n}\""));
        tags.push(format!(
            "<DataArray type=\"{ty}\"{name_attr} NumberOfComponents=\"{components}\" format=\"appended\" offset=\"{offset}\"/>"
        ));
    }
    let mut blob: Vec<u8> = Vec::new();
    let mut array_tags: Vec<String> = Vec::new();
    let points: Vec<f64> = dataset.positions.iter().flatten().copied().collect();
    let mut points_raw = Vec::with_capacity(points.len() * 8);
    for v in &points {
        points_raw.extend_from_slice(&v.to_le_bytes());
    }
    add_array(&mut blob, &mut array_tags, "Float64", None, 3, &points_raw);
    let points_tag = array_tags.pop().unwrap_or_default();

    let mut conn_raw = Vec::with_capacity(dataset.cells.connectivity.len() * 4);
    for index in dataset.cells.connectivity.iter() {
        conn_raw.extend_from_slice(&index.to_le_bytes());
    }
    add_array(
        &mut blob,
        &mut array_tags,
        "Int32",
        Some("connectivity"),
        1,
        &conn_raw,
    );
    let mut offsets_raw = Vec::with_capacity(dataset.cells.offsets.len() * 8);
    for offset in dataset.cells.offsets.iter().skip(1) {
        offsets_raw.extend_from_slice(&offset.to_le_bytes());
    }
    add_array(
        &mut blob,
        &mut array_tags,
        "Int64",
        Some("offsets"),
        1,
        &offsets_raw,
    );
    let types: Vec<u8> = dataset
        .cells
        .types
        .iter()
        .map(|t| vtk_code(*t) as u8)
        .collect();
    add_array(
        &mut blob,
        &mut array_tags,
        "UInt8",
        Some("types"),
        1,
        &types,
    );
    let cell_tags: Vec<String> = std::mem::take(&mut array_tags);

    let mut point_tags: Vec<String> = Vec::new();
    for field in &dataset.point_data {
        let (components, raw) = field_raw_bytes(field);
        let offset = blob.len();
        blob.extend_from_slice(&(raw.len() as u64).to_le_bytes());
        blob.extend_from_slice(&raw);
        point_tags.push(format!(
            "<DataArray type=\"Float64\" Name=\"{}\" NumberOfComponents=\"{components}\" format=\"appended\" offset=\"{offset}\"/>",
            field.name
        ));
    }
    let mut cell_data_tags: Vec<String> = Vec::new();
    for field in &dataset.cell_data {
        let (components, raw) = field_raw_bytes(field);
        let offset = blob.len();
        blob.extend_from_slice(&(raw.len() as u64).to_le_bytes());
        blob.extend_from_slice(&raw);
        cell_data_tags.push(format!(
            "<DataArray type=\"Float64\" Name=\"{}\" NumberOfComponents=\"{components}\" format=\"appended\" offset=\"{offset}\"/>",
            field.name
        ));
    }

    let mut out = String::new();
    let _ = write!(
        out,
        "<VTKFile type=\"UnstructuredGrid\" version=\"1.0\" byte_order=\"LittleEndian\" header_type=\"UInt64\">\
         <UnstructuredGrid><Piece NumberOfPoints=\"{}\" NumberOfCells=\"{}\">",
        dataset.point_count(),
        dataset.cell_count()
    );
    out.push_str("<Points>");
    out.push_str(&points_tag);
    out.push_str("</Points><Cells>");
    for tag in &cell_tags {
        out.push_str(tag);
    }
    out.push_str("</Cells><PointData>");
    for tag in &point_tags {
        out.push_str(tag);
    }
    out.push_str("</PointData><CellData>");
    for tag in &cell_data_tags {
        out.push_str(tag);
    }
    out.push_str("</CellData></Piece></UnstructuredGrid>");
    let mut bytes = out.into_bytes();
    bytes.extend_from_slice(b"<AppendedData encoding=\"raw\">_");
    bytes.extend_from_slice(&blob);
    bytes.extend_from_slice(b"</AppendedData></VTKFile>");
    bytes
}

#[cfg(test)]
mod appended_tests {
    use super::*;

    fn sample() -> DataSet {
        DataSet::structured_grid("room", [3, 2, 2], [0.0; 3], [0.5; 3])
            .unwrap()
            .with_field(Field::scalar("pressure", vec![1.5; 12]))
            .with_field(Field::vector("velocity", vec![[1.0, 2.0, 3.0]; 12]))
            .with_field(Field::scalar("cell_id", vec![0.0, 1.0]).on_cells())
    }

    #[test]
    fn compressed_and_appended_round_trip() {
        let dataset = sample();
        let back = read_vtu(&write_vtu_compressed(&dataset)).unwrap();
        assert_eq!(back.positions, dataset.positions);
        assert_eq!(
            back.field(Association::Point, "pressure")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[1.5; 12]
        );
        let back = read_vtu_bytes(&write_vtu_appended(&dataset)).unwrap();
        assert_eq!(back.positions, dataset.positions);
        assert_eq!(back.cells.types, dataset.cells.types);
        assert_eq!(
            back.field(Association::Cell, "cell_id")
                .unwrap()
                .as_scalar()
                .unwrap(),
            &[0.0, 1.0]
        );
    }
}
