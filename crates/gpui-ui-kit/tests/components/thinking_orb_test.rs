//! ThinkingOrb API/behavior unit tests.
//!
//! Covers the non-entity surface of `gpui_ui_kit::thinking_orb`: the state and
//! size enums, preset resolution, density-scaling invariants, and an engine
//! smoke test per mode. Golden-vector parity against the TypeScript engine
//! lives in `thinking_orb_parity_test.rs`; entity creation and rendering need
//! a GPUI context and live in `tests/integration/thinking_orb_test.rs`.

use std::time::Duration;

use gpui_ui_kit::FrameStats;
use gpui_ui_kit::thinking_orb::engine::profiles::{scale_counts, scale_radii};
use gpui_ui_kit::thinking_orb::engine::{self, ModeKey};
use gpui_ui_kit::thinking_orb::presets::{OrbSize, OrbState, resolve_preset};

// ============================================================================
// OrbState
// ============================================================================

#[test]
fn test_orb_state_all_variants_covered() {
    assert_eq!(OrbState::ALL.len(), 9, "expected exactly 9 states");
    for state in OrbState::ALL {
        assert!(!state.as_str().is_empty(), "{state:?} has empty key");
        assert!(!state.label().is_empty(), "{state:?} has empty label");
    }
}

#[test]
fn test_orb_state_keys_and_labels_distinct() {
    let keys: Vec<&str> = OrbState::ALL.iter().map(|s| s.as_str()).collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        keys.len(),
        "duplicate as_str() keys: {keys:?}"
    );

    let labels: Vec<&str> = OrbState::ALL.iter().map(|s| s.label()).collect();
    let mut sorted_labels = labels.clone();
    sorted_labels.sort_unstable();
    sorted_labels.dedup();
    assert_eq!(
        sorted_labels.len(),
        labels.len(),
        "duplicate labels: {labels:?}"
    );
}

#[test]
fn test_orb_state_mode_mapping() {
    let expected = [
        (OrbState::Working, ModeKey::Orbits),
        (OrbState::Searching, ModeKey::Globe),
        (OrbState::Solving, ModeKey::Rubik),
        (OrbState::Listening, ModeKey::Wave),
        (OrbState::Connecting, ModeKey::Web),
        (OrbState::Weaving, ModeKey::Braid),
        (OrbState::Composing, ModeKey::Ribbon),
        (OrbState::Breathing, ModeKey::Ring),
        (OrbState::Shaping, ModeKey::Morph),
    ];
    for (state, mode) in expected {
        assert_eq!(state.mode(), mode, "mode mismatch for {state:?}");
    }
    // The mapping must be a bijection onto the nine modes.
    let mut modes: Vec<ModeKey> = OrbState::ALL.iter().map(|s| s.mode()).collect();
    modes.sort_by_key(|m| m.as_str());
    modes.dedup();
    assert_eq!(modes.len(), ModeKey::ALL.len());
}

// ============================================================================
// resolve_preset — all 18 (state × OrbSize) combos
// ============================================================================

#[test]
fn test_resolve_preset_all_combos() {
    for state in OrbState::ALL {
        for size in OrbSize::ALL {
            let resolved = resolve_preset(state, size);
            assert_eq!(
                resolved.mode,
                state.mode(),
                "mode mismatch for {state:?} {size:?}"
            );
            assert!(
                resolved.speed > 0.0,
                "non-positive speed for {state:?} {size:?}: {}",
                resolved.speed
            );
            assert!(
                !resolved.opts.is_empty(),
                "empty opts for {state:?} {size:?}"
            );
        }
    }
}

// ============================================================================
// Density scaling invariants
// ============================================================================

/// Total dot count as the engine would see it: √-scaled lattice pairs
/// multiply, flat count keys add.
fn total_count(opts: &gpui_ui_kit::thinking_orb::engine::profiles::ModeOpts) -> f64 {
    let mut total = 0.0;
    for (a, b) in [
        ("latRings", "lonDensity"),
        ("rings", "lonDensity"),
        ("lanes", "segs"),
    ] {
        if opts.contains(a) && opts.contains(b) {
            total += opts.get(a, 0.0) * opts.get(b, 0.0);
        }
    }
    for k in ["orbitN", "ghostN", "nodeN", "strandN", "signals"] {
        if opts.contains(k) {
            total += opts.get(k, 0.0);
        }
    }
    total
}

#[test]
fn test_scale_counts_zero_opt_out_not_resurrected() {
    // Ring's base profile has ghostN = 0.0 (no ghost sphere): scaling must
    // leave it at 0 rather than rounding up to a single stray dot.
    let opts = engine::profiles::ModeOpts::from_pairs(&[("ghostN", 0.0), ("orbitN", 12.0)]);
    for scale in [0.5, 2.0, 8.0] {
        let scaled = scale_counts(&opts, scale);
        assert_eq!(
            scaled.get("ghostN", f64::NAN),
            0.0,
            "0-opt-out key resurrected at scale {scale}"
        );
        assert!(
            scaled.get("orbitN", 0.0) > 0.0,
            "non-zero key lost at scale {scale}"
        );
    }
}

#[test]
fn test_scale_counts_doubling_increases_totals() {
    for mode in ModeKey::ALL {
        let base = engine::profiles::base_profile(mode);
        let scaled = scale_counts(&base, 2.0);
        let base_total = total_count(&base);
        let scaled_total = total_count(&scaled);
        if base_total > 0.0 {
            assert!(
                scaled_total > base_total,
                "doubling scale did not increase counts for {mode:?}: {base_total} -> {scaled_total}"
            );
        }
    }
}

#[test]
fn test_scale_radii_scales_radius_keys_and_accumulates_r_size_mul() {
    let opts = engine::profiles::ModeOpts::from_pairs(&[("rBase", 0.6), ("nodeR", 1.4)]);
    let once = scale_radii(&opts, 2.0);
    assert!((once.get("rBase", 0.0) - 1.2).abs() < 1e-9);
    assert!((once.get("nodeR", 0.0) - 2.8).abs() < 1e-9);
    assert!(
        (once.get("rSizeMul", 0.0) - 2.0).abs() < 1e-9,
        "rSizeMul must record the applied multiplier"
    );

    let twice = scale_radii(&once, 2.0);
    assert!(
        (twice.get("rSizeMul", 0.0) - 4.0).abs() < 1e-9,
        "rSizeMul must accumulate across scalings"
    );
}

// ============================================================================
// engine::frame smoke — every mode with its resolved Px64 opts
// ============================================================================

#[test]
fn test_engine_frame_smoke_all_modes() {
    for state in OrbState::ALL {
        let resolved = resolve_preset(state, OrbSize::Px64);
        let frame = engine::frame(resolved.mode, 64.0, 1.7, &resolved.opts);
        if resolved.mode == ModeKey::Web {
            assert!(
                !frame.lines.is_empty(),
                "web mode produced no lines for {state:?}"
            );
        } else {
            assert!(
                !frame.dots.is_empty(),
                "no dots for {state:?} ({:?})",
                resolved.mode
            );
        }
        for dot in &frame.dots {
            assert!(
                dot.x.is_finite() && dot.y.is_finite() && dot.z.is_finite() && dot.r.is_finite(),
                "non-finite dot for {state:?}: {dot:?}"
            );
        }
        for line in &frame.lines {
            assert!(
                line.x1.is_finite()
                    && line.y1.is_finite()
                    && line.x2.is_finite()
                    && line.y2.is_finite(),
                "non-finite line for {state:?}: {line:?}"
            );
        }
    }
}

// ============================================================================
// FrameStats (feature `vello`)
// ============================================================================

#[test]
fn test_frame_stats_field_types() {
    let stats = FrameStats {
        dots: 42usize,
        lines: 7usize,
        geometry_time: Duration::from_micros(123),
    };
    let _: usize = stats.dots;
    let _: usize = stats.lines;
    let _: Duration = stats.geometry_time;

    let default = FrameStats::default();
    assert_eq!(default.dots, 0);
    assert_eq!(default.lines, 0);
    assert_eq!(default.geometry_time, Duration::ZERO);
}
