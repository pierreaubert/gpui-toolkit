#![cfg(feature = "gpu-2d")]

use d3rs::gpu2d::Gpu2DContext;
use d3rs::gpu2d::text::TextAtlas;

#[test]
fn atlas_grows_and_repacks_cached_glyphs() {
    let Ok(gpu) = Gpu2DContext::try_global() else {
        return;
    };
    let font = include_bytes!("../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf");
    let mut atlas = TextAtlas::new(gpu.device(), gpu.queue(), font, 32);
    let original = atlas.get_glyph('A', 16.0).expect("initial glyph fits");

    for codepoint in b'!'..=b'~' {
        assert!(atlas.get_glyph(char::from(codepoint), 16.0).is_some());
    }

    assert!(atlas.get_glyph('A', 16.0).is_some());
    assert_eq!(
        atlas.get_glyph('A', 16.0).expect("repacked glyph").advance,
        original.advance
    );
}
