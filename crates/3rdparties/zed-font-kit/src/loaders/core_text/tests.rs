use crate :: properties :: { Stretch , Weight } ;
use super::core::core_text_to_css_font_weight;
use super::core::core_text_width_to_css_stretchiness;
use super::font::Font;

#[cfg(test)]
mod test {
    use super::*;
    use super::super::Font;
    use crate::properties::{Stretch, Weight};

    #[cfg(feature = "source")]
    use crate::source::SystemSource;

    static TEST_FONT_POSTSCRIPT_NAME: &'static str = "ArialMT";

    #[cfg(feature = "source")]
    #[test]
    fn test_from_core_graphics_font() {
        let font0 = SystemSource::new()
            .select_by_postscript_name(TEST_FONT_POSTSCRIPT_NAME)
            .unwrap()
            .load()
            .unwrap();
        let core_text_font = font0.native_font();
        let core_graphics_font = core_text_font.copy_to_CGFont();
        let font1 = Font::from_core_graphics_font(core_graphics_font);
        assert_eq!(font1.postscript_name().unwrap(), TEST_FONT_POSTSCRIPT_NAME);
    }

    #[test]
    fn test_core_text_to_css_font_weight() {
        // Exact matches
        assert_eq!(super::super::core_text_to_css_font_weight(-0.7), Weight(100.0));
        assert_eq!(super::super::core_text_to_css_font_weight(0.0), Weight(400.0));
        assert_eq!(super::super::core_text_to_css_font_weight(0.4), Weight(700.0));
        assert_eq!(super::super::core_text_to_css_font_weight(0.8), Weight(900.0));

        // Linear interpolation
        assert_eq!(super::super::core_text_to_css_font_weight(0.1), Weight(450.0));
    }

    #[test]
    fn test_core_text_to_css_font_stretch() {
        // Exact matches
        assert_eq!(
            super::super::core_text_width_to_css_stretchiness(0.0),
            Stretch(1.0)
        );
        assert_eq!(
            super::super::core_text_width_to_css_stretchiness(-1.0),
            Stretch(0.5)
        );
        assert_eq!(
            super::super::core_text_width_to_css_stretchiness(1.0),
            Stretch(2.0)
        );

        // Linear interpolation
        assert_eq!(
            super::super::core_text_width_to_css_stretchiness(0.85),
            Stretch(1.7)
        );
    }
}

