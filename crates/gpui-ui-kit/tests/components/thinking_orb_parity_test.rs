//! Golden-vector parity test for the thinking-orbs engine port.
//!
//! Compares the Rust engine against `tests/data/orbs-golden.json`, the exact
//! output of the TypeScript `thinking-orbs` 0.3.1 engine (MIT © Jakub
//! Antalik) at fixed timestamps. Two layers:
//!
//! 1. Resolved-opts parity — `resolve_preset(state, size)` must reproduce the
//!    golden's resolved mode/speed/opts for every (state, size) pair.
//! 2. Frame parity — `engine::frame(mode, size, t, opts)` must reproduce every
//!    dot (stride 6) and line (stride 7) within the golden's 1e-4 tolerance.

use std::str::FromStr;

use gpui_ui_kit::thinking_orb::engine;
use gpui_ui_kit::thinking_orb::presets::{OrbSize, OrbState, resolve_preset};
use serde_json::Value;

const GOLDEN: &str = include_str!("../data/orbs-golden.json");

fn golden() -> Value {
    serde_json::from_str(GOLDEN).expect("orbs-golden.json must parse")
}

fn parse_state_size(case: &Value) -> (OrbState, OrbSize) {
    let state = OrbState::from_str(case["state"].as_str().unwrap())
        .unwrap_or_else(|_| panic!("unknown state in case {}", case["key"]));
    let size = match case["size"].as_u64().unwrap() {
        64 => OrbSize::Px64,
        20 => OrbSize::Px20,
        other => panic!("unknown size {other} in case {}", case["key"]),
    };
    (state, size)
}

#[test]
fn test_thinking_orb_resolved_opts_parity() {
    let golden = golden();
    let resolved = golden["resolved"].as_object().unwrap();
    let mut matched = 0usize;
    for (key, entry) in resolved {
        let (state_key, size_key) = key
            .rsplit_once('-')
            .unwrap_or_else(|| panic!("malformed resolved key {key}"));
        let state = OrbState::from_str(state_key).expect("known state");
        let size = OrbSize::from_str(size_key).expect("known size");

        let ours = resolve_preset(state, size);
        assert_eq!(
            ours.mode.as_str(),
            entry["mode"].as_str().unwrap(),
            "mode mismatch for {key}"
        );
        let golden_speed = entry["speed"].as_f64().unwrap();
        assert!(
            (ours.speed - golden_speed).abs() <= 1e-9,
            "speed mismatch for {key}: ours={} golden={golden_speed}",
            ours.speed
        );

        let golden_opts = entry["opts"].as_object().unwrap();
        assert_eq!(
            ours.opts.len(),
            golden_opts.len(),
            "opts key-count mismatch for {key}: ours={:?}",
            ours.opts
        );
        for (k, v) in golden_opts {
            let gv = v.as_f64().unwrap();
            assert!(ours.opts.contains(k), "missing opts key {k} for {key}");
            let ov = ours.opts.get(k, f64::NAN);
            if gv.fract() == 0.0 {
                // integer-valued keys (counts) must match exactly
                assert_eq!(ov, gv, "integer opts key {k} mismatch for {key}");
            } else {
                assert!(
                    (ov - gv).abs() <= 1e-9,
                    "opts key {k} mismatch for {key}: ours={ov} golden={gv}"
                );
            }
        }
        matched += 1;
    }
    println!(
        "resolved-opts parity: {matched}/{} (state, size) pairs matched",
        resolved.len()
    );
    assert_eq!(matched, resolved.len());
}

#[test]
fn test_thinking_orb_frame_parity() {
    let golden = golden();
    let tolerance = golden["tolerance"].as_f64().unwrap();
    let cases = golden["cases"].as_array().unwrap();

    let mut dots_matched = 0usize;
    let mut lines_matched = 0usize;
    for case in cases {
        let key = case["key"].as_str().unwrap();
        let (state, size) = parse_state_size(case);
        let t = case["t"].as_f64().unwrap();

        let resolved = resolve_preset(state, size);
        assert_eq!(
            resolved.mode.as_str(),
            case["mode"].as_str().unwrap(),
            "mode mismatch for case {key}"
        );
        // golden frames use RAW t — speed is a renderer-side clock multiplier
        let frame = engine::frame(resolved.mode, size.pixels(), t, &resolved.opts);

        let golden_dots = case["dots"].as_array().unwrap();
        let golden_lines = case["lines"].as_array().unwrap();
        assert_eq!(
            frame.dots.len(),
            case["dotCount"].as_u64().unwrap() as usize,
            "dotCount mismatch for case {key}"
        );
        assert_eq!(
            frame.lines.len(),
            case["lineCount"].as_u64().unwrap() as usize,
            "lineCount mismatch for case {key}"
        );
        assert_eq!(
            frame.dots.len() * 6,
            golden_dots.len(),
            "dots stride for case {key}"
        );
        assert_eq!(
            frame.lines.len() * 7,
            golden_lines.len(),
            "lines stride for case {key}"
        );

        // Compare in draw order. Dots are z-sorted; when the golden's rounded
        // z ties exactly across a run, the raw pre-rounding z's differ by less
        // than 1e-6 and their order encodes last-ulp libm differences between
        // V8 (which produced the golden) and Rust's system libm — not a port
        // error (e.g. ring's face-on mid-lane has true z = 0, so the sort key
        // is pure FP residue). Within a tie run we therefore compare as a
        // multiset; every value is still checked against the same tolerance.
        let mut run_start = 0usize;
        while run_start < frame.dots.len() {
            let z0 = golden_dots[run_start * 6 + 2].as_f64().unwrap();
            let mut run_end = run_start + 1;
            while run_end < frame.dots.len() && golden_dots[run_end * 6 + 2].as_f64().unwrap() == z0
            {
                run_end += 1;
            }
            if run_end - run_start == 1 {
                let d = &frame.dots[run_start];
                let vals = [d.x, d.y, d.z, d.r, d.white, d.a.unwrap_or(1.0)];
                for (field, fv) in vals.iter().enumerate() {
                    let gv = golden_dots[run_start * 6 + field].as_f64().unwrap();
                    assert!(
                        (fv - gv).abs() <= tolerance,
                        "dot {run_start} field {field} mismatch for case {key}: ours={fv} golden={gv}"
                    );
                }
            } else {
                let mut used = vec![false; run_end - run_start];
                for i in run_start..run_end {
                    let d = &frame.dots[i];
                    let vals = [d.x, d.y, d.z, d.r, d.white, d.a.unwrap_or(1.0)];
                    let mut found = false;
                    for j in run_start..run_end {
                        if used[j - run_start] {
                            continue;
                        }
                        let matches = vals.iter().enumerate().all(|(field, fv)| {
                            (fv - golden_dots[j * 6 + field].as_f64().unwrap()).abs() <= tolerance
                        });
                        if matches {
                            used[j - run_start] = true;
                            found = true;
                            break;
                        }
                    }
                    assert!(
                        found,
                        "dot {i} in z-tie run {run_start}..{run_end} has no match for case {key}"
                    );
                }
            }
            dots_matched += run_end - run_start;
            run_start = run_end;
        }
        for (i, l) in frame.lines.iter().enumerate() {
            let vals = [l.x1, l.y1, l.x2, l.y2, l.white, l.a.unwrap_or(1.0), l.w];
            for (field, fv) in vals.iter().enumerate() {
                let gv = golden_lines[i * 7 + field].as_f64().unwrap();
                assert!(
                    (fv - gv).abs() <= tolerance,
                    "line {i} field {field} mismatch for case {key}: ours={fv} golden={gv}"
                );
            }
            lines_matched += 1;
        }
    }
    println!(
        "frame parity: {}/{} cases, {} dots, {} lines matched (tolerance {tolerance})",
        cases.len(),
        cases.len(),
        dots_matched,
        lines_matched
    );
}
