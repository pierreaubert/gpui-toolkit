//! Density profiles + the multiplier machinery that scales them. The base
//! rows are inkform's `fine` profiles; each shipped preset (state × size)
//! applies count / radius multipliers on top. Ported from `thinking-orbs`
//! 0.3.1 `engine/profiles.ts`, MIT © Jakub Antalik.

use std::collections::HashMap;

use super::ModeKey;

/// Per-mode draw options: a flat key→number map preserving the TypeScript
/// `ModeOpts` semantics (dynamic keys, extras merged verbatim, `??` defaults,
/// 0-opt-out). Missing keys read as their per-mode default via [`ModeOpts::get`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeOpts {
    values: HashMap<&'static str, f64>,
}

impl ModeOpts {
    /// An empty option set (every read falls back to its default).
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from `(key, value)` pairs.
    pub fn from_pairs(pairs: &[(&'static str, f64)]) -> Self {
        Self {
            values: pairs.iter().copied().collect(),
        }
    }

    /// Read `key`, falling back to `default` when absent (the TS `o.k ?? d`).
    pub fn get(&self, key: &str, default: f64) -> f64 {
        self.values.get(key).copied().unwrap_or(default)
    }

    /// Insert or overwrite a key.
    pub fn set(&mut self, key: &'static str, value: f64) {
        self.values.insert(key, value);
    }

    /// Merge `other` verbatim on top of `self` (the TS `{...opts, ...extra}`).
    pub fn merge(&mut self, other: &ModeOpts) {
        self.values
            .extend(other.values.iter().map(|(k, v)| (*k, *v)));
    }

    /// Whether `key` is present (regardless of value).
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Iterate over every `(key, value)` pair.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, f64)> + '_ {
        self.values.iter().map(|(k, v)| (*k, *v))
    }

    /// Number of keys present.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether no keys are present.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

// 2-D lattices (rings × dots-per-ring) come in pairs — each side takes
// √scale so the TOTAL dot count scales by `scale`; flat lists scale
// linearly. `iconD` sets the morph outline's sampling density.
const COUNT_PAIRS: [(&str, &str); 3] = [
    ("latRings", "lonDensity"),
    ("rings", "lonDensity"),
    ("lanes", "segs"),
];
const COUNT_KEYS: [&str; 5] = ["orbitN", "ghostN", "nodeN", "strandN", "signals"];
const ICON_DENSITY_KEYS: [&str; 1] = ["iconD"];

// Every key that sets a dot's rendered radius — scaling all of them keeps
// a dot's near/far falloff intact while shrinking or growing the mark.
const RADIUS_KEYS: [&str; 9] = [
    "rBase",
    "rDepth",
    "rActive",
    "rDot",
    "ghostR",
    "partR",
    "partRDepth",
    "nodeR",
    "nodeRDepth",
];

/// Scale every dot-count key by `scale` (√-scaled for lattice pairs so the
/// total count scales linearly). A 0 value is an opt-out and is left alone.
pub fn scale_counts(opts: &ModeOpts, scale: f64) -> ModeOpts {
    let mut out = opts.clone();
    let mut done: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let rt = scale.sqrt();
    for (a, b) in COUNT_PAIRS {
        if out.contains(a) && out.contains(b) && !done.contains(a) && !done.contains(b) {
            let va = out.get(a, 0.0);
            let vb = out.get(b, 0.0);
            out.set(a, (va * rt).round().max(2.0));
            out.set(b, (vb * rt).round().max(2.0));
            done.insert(a);
            done.insert(b);
        }
    }
    for k in COUNT_KEYS {
        // 0 means the mode opted out of that layer entirely (ring has no ghost
        // sphere) — scaling must not resurrect it as a single stray dot
        if out.contains(k) && out.get(k, 0.0) != 0.0 && !done.contains(k) {
            let v = out.get(k, 0.0);
            out.set(k, (v * scale).round().max(1.0));
        }
    }
    for k in ICON_DENSITY_KEYS {
        if out.contains(k) {
            let v = out.get(k, 0.0);
            out.set(k, (v * scale).max(0.02));
        }
    }
    out
}

/// Scale every dot-radius key by `scale`, and record the multiplier itself
/// in `rSizeMul` — spacing-derived radii (the morph outline) use it, since
/// they aren't based on any single radius key.
pub fn scale_radii(opts: &ModeOpts, scale: f64) -> ModeOpts {
    let mut out = opts.clone();
    for k in RADIUS_KEYS {
        if out.contains(k) {
            let v = out.get(k, 0.0);
            out.set(k, v * scale);
        }
    }
    let prev = out.get("rSizeMul", 1.0);
    out.set("rSizeMul", prev * scale);
    out
}

/// Base (fine) profile for a mode, before preset multipliers.
pub fn base_profile(mode: ModeKey) -> ModeOpts {
    match mode {
        ModeKey::Globe => ModeOpts::from_pairs(&[
            ("latRings", 17.0),
            ("lonDensity", 44.0),
            ("rBase", 0.6),
            ("rDepth", 1.7),
            ("rBoost", 1.0),
            ("inkFar", 0.62),
            ("inkSpan", 0.54),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Orbits => ModeOpts::from_pairs(&[
            ("orbitN", 12.0),
            ("ghostN", 40.0),
            ("ghostR", 0.9),
            ("ghostA", 0.5),
            ("particles", 3.0),
            ("partR", 1.2),
            ("partRDepth", 1.6),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Rubik => ModeOpts::from_pairs(&[
            ("latRings", 15.0),
            ("lonDensity", 40.0),
            ("moveCount", 14.0),
            ("rBase", 0.6),
            ("rDepth", 1.7),
            ("rActive", 0.3),
            ("inkFar", 0.62),
            ("inkSpan", 0.54),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Wave => ModeOpts::from_pairs(&[
            ("rings", 15.0),
            ("lonDensity", 40.0),
            ("rBase", 0.6),
            ("rDepth", 1.7),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Web => ModeOpts::from_pairs(&[
            ("nodeN", 30.0),
            ("thr", 0.72),
            ("signals", 5.0),
            ("nodeR", 1.4),
            ("nodeRDepth", 1.8),
            ("lineW", 0.8),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Braid => ModeOpts::from_pairs(&[
            ("strandN", 52.0),
            ("turns", 3.0),
            ("ghostN", 150.0),
            ("rBase", 1.2),
            ("rDepth", 1.8),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Ribbon => ModeOpts::from_pairs(&[
            ("lanes", 5.0),
            ("segs", 88.0),
            ("ghostN", 150.0),
            ("rBase", 1.1),
            ("rDepth", 1.7),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        // ring shares ribbon's painter; faceOn cancels the camera tilt and
        // moves the undulation onto the radius, and there is no ghost sphere
        // behind it
        ModeKey::Ring => ModeOpts::from_pairs(&[
            ("lanes", 5.0),
            ("segs", 88.0),
            ("ghostN", 0.0),
            ("faceOn", 1.0),
            ("rBase", 1.1),
            ("rDepth", 1.7),
            ("rsPow", 0.6),
            ("rMin", 0.3),
        ]),
        ModeKey::Morph => ModeOpts::from_pairs(&[("rDot", 0.021), ("iconD", 1.0), ("rMin", 0.25)]),
    }
}
