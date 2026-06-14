use super::*;
use gpui::{FontRun, px};

fn test_state() -> IosTextSystemState {
    IosTextSystemState {
        memory_source: MemSource::empty(),
        system_source: SystemSource::new(),
        fonts: Vec::new(),
        font_selections: HashMap::default(),
        font_ids_by_postscript_name: HashMap::default(),
        font_ids_by_font_key: HashMap::default(),
        postscript_names_by_font_id: HashMap::default(),
        layout_cache: HashMap::default(),
    }
}

#[test]
fn layout_line_reuses_cached_result() {
    let mut state = test_state();
    let font_ids = state
        .load_family("Helvetica", &Default::default(), None)
        .expect("Helvetica should be available on Apple targets");
    assert!(
        !font_ids.is_empty(),
        "load_family should return at least one font"
    );

    let run = FontRun {
        len: 5,
        font_id: font_ids[0],
    };
    let layout1 = state.layout_line("Hello", px(16.0), &[run]);
    assert_eq!(state.layout_cache.len(), 1);

    let layout2 = state.layout_line("Hello", px(16.0), &[run]);
    assert_eq!(state.layout_cache.len(), 1);
    assert_eq!(layout1.width, layout2.width);
    assert_eq!(layout1.runs.len(), layout2.runs.len());
}

#[test]
fn layout_line_caches_by_key() {
    let mut state = test_state();
    let font_ids = state
        .load_family("Helvetica", &Default::default(), None)
        .expect("Helvetica should be available");
    let run = FontRun {
        len: 5,
        font_id: font_ids[0],
    };

    let _ = state.layout_line("Hello", px(16.0), &[run]);
    let _ = state.layout_line("World", px(16.0), &[run]);
    let _ = state.layout_line("Hello", px(24.0), &[run]);
    assert_eq!(state.layout_cache.len(), 3);
}

#[test]
fn id_for_native_font_looks_up_existing_font() {
    let mut state = test_state();
    let font_ids = state
        .load_family("Helvetica", &Default::default(), None)
        .expect("Helvetica should be available");
    let font_id = font_ids[0];

    let ct_font = state.fonts[font_id.0]
        .native_font()
        .clone_with_font_size(16.0);
    assert_eq!(state.id_for_native_font(ct_font), font_id);
    assert_eq!(state.font_ids_by_postscript_name.len(), state.fonts.len());
}

fn glyph_bitmap_scratch_capacity() -> usize {
    GLYPH_BITMAP_SCRATCH.with(|s| s.borrow().capacity())
}

#[test]
fn rasterize_glyph_reuses_scratch() {
    let mut state = test_state();
    let font_id = state
        .load_family(".AppleSystemUIFont", &Default::default(), None)
        .ok()
        .and_then(|ids| ids.first().copied())
        .expect("system font should be available");
    let glyph_id = state
        .glyph_for_char(font_id, 'A')
        .expect("glyph should exist");
    let params = RenderGlyphParams {
        font_id,
        glyph_id,
        font_size: px(24.0),
        subpixel_variant: point(0, 0),
        scale_factor: 1.0,
        is_emoji: false,
        subpixel_rendering: false,
    };
    let bounds = state.raster_bounds(&params).expect("bounds");
    let (size1, _) = state.rasterize_glyph(&params, bounds).expect("rasterize");
    let cap_after_warmup = glyph_bitmap_scratch_capacity();
    assert!(
        cap_after_warmup >= size1.width.0 as usize * size1.height.0 as usize,
        "scratch should be large enough for the bitmap"
    );

    let (size2, _) = state
        .rasterize_glyph(&params, bounds)
        .expect("rasterize again");
    let cap_after_reuse = glyph_bitmap_scratch_capacity();
    assert_eq!(size1, size2);
    assert_eq!(
        cap_after_warmup, cap_after_reuse,
        "scratch buffer capacity should not grow on repeated rasterization"
    );
}

#[test]
fn rasterize_glyph_reuses_context() {
    let mut state = test_state();
    let font_id = state
        .load_family(".AppleSystemUIFont", &Default::default(), None)
        .ok()
        .and_then(|ids| ids.first().copied())
        .expect("system font should be available");
    let glyph_id = state
        .glyph_for_char(font_id, 'A')
        .expect("glyph should exist");
    let params = RenderGlyphParams {
        font_id,
        glyph_id,
        font_size: px(24.0),
        subpixel_variant: point(0, 0),
        scale_factor: 1.0,
        is_emoji: false,
        subpixel_rendering: false,
    };
    let bounds = state.raster_bounds(&params).expect("bounds");

    let count_before = context_create_count();
    let (size1, _) = state.rasterize_glyph(&params, bounds).expect("rasterize");
    let count_after_first = context_create_count();
    assert_eq!(
        count_after_first,
        count_before + 1,
        "first rasterization should create a context"
    );

    let (size2, _) = state
        .rasterize_glyph(&params, bounds)
        .expect("rasterize again");
    let count_after_second = context_create_count();
    assert_eq!(size1, size2);
    assert_eq!(
        count_after_second, count_after_first,
        "second rasterization should reuse the existing context"
    );
}
