//! Bounded host-owned audio frames for the Python session binary data plane.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub const MAX_AUDIO_FRAME_BYTES: usize = 256 * 1024;
pub const MAX_AUDIO_STREAMS: usize = 64;
pub const MAX_METER_CHANNELS: usize = 128;
pub const MAX_SPECTRUM_BINS: usize = 32 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioFrameKind {
    Meter,
    Spectrum,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioFrame {
    pub resource_id: String,
    pub generation: u64,
    pub sequence: u64,
    pub frame_kind: AudioFrameKind,
    pub byte_length: usize,
    pub shape: Vec<usize>,
    #[serde(default = "default_dtype")]
    pub dtype: String,
    #[serde(default = "default_byte_order")]
    pub byte_order: String,
    #[serde(default = "default_finite_policy")]
    pub finite_policy: String,
    #[serde(default = "default_coalesce_policy")]
    pub coalesce: String,
    pub sample_rate: f64,
    #[serde(default)]
    pub attack_ms: Option<f64>,
    #[serde(default)]
    pub release_ms: Option<f64>,
    #[serde(default)]
    pub minimum_frequency: Option<f64>,
    #[serde(default)]
    pub maximum_frequency: Option<f64>,
    /// Populated by the framed stdout reader, never encoded in the JSON header.
    #[serde(skip)]
    pub payload: Vec<u8>,
}

fn default_dtype() -> String {
    "f32".into()
}
fn default_byte_order() -> String {
    "little".into()
}
fn default_finite_policy() -> String {
    "drop_frame".into()
}
fn default_coalesce_policy() -> String {
    "latest".into()
}

#[derive(Debug, Clone, PartialEq)]
pub struct RetainedAudioFrame {
    pub generation: u64,
    pub sequence: u64,
    pub frame_kind: AudioFrameKind,
    pub shape: Vec<usize>,
    pub sample_rate: f64,
    pub attack_ms: Option<f64>,
    pub release_ms: Option<f64>,
    pub minimum_frequency: Option<f64>,
    pub maximum_frequency: Option<f64>,
    pub values: Vec<f32>,
}

impl RetainedAudioFrame {
    pub fn meter_levels(&self) -> Option<&[f32]> {
        let channels = *self.shape.first()?;
        (self.frame_kind == AudioFrameKind::Meter).then(|| &self.values[..channels])
    }

    pub fn meter_peaks(&self) -> Option<&[f32]> {
        let channels = *self.shape.first()?;
        (self.frame_kind == AudioFrameKind::Meter && self.shape.get(1) == Some(&2))
            .then(|| &self.values[channels..channels * 2])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFrameOutcome {
    Stored,
    Coalesced,
    DroppedNonFinite,
    DroppedStale,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioFrameStats {
    pub streams: usize,
    pub bytes_used: usize,
    pub coalesced: u64,
    pub dropped_non_finite: u64,
    pub dropped_stale: u64,
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AudioFrameError {
    #[error("audio resource id is empty or too long")]
    InvalidId,
    #[error("audio frame metadata is invalid")]
    InvalidMetadata,
    #[error("audio frame payload has {received} bytes; expected {expected}")]
    InvalidPayload { received: usize, expected: usize },
    #[error("audio frame exceeds the {limit}-byte limit")]
    TooLarge { limit: usize },
    #[error("audio stream limit of {limit} is exhausted")]
    StreamLimit { limit: usize },
}

#[derive(Debug, Default)]
pub struct AudioFrameStore {
    frames: HashMap<String, RetainedAudioFrame>,
    stats: AudioFrameStats,
}

impl AudioFrameStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, id: &str) -> Option<&RetainedAudioFrame> {
        self.frames.get(id)
    }

    pub fn stats(&self) -> AudioFrameStats {
        AudioFrameStats {
            streams: self.frames.len(),
            ..self.stats
        }
    }

    pub fn ingest(&mut self, frame: AudioFrame) -> Result<AudioFrameOutcome, AudioFrameError> {
        validate_header(&frame)?;
        if frame.payload.len() != frame.byte_length {
            return Err(AudioFrameError::InvalidPayload {
                received: frame.payload.len(),
                expected: frame.byte_length,
            });
        }
        if let Some(current) = self.frames.get(&frame.resource_id) {
            if frame.generation < current.generation
                || (frame.generation == current.generation && frame.sequence <= current.sequence)
            {
                self.stats.dropped_stale += 1;
                return Ok(AudioFrameOutcome::DroppedStale);
            }
        } else if self.frames.len() >= MAX_AUDIO_STREAMS {
            return Err(AudioFrameError::StreamLimit {
                limit: MAX_AUDIO_STREAMS,
            });
        }

        let values = frame
            .payload
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            .collect::<Vec<_>>();
        if values.iter().any(|value| !value.is_finite()) {
            self.stats.dropped_non_finite += 1;
            return Ok(AudioFrameOutcome::DroppedNonFinite);
        }

        let retained = RetainedAudioFrame {
            generation: frame.generation,
            sequence: frame.sequence,
            frame_kind: frame.frame_kind,
            shape: frame.shape,
            sample_rate: frame.sample_rate,
            attack_ms: frame.attack_ms,
            release_ms: frame.release_ms,
            minimum_frequency: frame.minimum_frequency,
            maximum_frequency: frame.maximum_frequency,
            values,
        };
        let old = self.frames.insert(frame.resource_id, retained);
        if let Some(ref old) = old {
            self.stats.bytes_used = self.stats.bytes_used.saturating_sub(old.values.len() * 4);
            self.stats.coalesced += 1;
        }
        self.stats.bytes_used += frame.byte_length;
        Ok(if old.is_some() {
            AudioFrameOutcome::Coalesced
        } else {
            AudioFrameOutcome::Stored
        })
    }

    pub fn release(&mut self, id: &str, generation: u64) -> bool {
        if self
            .frames
            .get(id)
            .is_some_and(|frame| frame.generation == generation)
        {
            if let Some(frame) = self.frames.remove(id) {
                self.stats.bytes_used =
                    self.stats.bytes_used.saturating_sub(frame.values.len() * 4);
            }
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
        self.stats.bytes_used = 0;
    }
}

fn validate_header(frame: &AudioFrame) -> Result<(), AudioFrameError> {
    if frame.resource_id.trim().is_empty() || frame.resource_id.len() > 128 {
        return Err(AudioFrameError::InvalidId);
    }
    if frame.byte_length > MAX_AUDIO_FRAME_BYTES {
        return Err(AudioFrameError::TooLarge {
            limit: MAX_AUDIO_FRAME_BYTES,
        });
    }
    if frame.generation == 0
        || frame.sequence == 0
        || !frame.byte_length.is_multiple_of(4)
        || frame.dtype != "f32"
        || frame.byte_order != "little"
        || frame.finite_policy != "drop_frame"
        || frame.coalesce != "latest"
        || !frame.sample_rate.is_finite()
        || frame.sample_rate <= 0.0
        || frame
            .attack_ms
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        || frame
            .release_ms
            .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(AudioFrameError::InvalidMetadata);
    }
    let value_count = frame.byte_length / 4;
    let valid_shape = match frame.frame_kind {
        AudioFrameKind::Meter => {
            matches!(frame.shape.as_slice(), [channels, planes]
                if (1..=MAX_METER_CHANNELS).contains(channels)
                    && (*planes == 1 || *planes == 2)
                    && channels * planes == value_count)
        }
        AudioFrameKind::Spectrum => {
            matches!(frame.shape.as_slice(), [bins]
                if (1..=MAX_SPECTRUM_BINS).contains(bins) && *bins == value_count)
                && frame
                    .minimum_frequency
                    .is_some_and(|value| value.is_finite() && value > 0.0)
                && frame.maximum_frequency.is_some_and(|value| {
                    value.is_finite() && value > frame.minimum_frequency.unwrap_or(0.0)
                })
        }
    };
    if !valid_shape {
        return Err(AudioFrameError::InvalidMetadata);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meter(sequence: u64, values: &[f32]) -> AudioFrame {
        AudioFrame {
            resource_id: "main-meter".into(),
            generation: 1,
            sequence,
            frame_kind: AudioFrameKind::Meter,
            byte_length: values.len() * 4,
            shape: vec![values.len(), 1],
            dtype: "f32".into(),
            byte_order: "little".into(),
            finite_policy: "drop_frame".into(),
            coalesce: "latest".into(),
            sample_rate: 48_000.0,
            attack_ms: Some(10.0),
            release_ms: Some(120.0),
            minimum_frequency: None,
            maximum_frequency: None,
            payload: values
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect(),
        }
    }

    #[test]
    fn retains_only_latest_finite_audio_frame() {
        let mut store = AudioFrameStore::new();
        assert_eq!(
            store.ingest(meter(1, &[-12.0, -6.0])).unwrap(),
            AudioFrameOutcome::Stored
        );
        assert_eq!(
            store.ingest(meter(2, &[-10.0, -4.0])).unwrap(),
            AudioFrameOutcome::Coalesced
        );
        assert_eq!(
            store.ingest(meter(1, &[-20.0, -20.0])).unwrap(),
            AudioFrameOutcome::DroppedStale
        );
        assert_eq!(
            store.get("main-meter").unwrap().meter_levels().unwrap(),
            &[-10.0, -4.0]
        );
        assert_eq!(store.stats().coalesced, 1);
        assert_eq!(store.stats().dropped_stale, 1);
    }

    #[test]
    fn drops_non_finite_frames_and_releases_generation_safely() {
        let mut store = AudioFrameStore::new();
        assert_eq!(
            store.ingest(meter(1, &[f32::NAN])).unwrap(),
            AudioFrameOutcome::DroppedNonFinite
        );
        assert!(store.get("main-meter").is_none());
        store.ingest(meter(2, &[-3.0])).unwrap();
        assert!(!store.release("main-meter", 2));
        assert!(store.release("main-meter", 1));
        assert_eq!(store.stats().bytes_used, 0);
    }
}
