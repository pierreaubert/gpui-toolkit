//! Bounded binary transport for Python mesh resources.
//!
//! Mesh headers stay JSON so they share the session protocol's diagnostics,
//! while array bytes are written verbatim after the header line.  A resource
//! may be split into several frames; the store validates the complete shape
//! only after all chunks have arrived.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
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
}
