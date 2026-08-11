//! Bounded binary transport for Python mesh resources.
//!
//! Mesh headers stay JSON so they share the session protocol's diagnostics,
//! while array bytes are written verbatim after the header line.  A resource
//! may be split into several frames; the store validates the complete shape
//! only after all chunks have arrived.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        if let Some(message_type) = header.message_type
            && message_type != "mesh_frame"
        {
            return Err(MeshFrameError::UnexpectedType {
                received: message_type,
            });
        }
        if header.byte_length != payload.len() {
            return Err(MeshFrameError::HeaderPayloadMismatch {
                declared: header.byte_length,
                received: payload.len(),
            });
        }
        let frame = Self {
            resource_id: header.resource_id,
            generation: header.generation,
            sequence: header.sequence,
            chunk_count: header.chunk_count,
            kind: header.kind,
            dtype: header.dtype,
            shape: header.shape,
            payload: payload.to_vec(),
        };
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

/// Reassembles mesh chunks and retains completed resources under a bounded,
/// deterministic FIFO eviction policy.
#[derive(Debug)]
pub struct MeshFrameStore {
    entries: HashMap<MeshKey, MeshEntry>,
    order: VecDeque<MeshKey>,
    latest_generation: HashMap<String, u64>,
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
        }
    }

    pub fn ingest(&mut self, frame: MeshFrame) -> Result<MeshFrameOutcome, MeshFrameError> {
        frame.validate()?;
        let expected = frame.expected_bytes()?;
        let key = (frame.resource_id.clone(), frame.generation);
        if let Some(current) = self.latest_generation.get(&frame.resource_id).copied() {
            if frame.generation < current {
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
        self.ensure_capacity(additional, &key)?;

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
        let Some(entry) = self.entries.get_mut(&key) else {
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
        if assembly.bytes != expected {
            return Err(MeshFrameError::AssembledPayloadMismatch {
                received: assembly.bytes,
                expected,
            });
        }
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
        self.remove_entry(&key).is_some()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.latest_generation.clear();
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
            let Some(key) = self.order.pop_front() else {
                return Err(MeshFrameError::ResourceTooLarge {
                    limit: self.byte_budget,
                });
            };
            if &key == current {
                self.order.push_front(key);
                return Err(MeshFrameError::ResourceTooLarge {
                    limit: self.byte_budget,
                });
            }
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
            .filter(|(id, generation)| id == resource_id && *generation < keep_generation)
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            self.remove_entry(&key);
        }
    }

    fn remove_entry(&mut self, key: &MeshKey) -> Option<MeshEntry> {
        let entry = self.entries.remove(key)?;
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
}
