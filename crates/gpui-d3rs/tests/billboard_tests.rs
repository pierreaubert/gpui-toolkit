//! CPU-side coverage for the shared SDF billboard pass: instance layout,
//! alignment, atlas coverage, and the vertex-attribute contract the WGSL
//! shader relies on.

use d3rs::gputext::billboard::{BillboardInstance, WorldLabel, build_instances, shared_atlas};
use d3rs::text::{HorizontalTextAnchor, VerticalTextAnchor};
use glam::Vec3;

fn label(text: &str) -> WorldLabel {
    WorldLabel {
        text: text.to_string(),
        anchor: Vec3::ZERO,
        size_px: 10.0,
        color: [1.0, 1.0, 1.0, 1.0],
        horizontal: HorizontalTextAnchor::Middle,
        vertical: VerticalTextAnchor::Middle,
        screen_offset_px: [0.0, 0.0],
    }
}

#[test]
fn atlas_covers_tick_charset() {
    let atlas = shared_atlas();
    assert!(atlas.size_px > 0);
    assert_eq!(atlas.data.len(), (atlas.size_px * atlas.size_px) as usize);
    for ch in "0123456789.-+e°dB".chars() {
        assert!(atlas.glyph(ch).is_some(), "atlas misses {ch:?}");
    }
}

#[test]
fn middle_alignment_centers_advance() {
    use d3rs::gputext::sdf::layout_string;

    // Middle alignment shifts every glyph left by half the pen advance — the
    // same typographic convention the screen-space path uses for its Middle
    // anchor (ink bearings may overhang symmetrically; the advance centers).
    let middle = build_instances(shared_atlas(), &[label("012")], |_| Some(0.01));
    let start = build_instances(
        shared_atlas(),
        &[WorldLabel {
            horizontal: HorizontalTextAnchor::Start,
            ..label("012")
        }],
        |_| Some(0.01),
    );
    assert_eq!(middle.len(), 3);
    assert_eq!(start.len(), 3);
    let advance = layout_string(shared_atlas(), "012", 10.0).advance_px;
    for (mid, first) in middle.iter().zip(start.iter()) {
        assert_eq!(mid.offset[0] - first.offset[0], -advance / 2.0);
        assert_eq!(mid.offset[1], first.offset[1]);
    }
    // ...and the advance straddles the anchor.
    assert!(middle[0].offset[0] < 0.0);
    assert!(middle[2].offset[0] > 0.0);
}

#[test]
fn unprojectable_and_empty_labels_emit_nothing() {
    let labels = vec![
        label(""),
        WorldLabel {
            text: "12".to_string(),
            ..label("")
        },
    ];
    // First label is empty; the second anchor refuses to project.
    let instances = build_instances(shared_atlas(), &labels, |anchor| {
        (anchor != Vec3::ZERO).then_some(0.01)
    });
    assert!(instances.is_empty());
}

#[test]
fn screen_offset_shifts_every_glyph() {
    let plain = build_instances(shared_atlas(), &[label("7")], |_| Some(0.02));
    let shifted = build_instances(
        shared_atlas(),
        &[WorldLabel {
            screen_offset_px: [3.0, -4.0],
            ..label("7")
        }],
        |_| Some(0.02),
    );
    assert_eq!(plain.len(), 1);
    assert_eq!(shifted.len(), 1);
    assert_eq!(shifted[0].offset[0] - plain[0].offset[0], 3.0);
    assert_eq!(shifted[0].offset[1] - plain[0].offset[1], -4.0);
    // Scale and color pass through untouched.
    assert_eq!(shifted[0].px_to_world, 0.02);
    assert_eq!(shifted[0].color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn instance_layout_matches_wgsl_attributes() {
    // repr(C) float order the shader's locations 1-7 decode: anchor@0,
    // px_to_world@12, offset@16, size@24, uv_min@32, uv_max@40, color@48.
    assert_eq!(std::mem::size_of::<BillboardInstance>(), 64);
    let instance = BillboardInstance {
        anchor: [1.0, 2.0, 3.0],
        px_to_world: 0.5,
        offset: [4.0, 5.0],
        size: [6.0, 7.0],
        uv_min: [0.0, 0.25],
        uv_max: [0.5, 0.75],
        color: [1.0, 0.0, 0.5, 1.0],
    };
    let words: &[f32] = bytemuck::cast_slice(bytemuck::bytes_of(&instance));
    assert_eq!(words[0..3], [1.0, 2.0, 3.0]);
    assert_eq!(words[3], 0.5);
    assert_eq!(words[4..6], [4.0, 5.0]);
    assert_eq!(words[6..8], [6.0, 7.0]);
    assert_eq!(words[8..10], [0.0, 0.25]);
    assert_eq!(words[10..12], [0.5, 0.75]);
    assert_eq!(words[12..16], [1.0, 0.0, 0.5, 1.0]);
}
