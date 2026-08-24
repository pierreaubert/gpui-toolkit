//! Pure-math geometry engine for the thinking orbs: mode dispatch plus the
//! shared primitives. Ported from `thinking-orbs` 0.3.1, MIT © Jakub Antalik.

pub mod braid;
pub mod core;
pub mod lattice;
pub mod morph;
pub mod orbits;
pub mod profiles;
pub mod ribbon;
pub mod web;

pub use core::{Dot, Line, OrbFrame};
pub use profiles::ModeOpts;

use std::str::FromStr;

/// The nine animation modes, keyed as in the TypeScript `ModeKey` union.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ModeKey {
    /// Particles on tilted orbits (the "working" state).
    #[default]
    Orbits,
    /// A scan meridian sweeps a dotted globe ("searching").
    Globe,
    /// Bands scramble in quarter turns, then click back ("solving").
    Rubik,
    /// A waveform rolls through latitude rings ("listening").
    Wave,
    /// A constellation wires itself ("connecting").
    Web,
    /// Three strands plait around the sphere ("weaving").
    Braid,
    /// An undulating multi-band sash ("composing").
    Ribbon,
    /// A face-on ring slowly morphing ("breathing"); shares ribbon's geometry.
    Ring,
    /// A dotted outline morphs circle → triangle → square ("shaping").
    Morph,
}

impl ModeKey {
    /// The lowercase TypeScript mode key (e.g. `"orbits"`).
    pub fn as_str(self) -> &'static str {
        match self {
            ModeKey::Orbits => "orbits",
            ModeKey::Globe => "globe",
            ModeKey::Rubik => "rubik",
            ModeKey::Wave => "wave",
            ModeKey::Web => "web",
            ModeKey::Braid => "braid",
            ModeKey::Ribbon => "ribbon",
            ModeKey::Ring => "ring",
            ModeKey::Morph => "morph",
        }
    }

    /// All nine modes, in declaration order.
    pub const ALL: [ModeKey; 9] = [
        ModeKey::Orbits,
        ModeKey::Globe,
        ModeKey::Rubik,
        ModeKey::Wave,
        ModeKey::Web,
        ModeKey::Braid,
        ModeKey::Ribbon,
        ModeKey::Ring,
        ModeKey::Morph,
    ];
}

impl std::fmt::Display for ModeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModeKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "orbits" => ModeKey::Orbits,
            "globe" => ModeKey::Globe,
            "rubik" => ModeKey::Rubik,
            "wave" => ModeKey::Wave,
            "web" => ModeKey::Web,
            "braid" => ModeKey::Braid,
            "ribbon" => ModeKey::Ribbon,
            "ring" => ModeKey::Ring,
            "morph" => ModeKey::Morph,
            _ => return Err(()),
        })
    }
}

/// Geometry for one instant: pure math over `(size, t, opts)`.
///
/// `size` is the frame size in pixels, `t` the animation clock in seconds
/// (the preset `speed` multiplier is applied by the caller, not here), and
/// `opts` the fully-resolved mode options. Mirrors the TypeScript
/// `MODE_FRAMES` dispatch; `Ring` shares ribbon's frame function, switched by
/// the `faceOn` profile flag.
pub fn frame(mode: ModeKey, size: f64, t: f64, opts: &ModeOpts) -> OrbFrame {
    match mode {
        ModeKey::Orbits => orbits::frame_orbits(size, t, opts),
        ModeKey::Globe => lattice::frame_globe(size, t, opts),
        ModeKey::Rubik => lattice::frame_rubik(size, t, opts),
        ModeKey::Wave => lattice::frame_wave(size, t, opts),
        ModeKey::Web => web::frame_web(size, t, opts),
        ModeKey::Braid => braid::frame_braid(size, t, opts),
        ModeKey::Ribbon | ModeKey::Ring => ribbon::frame_ribbon(size, t, opts),
        ModeKey::Morph => morph::frame_morph(size, t, opts),
    }
}
