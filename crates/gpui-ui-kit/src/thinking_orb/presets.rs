//! The shipped tunings: nine states × two sizes, baked from the inkform
//! mini-page tuning session. `count`/`size` are multipliers over the base
//! fine profiles; `speed` multiplies the shared clock. Ported from
//! `thinking-orbs` 0.3.1 `presets.ts`, MIT © Jakub Antalik.

use std::str::FromStr;

use super::engine::ModeKey;
use super::engine::profiles::{ModeOpts, base_profile, scale_counts, scale_radii};

/// The nine shipped states — each a hand-tuned animation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OrbState {
    /// Particles on tilted orbits.
    #[default]
    Working,
    /// A scan meridian sweeps a dotted globe.
    Searching,
    /// Bands scramble in quarter turns, then click back.
    Solving,
    /// A waveform rolls through latitude rings.
    Listening,
    /// A constellation wires itself, packets running the edges.
    Connecting,
    /// Three strands plait around the sphere.
    Weaving,
    /// An undulating multi-band sash.
    Composing,
    /// A face-on ring slowly morphing.
    Breathing,
    /// A dotted outline morphs circle → triangle → square.
    Shaping,
}

impl OrbState {
    /// The lowercase TypeScript state key (e.g. `"working"`).
    pub fn as_str(self) -> &'static str {
        match self {
            OrbState::Working => "working",
            OrbState::Searching => "searching",
            OrbState::Solving => "solving",
            OrbState::Listening => "listening",
            OrbState::Connecting => "connecting",
            OrbState::Weaving => "weaving",
            OrbState::Composing => "composing",
            OrbState::Breathing => "breathing",
            OrbState::Shaping => "shaping",
        }
    }

    /// English per-state accessibility label (from the upstream
    /// `spec/orbs-spec.json` labels table).
    pub fn label(self) -> &'static str {
        match self {
            OrbState::Working => "Working…",
            OrbState::Searching => "Searching…",
            OrbState::Solving => "Solving…",
            OrbState::Listening => "Listening…",
            OrbState::Connecting => "Connecting…",
            OrbState::Weaving => "Weaving…",
            OrbState::Composing => "Composing…",
            OrbState::Breathing => "Thinking…",
            OrbState::Shaping => "Shaping…",
        }
    }

    /// The animation mode this state maps to (`STATE_TO_MODE`).
    pub fn mode(self) -> ModeKey {
        match self {
            OrbState::Working => ModeKey::Orbits,
            OrbState::Searching => ModeKey::Globe,
            OrbState::Solving => ModeKey::Rubik,
            OrbState::Listening => ModeKey::Wave,
            OrbState::Connecting => ModeKey::Web,
            OrbState::Weaving => ModeKey::Braid,
            OrbState::Composing => ModeKey::Ribbon,
            OrbState::Breathing => ModeKey::Ring,
            OrbState::Shaping => ModeKey::Morph,
        }
    }

    /// All nine states, in declaration order.
    pub const ALL: [OrbState; 9] = [
        OrbState::Working,
        OrbState::Searching,
        OrbState::Solving,
        OrbState::Listening,
        OrbState::Connecting,
        OrbState::Weaving,
        OrbState::Composing,
        OrbState::Breathing,
        OrbState::Shaping,
    ];
}

impl std::fmt::Display for OrbState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OrbState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "working" => OrbState::Working,
            "searching" => OrbState::Searching,
            "solving" => OrbState::Solving,
            "listening" => OrbState::Listening,
            "connecting" => OrbState::Connecting,
            "weaving" => OrbState::Weaving,
            "composing" => OrbState::Composing,
            "breathing" => OrbState::Breathing,
            "shaping" => OrbState::Shaping,
            _ => return Err(()),
        })
    }
}

/// Rendered size in pixels. Exactly two tuned presets ship: 64 (chat-avatar
/// scale) and 20 (inline-text scale). Each size carries its own dot count,
/// dot size and speed tuning — they are separate designs, not a scale factor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OrbSize {
    /// 64 px, chat-avatar scale.
    #[default]
    Px64,
    /// 20 px, inline-text scale.
    Px20,
}

impl OrbSize {
    /// The frame size in pixels (64.0 or 20.0).
    pub fn pixels(self) -> f64 {
        match self {
            OrbSize::Px64 => 64.0,
            OrbSize::Px20 => 20.0,
        }
    }

    /// The TypeScript size key (`"64"` or `"20"`).
    pub fn as_str(self) -> &'static str {
        match self {
            OrbSize::Px64 => "64",
            OrbSize::Px20 => "20",
        }
    }

    /// Both shipped sizes.
    pub const ALL: [OrbSize; 2] = [OrbSize::Px64, OrbSize::Px20];
}

impl std::fmt::Display for OrbSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for OrbSize {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "64" => OrbSize::Px64,
            "20" => OrbSize::Px20,
            _ => return Err(()),
        })
    }
}

/// One shipped tuning: multipliers over the base fine profile.
#[derive(Clone, Debug)]
pub struct Preset {
    /// Multiplies the shared animation clock.
    pub speed: f64,
    /// Dot-count multiplier over the base profile.
    pub count: f64,
    /// Dot-radius multiplier over the base profile.
    pub size: f64,
    /// Extra mode opts merged verbatim after scaling.
    pub extra: Option<ModeOpts>,
}

/// The shipped `PRESETS` table: per mode, per size.
pub fn preset(mode: ModeKey, size: OrbSize) -> Preset {
    match (mode, size) {
        (ModeKey::Orbits, OrbSize::Px64) => Preset {
            speed: 1.885,
            count: 1.0,
            size: 1.0,
            extra: None,
        },
        (ModeKey::Orbits, OrbSize::Px20) => Preset {
            speed: 3.9,
            count: 0.238,
            size: 2.4,
            extra: None,
        },
        (ModeKey::Globe, OrbSize::Px64) => Preset {
            speed: 2.015,
            count: 0.42,
            size: 1.15,
            extra: Some(ModeOpts::from_pairs(&[
                ("scanMul", 4.08),
                ("dimBase", 0.45),
            ])),
        },
        (ModeKey::Globe, OrbSize::Px20) => Preset {
            speed: 2.665,
            count: 0.105,
            size: 1.75,
            extra: Some(ModeOpts::from_pairs(&[
                ("scanMul", 4.335),
                ("dimBase", 0.45),
            ])),
        },
        (ModeKey::Rubik, OrbSize::Px64) => Preset {
            speed: 1.82,
            count: 0.35,
            size: 1.05,
            extra: None,
        },
        (ModeKey::Rubik, OrbSize::Px20) => Preset {
            speed: 1.95,
            count: 0.088,
            size: 1.9,
            extra: None,
        },
        (ModeKey::Wave, OrbSize::Px64) => Preset {
            speed: 4.388,
            count: 0.341,
            size: 1.0,
            extra: None,
        },
        (ModeKey::Wave, OrbSize::Px20) => Preset {
            speed: 3.998,
            count: 0.105,
            size: 1.6,
            extra: None,
        },
        (ModeKey::Web, OrbSize::Px64) => Preset {
            speed: 3.315,
            count: 1.35,
            size: 0.95,
            extra: None,
        },
        (ModeKey::Web, OrbSize::Px20) => Preset {
            speed: 6.63,
            count: 0.25,
            size: 1.52,
            extra: None,
        },
        (ModeKey::Braid, OrbSize::Px64) => Preset {
            speed: 1.625,
            count: 0.5,
            size: 1.0,
            extra: None,
        },
        (ModeKey::Braid, OrbSize::Px20) => Preset {
            speed: 2.75,
            count: 0.1125,
            size: 1.36,
            extra: None,
        },
        (ModeKey::Ribbon, OrbSize::Px64) => Preset {
            speed: 2.34,
            count: 0.25,
            size: 0.85,
            extra: Some(ModeOpts::from_pairs(&[
                ("spin", 0.0),
                ("bandMul", 3.9),
                ("wobMul", 1.0),
            ])),
        },
        (ModeKey::Ribbon, OrbSize::Px20) => Preset {
            speed: 3.12,
            count: 0.051,
            size: 1.073,
            extra: Some(ModeOpts::from_pairs(&[
                ("spin", 0.0),
                ("bandMul", 4.94),
                ("wobMul", 1.0),
            ])),
        },
        (ModeKey::Ring, OrbSize::Px64) => Preset {
            speed: 3.24,
            count: 0.25,
            size: 0.956,
            extra: Some(ModeOpts::from_pairs(&[
                ("spin", 0.0),
                ("bandMul", 3.627),
                ("wobMul", 0.368),
            ])),
        },
        (ModeKey::Ring, OrbSize::Px20) => Preset {
            speed: 3.78,
            count: 0.028,
            size: 1.622,
            extra: Some(ModeOpts::from_pairs(&[
                ("spin", 0.0),
                ("bandMul", 3.968),
                ("wobMul", 0.565),
            ])),
        },
        (ModeKey::Morph, OrbSize::Px64) => Preset {
            speed: 2.405,
            count: 0.702,
            size: 0.395,
            extra: Some(ModeOpts::from_pairs(&[("spread", 1.45)])),
        },
        (ModeKey::Morph, OrbSize::Px20) => Preset {
            speed: 2.08,
            count: 0.53,
            size: 1.011,
            extra: Some(ModeOpts::from_pairs(&[("spread", 1.45)])),
        },
    }
}

/// A `(state, size)` pair resolved to its mode + fully-scaled draw options.
#[derive(Clone, Debug)]
pub struct Resolved {
    /// The animation mode to run.
    pub mode: ModeKey,
    /// Clock multiplier (applied by the renderer, not the engine).
    pub speed: f64,
    /// Fully-resolved mode options.
    pub opts: ModeOpts,
}

/// Resolve a `(state, size)` pair to its mode + fully-scaled draw options.
pub fn resolve_preset(state: OrbState, size: OrbSize) -> Resolved {
    let mode = state.mode();
    let p = preset(mode, size);
    let mut opts = base_profile(mode);
    if p.count != 1.0 {
        opts = scale_counts(&opts, p.count);
    }
    if p.size != 1.0 {
        opts = scale_radii(&opts, p.size);
    }
    if let Some(extra) = &p.extra {
        opts.merge(extra);
    }
    Resolved {
        mode,
        speed: p.speed,
        opts,
    }
}
