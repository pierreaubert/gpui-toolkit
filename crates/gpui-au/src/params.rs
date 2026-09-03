//! Minimal host parameter bridge for the AU platform backend.
//!
//! A full `AUParameterTree`/`AUPreset`/`fullState` implementation needs
//! `AudioToolbox` observer tokens and preset plists on the Swift side. This
//! module is the Rust half of that bridge:
//!
//! - [`AuParameter`] — one realtime-safe float parameter. The value lives in
//!   an atomic (f32 bits), so the audio thread can `get`/`set` without
//!   taking a lock.
//! - [`AuParameterTree`] — an owned set of parameters with observer fan-out,
//!   exposed to Swift through the `gpui_au_parameter_*` FFI functions.
//! - [`AuFullState`] — a versioned little-endian byte encoding of an
//!   id/value snapshot (the `fullState` analogue) with strict decoding.
//!
//! `AuWindow` owns a tree pre-populated with
//! [`AuParameterTree::with_default_plugin_params`] (gain + bypass
//! placeholders); hosts extend it with [`AuParameterTree::add_parameter`].

use parking_lot::Mutex;
use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Magic bytes `"AUP1"` identifying an [`AuFullState`] payload.
pub const AU_STATE_MAGIC: u32 = 0x31505541;
/// Current [`AuFullState`] encoding version.
pub const AU_STATE_VERSION: u32 = 1;
/// Upper bound on decoded entry counts; rejects corrupt/oversized buffers.
pub const AU_STATE_MAX_ENTRIES: u32 = 4096;

/// Errors from parameter registration and lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuParamError {
    /// No parameter with this id is registered.
    UnknownParameter(u32),
    /// A parameter with this id is already registered.
    DuplicateParameter(u32),
    /// `min > max`, or a bound/default is not finite.
    InvalidRange { id: u32 },
}

impl fmt::Display for AuParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuParamError::UnknownParameter(id) => write!(f, "unknown AU parameter id {id}"),
            AuParamError::DuplicateParameter(id) => {
                write!(f, "duplicate AU parameter id {id}")
            }
            AuParamError::InvalidRange { id } => {
                write!(f, "invalid range for AU parameter id {id}")
            }
        }
    }
}

impl std::error::Error for AuParamError {}

/// Errors from [`AuFullState::decode`] and [`AuParameterTree::restore_state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuStateError {
    /// Nothing to decode.
    Empty,
    /// Wrong magic bytes (not an AU state payload).
    BadMagic { expected: u32, found: u32 },
    /// Payload was written by a newer, incompatible encoder.
    UnsupportedVersion { version: u32 },
    /// Buffer ends mid-record.
    Truncated { needed: usize, available: usize },
    /// Entry count exceeds [`AU_STATE_MAX_ENTRIES`].
    TooManyEntries { count: u32 },
}

impl fmt::Display for AuStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuStateError::Empty => write!(f, "empty AU state payload"),
            AuStateError::BadMagic { expected, found } => write!(
                f,
                "bad AU state magic {found:#x} (expected {expected:#x})"
            ),
            AuStateError::UnsupportedVersion { version } => {
                write!(f, "unsupported AU state version {version}")
            }
            AuStateError::Truncated { needed, available } => write!(
                f,
                "truncated AU state payload (needed {needed} bytes, have {available})"
            ),
            AuStateError::TooManyEntries { count } => {
                write!(f, "AU state payload has too many entries ({count})")
            }
        }
    }
}

impl std::error::Error for AuStateError {}

/// One realtime-safe float parameter.
///
/// Values are stored as f32 bits in an atomic: the audio thread's `get`/`set`
/// path never locks. Observer notification happens in [`AuParameterTree`],
/// not here.
#[derive(Debug)]
pub struct AuParameter {
    id: u32,
    name: String,
    min_value: f32,
    max_value: f32,
    default_value: f32,
    value_bits: AtomicU32,
}

impl AuParameter {
    /// Create a parameter, clamping an out-of-range default into `[min, max]`.
    pub fn new(
        id: u32,
        name: impl Into<String>,
        min_value: f32,
        max_value: f32,
        default_value: f32,
    ) -> Result<Self, AuParamError> {
        if !min_value.is_finite() || !max_value.is_finite() || !default_value.is_finite() {
            return Err(AuParamError::InvalidRange { id });
        }
        if min_value > max_value {
            return Err(AuParamError::InvalidRange { id });
        }
        let clamped = default_value.clamp(min_value, max_value);
        Ok(Self {
            id,
            name: name.into(),
            min_value,
            max_value,
            default_value: clamped,
            value_bits: AtomicU32::new(clamped.to_bits()),
        })
    }

    /// Stable numeric id used by the host (`AUParameter.address` analogue).
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Display name shown by the host.
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn min_value(&self) -> f32 {
        self.min_value
    }

    pub fn max_value(&self) -> f32 {
        self.max_value
    }

    pub fn default_value(&self) -> f32 {
        self.default_value
    }

    /// Current value (lock-free).
    pub fn get(&self) -> f32 {
        f32::from_bits(self.value_bits.load(Ordering::Relaxed))
    }

    /// Store a value, clamped to `[min, max]`. Non-finite inputs are ignored
    /// (the current value is kept). Returns the stored value.
    pub fn set(&self, value: f32) -> f32 {
        let stored = if value.is_finite() {
            value.clamp(self.min_value, self.max_value)
        } else {
            self.get()
        };
        self.value_bits.store(stored.to_bits(), Ordering::Relaxed);
        stored
    }

    /// Current value scaled to `0..=1` across `[min, max]`.
    /// A degenerate (`min == max`) parameter reports `0.0`.
    pub fn normalized(&self) -> f32 {
        let span = self.max_value - self.min_value;
        if span <= 0.0 {
            return 0.0;
        }
        ((self.get() - self.min_value) / span).clamp(0.0, 1.0)
    }

    /// Store a `0..=1` value scaled across `[min, max]`.
    /// Returns the stored (unscaled) value.
    pub fn set_normalized(&self, normalized: f32) -> f32 {
        if !normalized.is_finite() {
            return self.get();
        }
        let span = self.max_value - self.min_value;
        if span <= 0.0 {
            return self.set(self.min_value);
        }
        self.set(self.min_value + normalized.clamp(0.0, 1.0) * span)
    }
}

/// Observer callback invoked (on the setter thread) after a value changes.
type AuParamObserver = Box<dyn Fn(u32, f32) + Send + Sync + 'static>;

/// Owned set of [`AuParameter`]s with observer fan-out.
///
/// Lookup/storage is behind a short mutex; the per-parameter value itself is
/// lock-free. Hosts call `get_value` / `set_value`, whose mutex hold is
/// bounded and never blocks on the GPU.
///
/// Observer callbacks run synchronously on the thread that called `set_value`
/// and must not re-enter the tree (it is not reentrant).
pub struct AuParameterTree {
    params: Mutex<Vec<AuParameter>>,
    observers: Mutex<Vec<(u64, AuParamObserver)>>,
    next_observer: AtomicU64,
}

impl Default for AuParameterTree {
    fn default() -> Self {
        Self::new()
    }
}

impl AuParameterTree {
    /// Empty tree.
    pub fn new() -> Self {
        Self {
            params: Mutex::new(Vec::new()),
            observers: Mutex::new(Vec::new()),
            next_observer: AtomicU64::new(1),
        }
    }

    /// Tree with the standard placeholder pair every plugin starts with:
    /// id `0` (`"gain"`, `0..=1`, default `0.8`) and id `1` (`"bypass"`,
    /// `0..=1`, default `0.0`). Hosts replace or extend these with
    /// [`AuParameterTree::add_parameter`].
    pub fn with_default_plugin_params() -> Self {
        let tree = Self::new();
        // Bounds are valid by construction; a failure here is a bug.
        tree.add_parameter(0, "gain", 0.0, 1.0, 0.8)
            .expect("default gain parameter must be valid");
        tree.add_parameter(1, "bypass", 0.0, 1.0, 0.0)
            .expect("default bypass parameter must be valid");
        tree
    }

    /// Register a parameter. Fails on duplicate ids and invalid ranges.
    pub fn add_parameter(
        &self,
        id: u32,
        name: impl Into<String>,
        min_value: f32,
        max_value: f32,
        default_value: f32,
    ) -> Result<(), AuParamError> {
        if self.params.lock().iter().any(|param| param.id() == id) {
            return Err(AuParamError::DuplicateParameter(id));
        }
        let param = AuParameter::new(id, name, min_value, max_value, default_value)?;
        self.params.lock().push(param);
        Ok(())
    }

    /// Number of registered parameters.
    pub fn parameter_count(&self) -> usize {
        self.params.lock().len()
    }

    /// Current value of a parameter, or `None` for an unknown id.
    pub fn get_value(&self, id: u32) -> Option<f32> {
        self.params
            .lock()
            .iter()
            .find(|param| param.id() == id)
            .map(AuParameter::get)
    }

    /// Store a clamped value and notify observers.
    /// Returns the stored value, or [`AuParamError::UnknownParameter`].
    pub fn set_value(&self, id: u32, value: f32) -> Result<f32, AuParamError> {
        let stored = {
            let params = self.params.lock();
            let param = params
                .iter()
                .find(|param| param.id() == id)
                .ok_or(AuParamError::UnknownParameter(id))?;
            param.set(value)
        };
        for (_, observer) in self.observers.lock().iter() {
            observer(id, stored);
        }
        Ok(stored)
    }

    /// Register an observer; returns a token for [`AuParameterTree::remove_observer`].
    pub fn observe(&self, observer: impl Fn(u32, f32) + Send + Sync + 'static) -> u64 {
        let token = self.next_observer.fetch_add(1, Ordering::Relaxed);
        self.observers.lock().push((token, Box::new(observer)));
        token
    }

    /// Remove an observer by token. Returns false for an unknown token.
    pub fn remove_observer(&self, token: u64) -> bool {
        let mut observers = self.observers.lock();
        let before = observers.len();
        observers.retain(|(candidate, _)| *candidate != token);
        observers.len() != before
    }

    /// Current `(id, value)` pairs in registration order.
    pub fn snapshot(&self) -> Vec<(u32, f32)> {
        self.params
            .lock()
            .iter()
            .map(|param| (param.id(), param.get()))
            .collect()
    }

    /// Serialize the current snapshot (`fullState` analogue).
    pub fn capture_state(&self) -> Vec<u8> {
        AuFullState::encode(&self.snapshot())
    }

    /// Decode `bytes` and apply every entry whose id is registered.
    /// Unknown ids are ignored (forward compatibility); values are clamped.
    /// Returns the number of applied entries.
    pub fn restore_state(&self, bytes: &[u8]) -> Result<usize, AuStateError> {
        let entries = AuFullState::decode(bytes)?;
        let mut applied = 0;
        for (id, value) in entries {
            if self.set_value(id, value).is_ok() {
                applied += 1;
            }
        }
        Ok(applied)
    }
}

/// Versioned byte encoding of a parameter snapshot (`fullState` analogue).
///
/// Layout (all little-endian): magic `u32`, version `u32`, count `u32`,
/// then `count` × (`id u32`, `value f32`).
pub struct AuFullState;

impl AuFullState {
    /// Encode entries to bytes.
    pub fn encode(entries: &[(u32, f32)]) -> Vec<u8> {
        let mut out = Vec::with_capacity(12 + entries.len() * 8);
        out.extend_from_slice(&AU_STATE_MAGIC.to_le_bytes());
        out.extend_from_slice(&AU_STATE_VERSION.to_le_bytes());
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (id, value) in entries {
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// Decode bytes, strictly validating magic, version, and length.
    pub fn decode(bytes: &[u8]) -> Result<Vec<(u32, f32)>, AuStateError> {
        fn word(bytes: &[u8], at: &mut usize) -> Result<[u8; 4], AuStateError> {
            let end = at.checked_add(4).ok_or(AuStateError::Truncated {
                needed: usize::MAX,
                available: bytes.len(),
            })?;
            let word: [u8; 4] = bytes
                .get(*at..end)
                .and_then(|slice| slice.try_into().ok())
                .ok_or(AuStateError::Truncated {
                    needed: end,
                    available: bytes.len(),
                })?;
            *at = end;
            Ok(word)
        }

        if bytes.is_empty() {
            return Err(AuStateError::Empty);
        }
        let mut at = 0;
        let magic = u32::from_le_bytes(word(bytes, &mut at)?);
        if magic != AU_STATE_MAGIC {
            return Err(AuStateError::BadMagic {
                expected: AU_STATE_MAGIC,
                found: magic,
            });
        }
        let version = u32::from_le_bytes(word(bytes, &mut at)?);
        if version != AU_STATE_VERSION {
            return Err(AuStateError::UnsupportedVersion { version });
        }
        let count = u32::from_le_bytes(word(bytes, &mut at)?);
        if count > AU_STATE_MAX_ENTRIES {
            return Err(AuStateError::TooManyEntries { count });
        }
        let mut entries = Vec::with_capacity(count.min(256) as usize);
        for _ in 0..count {
            let id = u32::from_le_bytes(word(bytes, &mut at)?);
            let value = f32::from_le_bytes(word(bytes, &mut at)?);
            entries.push((id, value));
        }
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn default_plugin_params_cover_gain_and_bypass() {
        let tree = AuParameterTree::with_default_plugin_params();
        assert_eq!(tree.parameter_count(), 2);
        assert_eq!(tree.get_value(0), Some(0.8));
        assert_eq!(tree.get_value(1), Some(0.0));
        assert_eq!(tree.get_value(999), None);
    }

    #[test]
    fn duplicate_and_invalid_parameters_are_rejected() {
        let tree = AuParameterTree::new();
        tree.add_parameter(7, "cutoff", 20.0, 20000.0, 440.0)
            .expect("valid parameter registers");
        assert_eq!(
            tree.add_parameter(7, "dup", 0.0, 1.0, 0.5),
            Err(AuParamError::DuplicateParameter(7))
        );
        assert_eq!(
            tree.add_parameter(8, "inverted", 1.0, 0.0, 0.5),
            Err(AuParamError::InvalidRange { id: 8 })
        );
        assert_eq!(
            tree.add_parameter(9, "nan-min", f32::NAN, 1.0, 0.5),
            Err(AuParamError::InvalidRange { id: 9 })
        );
        assert_eq!(tree.parameter_count(), 1);
    }

    #[test]
    fn values_clamp_and_reject_non_finite_input() {
        let tree = AuParameterTree::new();
        tree.add_parameter(3, "mix", 0.0, 1.0, 0.5)
            .expect("valid parameter registers");
        assert_eq!(tree.set_value(3, 2.0), Ok(1.0));
        assert_eq!(tree.set_value(3, -1.0), Ok(0.0));
        // NaN/+inf keep the current value instead of poisoning the mix.
        assert_eq!(tree.set_value(3, f32::NAN), Ok(0.0));
        assert_eq!(tree.set_value(3, f32::INFINITY), Ok(0.0));
        assert_eq!(tree.get_value(3), Some(0.0));
        assert_eq!(
            tree.set_value(404, 0.5),
            Err(AuParamError::UnknownParameter(404))
        );
    }

    #[test]
    fn normalized_mapping_round_trips() {
        let param = AuParameter::new(1, "pan", -1.0, 1.0, 0.0).expect("valid");
        assert_eq!(param.set_normalized(0.75), 0.5);
        assert!((param.normalized() - 0.75).abs() < 1e-6);
        let degenerate = AuParameter::new(2, "fixed", 0.5, 0.5, 0.5).expect("valid");
        assert_eq!(degenerate.normalized(), 0.0);
        assert_eq!(degenerate.set_normalized(1.0), 0.5);
    }

    #[test]
    fn observers_fire_and_unsubscribe_by_token() {
        let tree = AuParameterTree::new();
        tree.add_parameter(5, "level", 0.0, 1.0, 0.0)
            .expect("valid parameter registers");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let capture = Arc::clone(&seen);
        let token = tree.observe(move |id, value| {
            capture.lock().push((id, value));
        });
        tree.set_value(5, 0.25).expect("known id");
        assert_eq!(*seen.lock(), vec![(5, 0.25)]);
        assert!(tree.remove_observer(token));
        assert!(!tree.remove_observer(token));
        tree.set_value(5, 0.5).expect("known id");
        assert_eq!(*seen.lock(), vec![(5, 0.25)]);
    }

    #[test]
    fn state_encode_decode_round_trips() {
        let entries = vec![(0u32, 0.8f32), (1, 0.0), (42, -3.25)];
        let bytes = AuFullState::encode(&entries);
        assert_eq!(AuFullState::decode(&bytes), Ok(entries));
        assert!(AuFullState::decode(&[]).is_err());
    }

    #[test]
    fn state_decode_rejects_corrupt_payloads() {
        let bytes = AuFullState::encode(&[(0u32, 0.5f32)]);
        let mut bad_magic = bytes.clone();
        bad_magic[0] ^= 0xff;
        assert!(matches!(
            AuFullState::decode(&bad_magic),
            Err(AuStateError::BadMagic { .. })
        ));
        let mut bad_version = bytes.clone();
        bad_version[4] = 0x7f;
        assert!(matches!(
            AuFullState::decode(&bad_version),
            Err(AuStateError::UnsupportedVersion { .. })
        ));
        assert!(matches!(
            AuFullState::decode(&bytes[..bytes.len() - 1]),
            Err(AuStateError::Truncated { .. })
        ));
        assert!(matches!(
            AuFullState::decode(&[]),
            Err(AuStateError::Empty)
        ));
        let mut too_many = AuFullState::encode(&[]);
        too_many[8..12].copy_from_slice(&(AU_STATE_MAX_ENTRIES + 1).to_le_bytes());
        assert!(matches!(
            AuFullState::decode(&too_many),
            Err(AuStateError::TooManyEntries { .. })
        ));
    }

    #[test]
    fn tree_state_restore_applies_known_ids_and_ignores_unknown() {
        let tree = AuParameterTree::with_default_plugin_params();
        tree.set_value(0, 0.1).expect("known id");
        let bytes = AuFullState::encode(&[(0u32, 0.9f32), (777u32, 0.5f32)]);
        assert_eq!(tree.restore_state(&bytes), Ok(1));
        assert_eq!(tree.get_value(0), Some(0.9));
        assert_eq!(tree.capture_state(), tree.capture_state());
        // A captured snapshot restores exactly.
        tree.set_value(0, 0.1).expect("known id");
        let snapshot = tree.capture_state();
        assert_eq!(tree.restore_state(&snapshot), Ok(2));
        assert_eq!(tree.get_value(0), Some(0.1));
    }
}
