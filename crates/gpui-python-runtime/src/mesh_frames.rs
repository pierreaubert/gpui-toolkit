//! Bounded binary transport for Python mesh resources.
//!
//! Mesh headers stay JSON so they share the session protocol's diagnostics,
//! while array bytes are written verbatim after the header line.  A resource
//! may be split into several frames; the store validates the complete shape
//! only after all chunks have arrived.

#[cfg(feature = "showcase")]
use d3rs::mesh::{ScalarField, TriangleMesh};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use thiserror::Error;

pub const MAX_MESH_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MESH_RESOURCE_BYTES: usize = 1 << 30;
const MAX_MESH_CHUNKS: u32 = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshFrameKind {
    Geometry,
    Field,
    Mask,
    Ids,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MeshDtype {
    #[serde(rename = "f32le")]
    F32LE,
    #[serde(rename = "f64le")]
    F64LE,
    #[serde(rename = "u32le")]
    U32LE,
    #[serde(rename = "u64le")]
    U64LE,
    #[serde(rename = "bool_packed")]
    BoolPacked,
    #[serde(rename = "bool_bytes")]
    BoolBytes,
    #[serde(rename = "f32be")]
    F32BE,
    #[serde(rename = "f64be")]
    F64BE,
    #[serde(rename = "u32be")]
    U32BE,
    #[serde(rename = "u64be")]
    U64BE,
}

impl MeshDtype {
    fn is_little_endian(self) -> bool {
        matches!(
            self,
            Self::F32LE
                | Self::F64LE
                | Self::U32LE
                | Self::U64LE
                | Self::BoolPacked
                | Self::BoolBytes
        )
    }

    fn bytes_for(self, elements: usize) -> Option<usize> {
        match self {
            Self::F32LE | Self::F32BE => elements.checked_mul(4),
            Self::F64LE | Self::F64BE => elements.checked_mul(8),
            Self::U32LE | Self::U32BE => elements.checked_mul(4),
            Self::U64LE | Self::U64BE => elements.checked_mul(8),
            Self::BoolPacked => elements.checked_add(7).map(|bytes| bytes / 8),
            Self::BoolBytes => Some(elements),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeshFrame {
    pub resource_id: String,
    pub generation: u64,
    pub sequence: u32,
    pub chunk_count: u32,
    pub kind: MeshFrameKind,
    pub dtype: MeshDtype,
    pub shape: Vec<u32>,
    /// Filled by the framed stdout reader and omitted from the JSON header.
    #[serde(skip)]
    pub payload: Vec<u8>,
}

impl<'de> Deserialize<'de> for MeshFrame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let header = DecodedMeshFrameHeader::deserialize(deserializer)?;
        if header.byte_length > MAX_MESH_FRAME_BYTES {
            return Err(serde::de::Error::custom(format!(
                "mesh frame exceeds the {MAX_MESH_FRAME_BYTES}-byte frame limit"
            )));
        }
        let byte_length = header.byte_length;
        header
            .into_frame(vec![0; byte_length])
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum MeshFrameError {
    #[error("mesh resource id is empty or too long")]
    InvalidId,
    #[error("mesh frame generation must be non-zero")]
    InvalidGeneration,
    #[error("mesh frame chunk count must be between 1 and {max}")]
    InvalidChunkCount { max: u32 },
    #[error("mesh frame sequence {sequence} is outside chunk count {chunk_count}")]
    InvalidSequence { sequence: u32, chunk_count: u32 },
    #[error("mesh frame shape must contain at least one non-zero dimension")]
    InvalidShape,
    #[error("mesh frame shape element count overflows the host address space")]
    ShapeOverflow,
    #[error("mesh dtype uses unsupported big-endian byte order")]
    UnsupportedByteOrder,
    #[error("mesh frame exceeds the {limit}-byte frame limit")]
    FrameTooLarge { limit: usize },
    #[error("mesh resource exceeds the {limit}-byte assembled limit")]
    ResourceTooLarge { limit: usize },
    #[error("mesh payload has {received} bytes; expected {expected}")]
    ShapePayloadMismatch { received: usize, expected: usize },
    #[error("mesh frame header has byte_length {declared}, but payload has {received} bytes")]
    HeaderPayloadMismatch { declared: usize, received: usize },
    #[error("invalid mesh frame header: {message}")]
    InvalidHeader { message: String },
    #[error("unexpected mesh frame type {received:?}")]
    UnexpectedType { received: String },
    #[error("mesh chunk metadata does not match the existing resource assembly")]
    MetadataMismatch,
    #[error("mesh resource has {received} assembled bytes; expected {expected}")]
    AssembledPayloadMismatch { received: usize, expected: usize },
}

#[derive(Debug, Serialize)]
struct MeshFrameHeader<'a> {
    #[serde(rename = "type")]
    message_type: &'static str,
    resource_id: &'a str,
    generation: u64,
    sequence: u32,
    chunk_count: u32,
    kind: MeshFrameKind,
    dtype: MeshDtype,
    shape: &'a [u32],
    byte_length: usize,
}

#[derive(Debug, Deserialize)]
struct DecodedMeshFrameHeader {
    #[serde(rename = "type", default)]
    message_type: Option<String>,
    resource_id: String,
    generation: u64,
    sequence: u32,
    chunk_count: u32,
    kind: MeshFrameKind,
    dtype: MeshDtype,
    shape: Vec<u32>,
    byte_length: usize,
}

impl DecodedMeshFrameHeader {
    fn into_frame(self, payload: Vec<u8>) -> Result<MeshFrame, MeshFrameError> {
        if let Some(message_type) = self.message_type
            && message_type != "mesh_frame"
        {
            return Err(MeshFrameError::UnexpectedType {
                received: message_type,
            });
        }
        if self.byte_length != payload.len() {
            return Err(MeshFrameError::HeaderPayloadMismatch {
                declared: self.byte_length,
                received: payload.len(),
            });
        }
        Ok(MeshFrame {
            resource_id: self.resource_id,
            generation: self.generation,
            sequence: self.sequence,
            chunk_count: self.chunk_count,
            kind: self.kind,
            dtype: self.dtype,
            shape: self.shape,
            payload,
        })
    }
}

impl MeshFrame {
    /// Encode a JSON header, newline, exact payload, and the framing newline.
    pub fn encode(&self) -> Vec<u8> {
        let header = MeshFrameHeader {
            message_type: "mesh_frame",
            resource_id: &self.resource_id,
            generation: self.generation,
            sequence: self.sequence,
            chunk_count: self.chunk_count,
            kind: self.kind,
            dtype: self.dtype,
            shape: &self.shape,
            byte_length: self.payload.len(),
        };
        let mut encoded = serde_json::to_vec(&header)
            .expect("MeshFrameHeader contains only infallible serde values");
        encoded.push(b'\n');
        encoded.extend_from_slice(&self.payload);
        encoded.push(b'\n');
        encoded
    }

    /// Decode a header and its already-separated raw payload.
    pub fn decode(header: &str, payload: &[u8]) -> Result<Self, MeshFrameError> {
        let header: DecodedMeshFrameHeader =
            serde_json::from_str(header).map_err(|error| MeshFrameError::InvalidHeader {
                message: error.to_string(),
            })?;
        let frame = header.into_frame(payload.to_vec())?;
        frame.validate()?;
        Ok(frame)
    }

    /// Validate metadata and the payload size permitted for this chunk.
    pub fn validate(&self) -> Result<(), MeshFrameError> {
        if self.resource_id.trim().is_empty() || self.resource_id.len() > 128 {
            return Err(MeshFrameError::InvalidId);
        }
        if self.generation == 0 {
            return Err(MeshFrameError::InvalidGeneration);
        }
        if self.chunk_count == 0 || self.chunk_count > MAX_MESH_CHUNKS {
            return Err(MeshFrameError::InvalidChunkCount {
                max: MAX_MESH_CHUNKS,
            });
        }
        if self.sequence >= self.chunk_count {
            return Err(MeshFrameError::InvalidSequence {
                sequence: self.sequence,
                chunk_count: self.chunk_count,
            });
        }
        if !self.dtype.is_little_endian() {
            return Err(MeshFrameError::UnsupportedByteOrder);
        }
        if self.payload.len() > MAX_MESH_FRAME_BYTES {
            return Err(MeshFrameError::FrameTooLarge {
                limit: MAX_MESH_FRAME_BYTES,
            });
        }
        let elements = shape_elements(&self.shape)?;
        let expected = self
            .dtype
            .bytes_for(elements)
            .ok_or(MeshFrameError::ShapeOverflow)?;
        if expected > MAX_MESH_RESOURCE_BYTES {
            return Err(MeshFrameError::ResourceTooLarge {
                limit: MAX_MESH_RESOURCE_BYTES,
            });
        }
        if self.payload.is_empty() || self.payload.len() > expected {
            return Err(MeshFrameError::ShapePayloadMismatch {
                received: self.payload.len(),
                expected,
            });
        }
        if self.chunk_count == 1 && self.payload.len() != expected {
            return Err(MeshFrameError::ShapePayloadMismatch {
                received: self.payload.len(),
                expected,
            });
        }
        Ok(())
    }

    fn expected_bytes(&self) -> Result<usize, MeshFrameError> {
        let elements = shape_elements(&self.shape)?;
        self.dtype
            .bytes_for(elements)
            .ok_or(MeshFrameError::ShapeOverflow)
    }
}

fn shape_elements(shape: &[u32]) -> Result<usize, MeshFrameError> {
    if shape.is_empty() || shape.contains(&0) {
        return Err(MeshFrameError::InvalidShape);
    }
    shape.iter().try_fold(1_usize, |elements, &dimension| {
        elements
            .checked_mul(dimension as usize)
            .ok_or(MeshFrameError::ShapeOverflow)
    })
}

fn decode_floats(
    resource: &RetainedMeshResource,
    name: &str,
    allow_nan: bool,
) -> Result<Vec<f64>, String> {
    let elements = shape_elements(&resource.shape).map_err(|error| error.to_string())?;
    let bytes_per_value = match resource.dtype {
        MeshDtype::F32LE => 4,
        MeshDtype::F64LE => 8,
        _ => {
            return Err(format!(
                "{name} resource dtype {:?} is not f32le/f64le",
                resource.dtype
            ));
        }
    };
    let expected = elements
        .checked_mul(bytes_per_value)
        .ok_or_else(|| format!("{name} resource payload is too large"))?;
    if resource.payload.len() != expected {
        return Err(format!(
            "{name} resource payload {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    resource
        .payload
        .chunks_exact(bytes_per_value)
        .map(|chunk| {
            let value = match resource.dtype {
                MeshDtype::F32LE => {
                    f32::from_le_bytes(chunk.try_into().map_err(|_| "invalid f32 bytes")?) as f64
                }
                MeshDtype::F64LE => {
                    f64::from_le_bytes(chunk.try_into().map_err(|_| "invalid f64 bytes")?)
                }
                _ => unreachable!("dtype checked above"),
            };
            if value.is_infinite() || (!allow_nan && value.is_nan()) {
                return Err(format!("{name} resource contains non-finite value"));
            }
            Ok(value)
        })
        .collect()
}

fn decode_u32s(resource: &RetainedMeshResource, name: &str) -> Result<Vec<u32>, String> {
    let elements = shape_elements(&resource.shape).map_err(|error| error.to_string())?;
    if resource.dtype != MeshDtype::U32LE {
        return Err(format!(
            "{name} resource dtype {:?} is not u32le",
            resource.dtype
        ));
    }
    let expected = elements
        .checked_mul(4)
        .ok_or_else(|| format!("{name} resource payload is too large"))?;
    if resource.payload.len() != expected {
        return Err(format!(
            "{name} resource payload {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    resource
        .payload
        .chunks_exact(4)
        .map(|chunk| {
            Ok(u32::from_le_bytes(
                chunk.try_into().map_err(|_| "invalid u32 bytes")?,
            ))
        })
        .collect()
}

fn decode_mask(resource: &RetainedMeshResource, value_count: usize) -> Result<Vec<bool>, String> {
    let expected = match resource.dtype {
        MeshDtype::BoolBytes => value_count,
        MeshDtype::BoolPacked => {
            value_count
                .checked_add(7)
                .ok_or_else(|| "field.valid resource payload is too large".to_string())?
                / 8
        }
        _ => {
            return Err(format!(
                "field.valid resource dtype {:?} is not bool_bytes/bool_packed",
                resource.dtype
            ));
        }
    };
    if resource.payload.len() != expected {
        return Err(format!(
            "field.valid resource payload {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    match resource.dtype {
        MeshDtype::BoolBytes => resource
            .payload
            .iter()
            .enumerate()
            .map(|(index, &value)| match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(format!("field.valid resource byte {index} is not boolean")),
            })
            .collect(),
        MeshDtype::BoolPacked => Ok((0..value_count)
            .map(|index| resource.payload[index / 8] & (1 << (index % 8)) != 0)
            .collect()),
        _ => unreachable!("dtype checked above"),
    }
}

fn decode_ids(resource: &RetainedMeshResource, name: &str) -> Result<Vec<u64>, String> {
    let elements = shape_elements(&resource.shape).map_err(|error| error.to_string())?;
    let bytes_per_value = match resource.dtype {
        MeshDtype::U32LE => 4,
        MeshDtype::U64LE => 8,
        _ => {
            return Err(format!(
                "{name} resource dtype {:?} is not u32le/u64le",
                resource.dtype
            ));
        }
    };
    let expected = elements
        .checked_mul(bytes_per_value)
        .ok_or_else(|| format!("{name} resource payload is too large"))?;
    if resource.payload.len() != expected {
        return Err(format!(
            "{name} resource payload {} bytes, expected {expected}",
            resource.payload.len()
        ));
    }
    resource
        .payload
        .chunks_exact(bytes_per_value)
        .map(|chunk| match resource.dtype {
            MeshDtype::U32LE => {
                Ok(u32::from_le_bytes(chunk.try_into().map_err(|_| "invalid u32 bytes")?) as u64)
            }
            MeshDtype::U64LE => Ok(u64::from_le_bytes(
                chunk.try_into().map_err(|_| "invalid u64 bytes")?,
            )),
            _ => unreachable!("dtype checked above"),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetainedMeshResource {
    pub resource_id: String,
    pub generation: u64,
    pub kind: MeshFrameKind,
    pub dtype: MeshDtype,
    pub shape: Vec<u32>,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MeshFrameOutcome {
    Incomplete,
    Assembled(RetainedMeshResource),
    DroppedStale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MeshFrameStats {
    pub resources: usize,
    pub in_flight: usize,
    pub bytes_used: usize,
    pub evictions: u64,
    /// Number of completed resources with at least one active owner.
    pub referenced_resources: usize,
    /// Sum of all active owners across completed resources.
    pub references: usize,
}

#[derive(Debug)]
struct MeshAssembly {
    kind: MeshFrameKind,
    dtype: MeshDtype,
    shape: Vec<u32>,
    chunks: Vec<Option<Vec<u8>>>,
    bytes: usize,
}

#[derive(Debug)]
enum MeshEntry {
    Assembly(MeshAssembly),
    Resource(RetainedMeshResource),
}

type MeshKey = (String, u64);

/// Decoded retained resource data. Entries use the same `(resource_id,
/// generation)` key and are removed alongside their binary resource.
#[derive(Debug, Default)]
struct DecodedMeshCache {
    positions: HashMap<MeshKey, Arc<[[f64; 3]]>>,
    triangles: HashMap<MeshKey, Arc<[[u32; 3]]>>,
    fields: HashMap<MeshKey, Arc<[f64]>>,
    masks: HashMap<MeshKey, Arc<[bool]>>,
    ids: HashMap<MeshKey, Arc<[u64]>>,
    #[cfg(feature = "showcase")]
    triangle_meshes: HashMap<u64, Arc<TriangleMesh>>,
    #[cfg(feature = "showcase")]
    scalar_fields: HashMap<u64, Arc<ScalarField>>,
}

/// Reassembles mesh chunks and retains completed resources under a bounded,
/// deterministic FIFO eviction policy.
#[derive(Debug)]
pub struct MeshFrameStore {
    entries: HashMap<MeshKey, MeshEntry>,
    order: VecDeque<MeshKey>,
    latest_generation: HashMap<String, u64>,
    /// Generations whose latest ingest failed after advancing the monotonic
    /// watermark. These may be retransmitted for recovery; explicit release,
    /// eviction, and clear still make a generation permanently stale.
    recoverable_generations: HashSet<MeshKey>,
    references: HashMap<MeshKey, usize>,
    decoded: RefCell<DecodedMeshCache>,
    byte_budget: usize,
    bytes_used: usize,
    evictions: u64,
}

impl Default for MeshFrameStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshFrameStore {
    pub fn new() -> Self {
        Self::with_budget(MAX_MESH_RESOURCE_BYTES)
    }

    pub fn with_budget(byte_budget: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            latest_generation: HashMap::new(),
            recoverable_generations: HashSet::new(),
            references: HashMap::new(),
            decoded: RefCell::new(DecodedMeshCache::default()),
            byte_budget: byte_budget.clamp(1, MAX_MESH_RESOURCE_BYTES),
            bytes_used: 0,
            evictions: 0,
        }
    }

    pub fn get(&self, resource_id: &str, generation: u64) -> Option<&RetainedMeshResource> {
        match self.entries.get(&(resource_id.to_owned(), generation)) {
            Some(MeshEntry::Resource(resource)) => Some(resource),
            _ => None,
        }
    }

    /// Retain a fully validated mesh object across declarative host rebuilds.
    #[cfg(feature = "showcase")]
    pub(crate) fn cached_triangle_mesh(
        &self,
        key: u64,
        build: impl FnOnce() -> Result<TriangleMesh, String>,
    ) -> Result<Arc<TriangleMesh>, String> {
        if let Some(mesh) = self.decoded.borrow().triangle_meshes.get(&key) {
            return Ok(mesh.clone());
        }
        let mesh = Arc::new(build()?);
        let mut decoded = self.decoded.borrow_mut();
        if decoded.triangle_meshes.len() >= 64 {
            decoded.triangle_meshes.clear();
        }
        decoded.triangle_meshes.insert(key, mesh.clone());
        Ok(mesh)
    }

    /// Retain a complete scalar field across declarative host rebuilds.
    #[cfg(feature = "showcase")]
    pub(crate) fn cached_scalar_field(
        &self,
        key: u64,
        build: impl FnOnce() -> Result<ScalarField, String>,
    ) -> Result<Arc<ScalarField>, String> {
        if let Some(field) = self.decoded.borrow().scalar_fields.get(&key) {
            return Ok(field.clone());
        }
        let field = Arc::new(build()?);
        let mut decoded = self.decoded.borrow_mut();
        if decoded.scalar_fields.len() >= 64 {
            decoded.scalar_fields.clear();
        }
        decoded.scalar_fields.insert(key, field.clone());
        Ok(field)
    }

    /// Decode geometry positions once for the lifetime of a retained resource
    /// generation. The returned `Arc` is cheap to share with repeated plots.
    pub fn decoded_positions(
        &self,
        resource_id: &str,
        generation: u64,
    ) -> Result<Arc<[[f64; 3]]>, String> {
        let key = (resource_id.to_owned(), generation);
        if let Some(positions) = self.decoded.borrow().positions.get(&key) {
            return Ok(positions.clone());
        }
        let resource = self.decode_resource(&key, MeshFrameKind::Geometry, "positions")?;
        if resource.shape.len() != 2 || resource.shape[1] != 3 {
            return Err("geometry.positions resource shape must be [vertex_count, 3]".into());
        }
        let values = decode_floats(resource, "geometry.positions", false)?;
        let positions: Arc<[[f64; 3]]> = values
            .chunks_exact(3)
            .map(|values| [values[0], values[1], values[2]])
            .collect::<Vec<_>>()
            .into();
        self.decoded
            .borrow_mut()
            .positions
            .insert(key, positions.clone());
        Ok(positions)
    }

    /// Decode triangle indices once for the lifetime of a retained resource
    /// generation.
    pub fn decoded_triangles(
        &self,
        resource_id: &str,
        generation: u64,
    ) -> Result<Arc<[[u32; 3]]>, String> {
        let key = (resource_id.to_owned(), generation);
        if let Some(triangles) = self.decoded.borrow().triangles.get(&key) {
            return Ok(triangles.clone());
        }
        let resource = self.decode_resource(&key, MeshFrameKind::Geometry, "triangles")?;
        if resource.shape.len() != 2 || resource.shape[1] != 3 {
            return Err("geometry.triangles resource shape must be [triangle_count, 3]".into());
        }
        let values = decode_u32s(resource, "geometry.triangles")?;
        let triangles: Arc<[[u32; 3]]> = values
            .chunks_exact(3)
            .map(|values| [values[0], values[1], values[2]])
            .collect::<Vec<_>>()
            .into();
        self.decoded
            .borrow_mut()
            .triangles
            .insert(key, triangles.clone());
        Ok(triangles)
    }

    /// Decode scalar field samples once for the lifetime of a retained
    /// resource generation. NaNs are preserved for missing-value policies;
    /// infinities are rejected.
    pub fn decoded_field(&self, resource_id: &str, generation: u64) -> Result<Arc<[f64]>, String> {
        let key = (resource_id.to_owned(), generation);
        if let Some(values) = self.decoded.borrow().fields.get(&key) {
            return Ok(values.clone());
        }
        let resource = self.decode_resource(&key, MeshFrameKind::Field, "field")?;
        if resource.shape.len() != 1 {
            return Err("field resource shape must be [value_count]".into());
        }
        let values: Arc<[f64]> = decode_floats(resource, "field", true)?.into();
        self.decoded.borrow_mut().fields.insert(key, values.clone());
        Ok(values)
    }

    /// Decode a packed or byte-per-value validity mask once for the lifetime
    /// of its retained generation.
    pub fn decoded_mask(
        &self,
        resource_id: &str,
        generation: u64,
        value_count: usize,
    ) -> Result<Arc<[bool]>, String> {
        let key = (resource_id.to_owned(), generation);
        if let Some(mask) = self.decoded.borrow().masks.get(&key) {
            if mask.len() == value_count {
                return Ok(mask.clone());
            }
        }
        let resource = self.decode_resource(&key, MeshFrameKind::Mask, "field.valid")?;
        if resource.shape.len() != 1 || resource.shape[0] as usize != value_count {
            return Err("field.valid resource shape must be [value_count]".into());
        }
        let mask: Arc<[bool]> = decode_mask(resource, value_count)?.into();
        self.decoded.borrow_mut().masks.insert(key, mask.clone());
        Ok(mask)
    }

    /// Decode stable vertex or cell IDs once for the lifetime of their
    /// retained generation.
    pub fn decoded_ids(
        &self,
        resource_id: &str,
        generation: u64,
        expected: usize,
        name: &str,
    ) -> Result<Arc<[u64]>, String> {
        let key = (resource_id.to_owned(), generation);
        if let Some(ids) = self.decoded.borrow().ids.get(&key) {
            if ids.len() == expected {
                return Ok(ids.clone());
            }
        }
        let resource = self.decode_resource(&key, MeshFrameKind::Ids, name)?;
        if resource.shape.len() != 1 || resource.shape[0] as usize != expected {
            return Err(format!("{name} resource shape must be [{expected}]"));
        }
        let ids: Arc<[u64]> = decode_ids(resource, name)?.into();
        self.decoded.borrow_mut().ids.insert(key, ids.clone());
        Ok(ids)
    }

    fn decode_resource<'a>(
        &'a self,
        key: &MeshKey,
        expected_kind: MeshFrameKind,
        name: &str,
    ) -> Result<&'a RetainedMeshResource, String> {
        let resource = match self.entries.get(key) {
            Some(MeshEntry::Resource(resource)) => resource,
            _ => {
                return Err(format!(
                    "missing {name} resource {:?} generation {}",
                    key.0, key.1
                ));
            }
        };
        if resource.kind != expected_kind {
            return Err(format!(
                "{name} resource {:?} has kind {:?}, expected {:?}",
                resource.resource_id, resource.kind, expected_kind
            ));
        }
        Ok(resource)
    }

    pub fn stats(&self) -> MeshFrameStats {
        MeshFrameStats {
            resources: self
                .entries
                .values()
                .filter(|entry| matches!(entry, MeshEntry::Resource(_)))
                .count(),
            in_flight: self
                .entries
                .values()
                .filter(|entry| matches!(entry, MeshEntry::Assembly(_)))
                .count(),
            bytes_used: self.bytes_used,
            evictions: self.evictions,
            referenced_resources: self.references.len(),
            references: self.references.values().sum(),
        }
    }

    /// Keep a completed resource alive while a native MeshPlot references it.
    ///
    /// Resource generations remain immutable. A newer generation may coexist
    /// with a referenced older generation until the old plot releases it.
    pub fn retain(&mut self, resource_id: &str, generation: u64) -> bool {
        let key = (resource_id.to_owned(), generation);
        if !matches!(self.entries.get(&key), Some(MeshEntry::Resource(_))) {
            return false;
        }
        *self.references.entry(key).or_default() += 1;
        true
    }

    /// Release one native owner of a completed resource.
    pub fn release_reference(&mut self, resource_id: &str, generation: u64) -> bool {
        let key = (resource_id.to_owned(), generation);
        let Some(count) = self.references.get_mut(&key) else {
            return false;
        };
        if *count > 1 {
            *count -= 1;
        } else {
            self.references.remove(&key);
        }
        true
    }

    pub fn ingest(&mut self, frame: MeshFrame) -> Result<MeshFrameOutcome, MeshFrameError> {
        frame.validate()?;
        let expected = frame.expected_bytes()?;
        let key = (frame.resource_id.clone(), frame.generation);
        if let Some(current) = self.latest_generation.get(&frame.resource_id).copied() {
            if frame.generation < current {
                return Ok(MeshFrameOutcome::DroppedStale);
            }
            if frame.generation == current
                && !self.entries.contains_key(&key)
                && !self.recoverable_generations.contains(&key)
            {
                // A released, evicted, or cleared generation must not become
                // live again merely because the producer retransmits it.
                return Ok(MeshFrameOutcome::DroppedStale);
            }
            if frame.generation > current {
                self.remove_generations(&frame.resource_id, frame.generation);
            }
        }
        self.latest_generation
            .insert(frame.resource_id.clone(), frame.generation);

        if matches!(self.entries.get(&key), Some(MeshEntry::Resource(_))) {
            return Ok(MeshFrameOutcome::DroppedStale);
        }
        // This is a new attempt after a transient ingest failure. Once a
        // valid chunk is accepted, later duplicate chunks follow the normal
        // immutable-generation rules again.
        self.recoverable_generations.remove(&key);
        if let Some(MeshEntry::Assembly(existing)) = self.entries.get(&key)
            && (existing.kind != frame.kind
                || existing.dtype != frame.dtype
                || existing.shape != frame.shape
                || existing.chunks.len() != frame.chunk_count as usize)
        {
            return Err(MeshFrameError::MetadataMismatch);
        }

        let additional = match self.entries.get(&key) {
            Some(MeshEntry::Assembly(existing))
                if existing.chunks[frame.sequence as usize].is_some() =>
            {
                0
            }
            _ => frame.payload.len(),
        };
        if let Err(error) = self.ensure_capacity(additional, &key) {
            self.recoverable_generations.insert(key);
            return Err(error);
        }

        if !self.entries.contains_key(&key) {
            self.order.push_back(key.clone());
            self.entries.insert(
                key.clone(),
                MeshEntry::Assembly(MeshAssembly {
                    kind: frame.kind,
                    dtype: frame.dtype,
                    shape: frame.shape.clone(),
                    chunks: (0..frame.chunk_count).map(|_| None).collect(),
                    bytes: 0,
                }),
            );
        }
        let received = {
            let Some(entry) = self.entries.get_mut(&key) else {
                self.recoverable_generations.insert(key);
                return Err(MeshFrameError::InvalidHeader {
                    message: "mesh assembly disappeared while ingesting a frame".into(),
                });
            };
            let MeshEntry::Assembly(assembly) = entry else {
                return Ok(MeshFrameOutcome::DroppedStale);
            };
            let chunk = &mut assembly.chunks[frame.sequence as usize];
            if chunk.is_none() {
                assembly.bytes += frame.payload.len();
                self.bytes_used += frame.payload.len();
                *chunk = Some(frame.payload);
            }

            if assembly.chunks.iter().any(Option::is_none) {
                return Ok(MeshFrameOutcome::Incomplete);
            }
            assembly.bytes
        };
        if received != expected {
            self.remove_entry(&key);
            self.recoverable_generations.insert(key.clone());
            return Err(MeshFrameError::AssembledPayloadMismatch { received, expected });
        }
        let Some(MeshEntry::Assembly(assembly)) = self.entries.get_mut(&key) else {
            self.recoverable_generations.insert(key);
            return Err(MeshFrameError::InvalidHeader {
                message: "mesh assembly disappeared before completion".into(),
            });
        };
        let payload = assembly
            .chunks
            .iter_mut()
            .filter_map(Option::take)
            .flatten()
            .collect::<Vec<_>>();
        let resource = RetainedMeshResource {
            resource_id: frame.resource_id,
            generation: frame.generation,
            kind: frame.kind,
            dtype: frame.dtype,
            shape: frame.shape,
            payload,
        };
        self.entries
            .insert(key, MeshEntry::Resource(resource.clone()));
        Ok(MeshFrameOutcome::Assembled(resource))
    }

    pub fn release(&mut self, resource_id: &str, generation: u64) -> bool {
        let key = (resource_id.to_owned(), generation);
        if self.references.contains_key(&key) {
            return false;
        }
        self.recoverable_generations.remove(&key);
        self.remove_entry(&key).is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.recoverable_generations.clear();
        self.references.clear();
        self.decoded.get_mut().fields.clear();
        self.decoded.get_mut().positions.clear();
        self.decoded.get_mut().triangles.clear();
        self.decoded.get_mut().masks.clear();
        self.decoded.get_mut().ids.clear();
        self.bytes_used = 0;
    }

    fn ensure_capacity(
        &mut self,
        additional: usize,
        current: &MeshKey,
    ) -> Result<(), MeshFrameError> {
        if additional > MAX_MESH_RESOURCE_BYTES
            || self.bytes_used.saturating_add(additional) > MAX_MESH_RESOURCE_BYTES
        {
            return Err(MeshFrameError::ResourceTooLarge {
                limit: MAX_MESH_RESOURCE_BYTES,
            });
        }
        while self.bytes_used.saturating_add(additional) > self.byte_budget {
            let Some(index) = self
                .order
                .iter()
                .position(|key| key != current && !self.references.contains_key(key))
            else {
                return Err(MeshFrameError::ResourceTooLarge {
                    limit: self.byte_budget,
                });
            };
            let key = self.order.remove(index).expect("eviction index is valid");
            if self.remove_entry(&key).is_some() {
                self.evictions += 1;
            }
        }
        Ok(())
    }

    fn remove_generations(&mut self, resource_id: &str, keep_generation: u64) {
        let keys = self
            .entries
            .keys()
            .filter(|key| {
                key.0 == resource_id
                    && key.1 < keep_generation
                    && !self.references.contains_key(*key)
            })
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            self.remove_entry(&key);
        }
        self.recoverable_generations
            .retain(|(id, generation)| id != resource_id || *generation >= keep_generation);
    }

    fn remove_entry(&mut self, key: &MeshKey) -> Option<MeshEntry> {
        let entry = self.entries.remove(key)?;
        self.order.retain(|queued| queued != key);
        self.references.remove(key);
        let decoded = self.decoded.get_mut();
        decoded.positions.remove(key);
        decoded.triangles.remove(key);
        decoded.fields.remove(key);
        decoded.masks.remove(key);
        decoded.ids.remove(key);
        // Complete objects may combine several resource generations. Any
        // resource retirement invalidates these small bounded object caches.
        #[cfg(feature = "showcase")]
        {
            decoded.triangle_meshes.clear();
            decoded.scalar_fields.clear();
        }
        self.bytes_used = self.bytes_used.saturating_sub(match &entry {
            MeshEntry::Assembly(assembly) => assembly.bytes,
            MeshEntry::Resource(resource) => resource.payload.len(),
        });
        Some(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MeshFrame {
        let payload = (0..300)
            .flat_map(|value| (value as f64).to_le_bytes())
            .collect();
        MeshFrame {
            resource_id: "geometry".into(),
            generation: 1,
            sequence: 0,
            chunk_count: 1,
            kind: MeshFrameKind::Geometry,
            dtype: MeshDtype::F64LE,
            shape: vec![100, 3],
            payload,
        }
    }

    fn tiny_frame(resource_id: &str, generation: u64, value: u8) -> MeshFrame {
        MeshFrame {
            resource_id: resource_id.into(),
            generation,
            sequence: 0,
            chunk_count: 1,
            kind: MeshFrameKind::Field,
            dtype: MeshDtype::U64LE,
            shape: vec![1],
            payload: vec![value; 8],
        }
    }

    fn split_header(bytes: &[u8]) -> (&str, &[u8]) {
        let separator = bytes.iter().position(|byte| *byte == b'\n').unwrap();
        let end = bytes.len() - 1;
        (
            std::str::from_utf8(&bytes[..separator]).unwrap(),
            &bytes[separator + 1..end],
        )
    }

    #[test]
    fn frame_roundtrip_preserves_payload() {
        let frame = fixture();
        let bytes = frame.encode();
        let (header, payload) = split_header(&bytes);
        let decoded = MeshFrame::decode(header, payload).unwrap();
        assert_eq!(decoded.payload, frame.payload);
    }

    #[test]
    fn json_header_deserialization_presizes_mesh_payload_once() {
        let header = serde_json::json!({
            "type": "mesh_frame",
            "resource_id": "field",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "kind": "field",
            "dtype": "f64le",
            "shape": [1],
            "byte_length": 8,
        });

        let frame: MeshFrame = serde_json::from_value(header).expect("header deserializes");
        assert_eq!(frame.payload, vec![0; 8]);
    }

    #[test]
    fn decoded_positions_are_shared_and_generation_scoped() {
        let mut store = MeshFrameStore::new();
        store.ingest(fixture()).expect("fixture ingests");

        let first = store
            .decoded_positions("geometry", 1)
            .expect("positions decode");
        let repeated = store
            .decoded_positions("geometry", 1)
            .expect("positions reuse");
        assert!(Arc::ptr_eq(&first, &repeated));

        let mut replacement = fixture();
        replacement.generation = 2;
        store.ingest(replacement).expect("replacement ingests");
        let newer = store
            .decoded_positions("geometry", 2)
            .expect("new generation decodes");
        assert!(!Arc::ptr_eq(&first, &newer));
        assert!(store.decoded_positions("geometry", 1).is_err());
    }

    #[test]
    fn decoded_field_preserves_nans_and_reuses_the_arc() {
        let payload = [1.0_f64, f64::NAN]
            .into_iter()
            .flat_map(f64::to_le_bytes)
            .collect();
        let frame = MeshFrame {
            resource_id: "field".into(),
            generation: 1,
            sequence: 0,
            chunk_count: 1,
            kind: MeshFrameKind::Field,
            dtype: MeshDtype::F64LE,
            shape: vec![2],
            payload,
        };
        let mut store = MeshFrameStore::new();
        store.ingest(frame).expect("field ingests");

        let first = store.decoded_field("field", 1).expect("field decodes");
        let repeated = store.decoded_field("field", 1).expect("field reuses");
        assert!(first[1].is_nan());
        assert!(Arc::ptr_eq(&first, &repeated));
    }

    #[test]
    fn big_endian_dtype_rejected() {
        let mut frame = fixture();
        frame.dtype = MeshDtype::F64BE;
        assert!(frame.validate().is_err());
    }

    #[test]
    fn shape_dtype_mismatch_rejected() {
        let mut frame = fixture();
        frame.shape = vec![100, 4];
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::ShapePayloadMismatch { .. })
        ));
    }

    #[test]
    fn chunk_assembly_orders_by_sequence() {
        let payload = (0..12_u32)
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let frames = (0..3)
            .map(|sequence| MeshFrame {
                resource_id: "field".into(),
                generation: 1,
                sequence,
                chunk_count: 3,
                kind: MeshFrameKind::Field,
                dtype: MeshDtype::U32LE,
                shape: vec![12],
                payload: payload[sequence as usize * 16..sequence as usize * 16 + 16].to_vec(),
            })
            .collect::<Vec<_>>();
        let mut store = MeshFrameStore::new();
        assert_eq!(
            store.ingest(frames[2].clone()).unwrap(),
            MeshFrameOutcome::Incomplete
        );
        assert_eq!(
            store.ingest(frames[0].clone()).unwrap(),
            MeshFrameOutcome::Incomplete
        );
        let MeshFrameOutcome::Assembled(resource) = store.ingest(frames[1].clone()).unwrap() else {
            panic!("three chunks should assemble");
        };
        assert_eq!(resource.payload, payload);
    }

    #[test]
    fn oversized_frame_rejected() {
        let header = serde_json::json!({
            "type": "mesh_frame",
            "resource_id": "geometry",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "kind": "geometry",
            "dtype": "f32le",
            "shape": [MAX_MESH_FRAME_BYTES as u32 + 1],
            "byte_length": MAX_MESH_FRAME_BYTES + 1,
        });
        assert!(
            MeshFrame::decode(&header.to_string(), &vec![0; MAX_MESH_FRAME_BYTES + 1]).is_err()
        );
    }

    #[test]
    fn malformed_headers_and_metadata_return_structured_errors() {
        let mut frame = tiny_frame("field", 1, 1);

        frame.resource_id = " ".into();
        assert!(matches!(frame.validate(), Err(MeshFrameError::InvalidId)));
        frame = tiny_frame("field", 1, 1);
        frame.generation = 0;
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::InvalidGeneration)
        ));
        frame = tiny_frame("field", 1, 1);
        frame.chunk_count = 0;
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::InvalidChunkCount { .. })
        ));
        frame = tiny_frame("field", 1, 1);
        frame.chunk_count = 2;
        frame.sequence = 2;
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::InvalidSequence { .. })
        ));
        frame = tiny_frame("field", 1, 1);
        frame.dtype = MeshDtype::BoolPacked;
        frame.shape = vec![8];
        frame.payload = vec![0x01];
        assert!(frame.validate().is_ok());
        frame.dtype = MeshDtype::BoolBytes;
        frame.shape = vec![1];
        assert!(frame.validate().is_ok());
        frame.dtype = MeshDtype::F64LE;
        frame.shape = Vec::new();
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::InvalidShape)
        ));
        frame.shape = vec![0];
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::InvalidShape)
        ));
        frame.shape = vec![u32::MAX; 3];
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::ShapeOverflow)
        ));
        frame.shape = vec![(MAX_MESH_RESOURCE_BYTES / 8 + 1) as u32];
        assert!(matches!(
            frame.validate(),
            Err(MeshFrameError::ResourceTooLarge { .. })
        ));

        assert!(matches!(
            MeshFrame::decode("not json", &[0; 8]),
            Err(MeshFrameError::InvalidHeader { .. })
        ));
        let wrong_type = serde_json::json!({
            "type": "not_mesh_frame",
            "resource_id": "field",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "kind": "field",
            "dtype": "u64le",
            "shape": [1],
            "byte_length": 8,
        });
        assert!(matches!(
            MeshFrame::decode(&wrong_type.to_string(), &[0; 8]),
            Err(MeshFrameError::UnexpectedType { .. })
        ));
        let short_header = serde_json::json!({
            "type": "mesh_frame",
            "resource_id": "field",
            "generation": 1,
            "sequence": 0,
            "chunk_count": 1,
            "kind": "field",
            "dtype": "u64le",
            "shape": [1],
            "byte_length": 7,
        });
        assert!(matches!(
            MeshFrame::decode(&short_header.to_string(), &[0; 8]),
            Err(MeshFrameError::HeaderPayloadMismatch { .. })
        ));
        let mut empty = tiny_frame("field", 1, 1);
        empty.payload.clear();
        assert!(matches!(
            empty.validate(),
            Err(MeshFrameError::ShapePayloadMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_and_incomplete_chunks_preserve_assembly_invariants() {
        let mut first = tiny_frame("assembly", 1, 1);
        first.chunk_count = 2;
        first.payload = vec![1; 4];
        let mut store = MeshFrameStore::new();
        assert_eq!(
            store.ingest(first.clone()).unwrap(),
            MeshFrameOutcome::Incomplete
        );
        assert_eq!(
            store.ingest(first.clone()).unwrap(),
            MeshFrameOutcome::Incomplete
        );

        let mut second = first.clone();
        second.sequence = 1;
        assert!(matches!(
            store.ingest(second).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));

        let mut mismatch = first.clone();
        mismatch.resource_id = "mismatch".into();
        assert_eq!(
            store.ingest(mismatch.clone()).unwrap(),
            MeshFrameOutcome::Incomplete
        );
        mismatch.sequence = 1;
        mismatch.dtype = MeshDtype::F32LE;
        assert!(matches!(
            store.ingest(mismatch),
            Err(MeshFrameError::MetadataMismatch)
        ));

        let mut short = first;
        short.resource_id = "short".into();
        short.payload = vec![1; 3];
        let mut short_store = MeshFrameStore::new();
        assert_eq!(
            short_store.ingest(short.clone()).unwrap(),
            MeshFrameOutcome::Incomplete
        );
        let mut bad_second = short.clone();
        bad_second.sequence = 1;
        assert!(matches!(
            short_store.ingest(bad_second),
            Err(MeshFrameError::AssembledPayloadMismatch { .. })
        ));
        assert_eq!(short_store.stats().in_flight, 0);

        let mut corrected_first = short.clone();
        corrected_first.payload = vec![1; 4];
        assert_eq!(
            short_store.ingest(corrected_first).unwrap(),
            MeshFrameOutcome::Incomplete
        );
        let mut corrected_second = short;
        corrected_second.sequence = 1;
        corrected_second.payload = vec![1; 4];
        let MeshFrameOutcome::Assembled(resource) = short_store
            .ingest(corrected_second)
            .expect("a corrected retransmission must recover the same generation")
        else {
            panic!("corrected chunks should assemble");
        };
        assert_eq!(resource.payload, vec![1; 8]);
    }

    #[test]
    fn reference_counts_and_budget_pressure_are_explicit() {
        let mut store = MeshFrameStore::with_budget(8);
        assert!(matches!(
            store.ingest(tiny_frame("held", 1, 1)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.retain("held", 1));
        assert!(store.retain("held", 1));
        assert_eq!(store.stats().references, 2);
        assert!(store.release_reference("held", 1));
        assert_eq!(store.stats().references, 1);
        assert!(store.release_reference("held", 1));
        assert!(!store.release_reference("held", 1));

        assert!(matches!(
            store.ingest(tiny_frame("blocked", 1, 2)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.retain("blocked", 1));
        assert!(matches!(
            store.ingest(tiny_frame("third", 1, 3)),
            Err(MeshFrameError::ResourceTooLarge { limit: 8 })
        ));
        assert!(store.release_reference("blocked", 1));
        assert!(store.release("blocked", 1));
        assert!(matches!(
            store.ingest(tiny_frame("third", 1, 3)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(!store.retain("missing", 1));
        assert!(!store.release("missing", 1));
    }

    #[test]
    fn clear_preserves_generation_history_for_stale_handles() {
        let mut store = MeshFrameStore::new();
        let frame = fixture();
        let resource_id = frame.resource_id.clone();
        assert!(matches!(
            store.ingest(frame).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        store.clear();

        assert_eq!(
            store.ingest(fixture()).unwrap(),
            MeshFrameOutcome::DroppedStale
        );

        let mut replacement = fixture();
        replacement.generation = 2;
        assert!(matches!(
            store.ingest(replacement).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.get(&resource_id, 1).is_none());
        assert!(store.get(&resource_id, 2).is_some());
    }

    #[test]
    fn release_removes_evicted_key_from_fifo_order() {
        let mut store = MeshFrameStore::new();
        let frame = fixture();
        let resource_id = frame.resource_id.clone();
        assert!(matches!(
            store.ingest(frame).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert_eq!(store.order.len(), 1);
        assert!(store.release(&resource_id, 1));
        assert!(store.order.is_empty());
        assert_eq!(store.stats().bytes_used, 0);
    }

    #[test]
    fn referenced_resources_are_not_evicted_or_explicitly_released() {
        let mut store = MeshFrameStore::with_budget(16);
        assert!(matches!(
            store.ingest(tiny_frame("first", 1, 1)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(matches!(
            store.ingest(tiny_frame("second", 1, 2)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.retain("first", 1));
        assert_eq!(store.stats().referenced_resources, 1);
        assert_eq!(store.stats().references, 1);
        assert!(!store.release("first", 1));

        assert!(matches!(
            store.ingest(tiny_frame("third", 1, 3)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.get("first", 1).is_some());
        assert!(store.get("second", 1).is_none());
        assert!(store.release_reference("first", 1));
        assert!(store.release("first", 1));
        assert_eq!(store.stats().references, 0);
    }

    #[test]
    fn newer_generation_keeps_referenced_older_generation_until_release() {
        let mut store = MeshFrameStore::with_budget(24);
        assert!(matches!(
            store.ingest(tiny_frame("field", 1, 1)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.retain("field", 1));
        assert!(matches!(
            store.ingest(tiny_frame("field", 2, 2)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.get("field", 1).is_some());
        assert!(store.get("field", 2).is_some());
        assert!(store.release_reference("field", 1));
        assert!(store.release("field", 1));
        assert!(store.get("field", 2).is_some());
    }

    #[test]
    fn alternating_field_updates_stay_bounded_while_geometry_is_retained() {
        let mut store = MeshFrameStore::with_budget(32);
        assert!(matches!(
            store.ingest(tiny_frame("geometry", 1, 7)).unwrap(),
            MeshFrameOutcome::Assembled(_)
        ));
        assert!(store.retain("geometry", 1));

        let mut maximum_resources = 0;
        let mut maximum_bytes = 0;
        for generation in 1..=1_000 {
            assert!(matches!(
                store
                    .ingest(tiny_frame("field", generation, (generation % 251) as u8))
                    .unwrap(),
                MeshFrameOutcome::Assembled(_)
            ));
            let stats = store.stats();
            maximum_resources = maximum_resources.max(stats.resources);
            maximum_bytes = maximum_bytes.max(stats.bytes_used);
            assert!(stats.resources <= 2);
            assert!(stats.bytes_used <= 32);
            assert_eq!(stats.referenced_resources, 1);
            assert_eq!(stats.references, 1);
            assert!(store.get("geometry", 1).is_some());
            assert!(store.get("field", generation).is_some());
        }

        assert_eq!(maximum_resources, 2);
        assert!(maximum_bytes <= 32);
        assert!(store.release_reference("geometry", 1));
        assert!(store.release("geometry", 1));
        assert!(store.release("field", 1_000));
        let stats = store.stats();
        assert_eq!(stats.resources, 0);
        assert_eq!(stats.bytes_used, 0);
        assert_eq!(stats.references, 0);
    }

    #[cfg(feature = "showcase")]
    #[test]
    fn complete_mesh_and_field_objects_are_reused_by_identity() {
        use d3rs::mesh::ScalarAssociation;
        use std::cell::Cell;

        let store = MeshFrameStore::new();
        let mesh_builds = Cell::new(0);
        let make_mesh = || {
            mesh_builds.set(mesh_builds.get() + 1);
            Ok(TriangleMesh {
                id: "mesh".into(),
                positions: Arc::from([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]),
                triangles: Arc::from([[0, 1, 2]]),
                vertex_ids: None,
                cell_ids: None,
            })
        };
        let first = store.cached_triangle_mesh(7, make_mesh).unwrap();
        let second = store.cached_triangle_mesh(7, make_mesh).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(mesh_builds.get(), 1);

        let field_builds = Cell::new(0);
        let make_field = || {
            field_builds.set(field_builds.get() + 1);
            Ok(ScalarField {
                id: "field".into(),
                label: "Field".into(),
                unit: None,
                values: Arc::from([1.0, 2.0, 3.0]),
                association: ScalarAssociation::Vertex,
                valid: None,
            })
        };
        let first = store.cached_scalar_field(11, make_field).unwrap();
        let second = store.cached_scalar_field(11, make_field).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(field_builds.get(), 1);
    }
}
