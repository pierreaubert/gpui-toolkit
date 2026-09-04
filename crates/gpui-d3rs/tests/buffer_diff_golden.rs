//! Golden equivalence: diff-gated uploads vs full per-frame uploads.
//!
//! Fixture `golden/gpu/buffer_diff.json` is generated with
//! `cd golden && node gpu_buffer_diff.mjs`: per-frame 12-float
//! composite-uniform payloads (the exact layout `vello2d::wgpu_draw` uploads)
//! across static, panning, and pan-then-settle scenarios.
//!
//! For every frame this test feeds the payload through two simulated upload
//! paths — one that "writes" unconditionally (the old behavior) and one
//! gated by `mesh::BufferUploadCache` (the persistent-buffer behavior in
//! `wgpu_draw`, `mesh::gpu::wgpu_backend`, and `mesh::gpu::renderer3d`) —
//! and asserts the delivered byte streams are identical while the gated
//! path performs exactly the frozen `expected_writes` count. Skipping a
//! write is therefore proven unobservable: the buffer always holds the
//! current frame's bytes.

use d3rs::mesh::BufferUploadCache;
use std::fs;

fn frame_bytes(frame: &[f64]) -> Vec<u8> {
    assert_eq!(frame.len(), 12, "composite uniform payload is 12 floats");
    frame
        .iter()
        .flat_map(|v| (*v as f32).to_le_bytes())
        .collect()
}

#[test]
fn diff_gated_upload_matches_full_upload() {
    let content = fs::read_to_string("golden/gpu/buffer_diff.json").expect("golden file not found");
    let golden: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(golden["module"], "d3rs-buffer-diff");
    assert_eq!(golden["function"], "BufferUploadCache");

    for case in golden["test_cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let frames: Vec<Vec<f64>> = serde_json::from_value(case["frames"].clone()).unwrap();
        let expected_writes = case["expected_writes"].as_u64().unwrap() as usize;
        assert!(!frames.is_empty(), "case '{name}' has no frames");

        // Full-upload path: every frame is written.
        let mut full_buffer: Vec<u8> = Vec::new();
        let mut full_writes = 0usize;
        // Diff-gated path: the queue write is skipped when bytes match.
        let mut cache = BufferUploadCache::new();
        let mut gated_buffer: Vec<u8> = Vec::new();
        let mut gated_writes = 0usize;

        for frame in &frames {
            let bytes = frame_bytes(frame);
            full_buffer.clear();
            full_buffer.extend_from_slice(&bytes);
            full_writes += 1;
            if cache.needs_write(&bytes) {
                gated_buffer.clear();
                gated_buffer.extend_from_slice(&bytes);
                gated_writes += 1;
            }
            // After every frame both paths must hold the current payload:
            // skipping the write is unobservable.
            assert_eq!(
                gated_buffer, full_buffer,
                "case '{name}': gated buffer diverged from full upload"
            );
        }

        assert_eq!(
            full_writes,
            frames.len(),
            "case '{name}': full path must write every frame"
        );
        assert_eq!(
            gated_writes, expected_writes,
            "case '{name}': gated writes {gated_writes} != frozen expectation {expected_writes}"
        );
        if expected_writes < frames.len() {
            println!(
                "case {name}: skipped {} of {} queue writes",
                frames.len() - gated_writes,
                frames.len()
            );
        }
    }
}
