use super::*;
use font_kit::source::SystemSource;
use gpui::{DevicePixels, FontRun, GlyphId, point, px, size};

fn empty_state() -> AuTextSystemState {
    AuTextSystemState {
        memory_source: MemSource::empty(),
        system_source: SystemSource::new(),
        fonts: Vec::new(),
        font_selections: HashMap::default(),
        font_ids_by_postscript_name: HashMap::default(),
        font_ids_by_font_key: HashMap::default(),
        postscript_names_by_font_id: HashMap::default(),
        is_emoji: Vec::new(),
        layout_cache: HashMap::default(),
    }
}

fn load_system_family(state: &mut AuTextSystemState, family: &str) -> Option<FontId> {
    state
        .load_family(family, &Default::default(), None)
        .ok()
        .and_then(|ids| ids.first().copied())
}

#[test]
fn test_is_emoji_caches_bool() {
    let mut state = empty_state();
    assert!(!state.is_emoji(FontId(0)));

    if let Some(regular_id) = load_system_family(&mut state, "Helvetica") {
        assert!(!state.is_emoji(regular_id));
    }

    if let Some(emoji_id) = load_system_family(&mut state, "AppleColorEmoji") {
        assert!(state.is_emoji(emoji_id));
    }
}

#[test]
fn test_rasterize_glyph_empty_bounds_errors() {
    let state = empty_state();
    let params = gpui::RenderGlyphParams {
        font_id: FontId(0),
        glyph_id: GlyphId(0),
        font_size: px(12.0),
        subpixel_variant: point(0, 0),
        scale_factor: 1.0,
        is_emoji: false,
        subpixel_rendering: false,
        dilation: 0,
    };
    let bounds = Bounds {
        origin: point(DevicePixels(0), DevicePixels(0)),
        size: size(DevicePixels(0), DevicePixels(0)),
    };
    assert!(state.rasterize_glyph(&params, bounds).is_err());
}

#[test]
fn test_layout_line_empty_text() {
    let mut state = empty_state();
    let layout = state.layout_line("", px(12.0), &[]);
    assert_eq!(layout.width, px(0.0));
    assert_eq!(layout.len, 0);
}

#[test]
fn test_layout_line_repeated_text_is_consistent() {
    let mut state = empty_state();
    let font_id = match load_system_family(&mut state, ".AppleSystemUIFont") {
        Some(id) => id,
        None => {
            // System font may not be available in all test environments.
            return;
        }
    };
    let text = "hello";
    let runs = [FontRun {
        font_id,
        len: text.len(),
    }];
    let layout1 = state.layout_line(text, px(12.0), &runs);
    let layout2 = state.layout_line(text, px(12.0), &runs);
    assert_eq!(layout1.width, layout2.width);
    assert_eq!(layout1.len, layout2.len);
}

#[test]
fn test_string_index_converter_rewind() {
    let mut converter = StringIndexConverter::new("aéb");
    converter.advance_to_utf16_ix(2);
    assert_eq!(converter.utf8_ix, 3);
    assert_eq!(converter.utf16_ix, 2);
    converter.rewind_to_utf16_ix(1);
    assert_eq!(converter.utf8_ix, 1);
    assert_eq!(converter.utf16_ix, 1);
}

#[test]
fn test_layout_line_uses_cache_for_repeated_calls() {
    let mut state = empty_state();
    let font_id = match load_system_family(&mut state, ".AppleSystemUIFont") {
        Some(id) => id,
        None => {
            // System font may not be available in all test environments.
            return;
        }
    };
    let text = "cache me";
    let runs = [FontRun {
        font_id,
        len: text.len(),
    }];

    let layout1 = state.layout_line(text, px(12.0), &runs);
    let cached_count_after_first = state.layout_cache.len();

    let layout2 = state.layout_line(text, px(12.0), &runs);
    let cached_count_after_second = state.layout_cache.len();

    assert_eq!(layout1.width, layout2.width);
    assert_eq!(layout1.len, layout2.len);
    assert_eq!(layout1.runs.len(), layout2.runs.len());
    assert!(
        cached_count_after_first > 0,
        "first layout should populate the cache"
    );
    assert_eq!(
        cached_count_after_first, cached_count_after_second,
        "second layout should reuse the cached entry"
    );
}

#[test]
fn layout_cache_is_bounded() {
    let mut state = empty_state();
    let font_id = match load_system_family(&mut state, ".AppleSystemUIFont") {
        Some(id) => id,
        None => return,
    };

    for index in 0..1_025 {
        let text = format!("layout cache entry {index}");
        let runs = [FontRun {
            font_id,
            len: text.len(),
        }];
        state.layout_line(&text, px(12.0), &runs);
    }

    // The cache is cleared before inserting the entry that would exceed its
    // cap, which bounds long-lived meter/readout sessions without an LRU cost.
    assert_eq!(state.layout_cache.len(), 1);
}

#[test]
fn rasterize_glyph_reuses_context() {
    let mut state = empty_state();
    let font_id = load_system_family(&mut state, ".AppleSystemUIFont")
        .expect("system font should be available");
    let glyph_id = state
        .glyph_for_char(font_id, 'A')
        .expect("glyph should exist");
    let params = gpui::RenderGlyphParams {
        font_id,
        glyph_id,
        font_size: px(24.0),
        subpixel_variant: point(0, 0),
        scale_factor: 1.0,
        is_emoji: false,
        subpixel_rendering: false,
        dilation: 0,
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
