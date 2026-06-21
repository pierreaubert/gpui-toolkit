use font_kit :: { properties :: { Style as FontkitStyle , Weight as FontkitWeight } } ;
use gpui :: { FontStyle , FontWeight } ;

pub(super) fn fontkit_weight(value: FontWeight) -> FontkitWeight {
    FontkitWeight(value.0)
}

pub(super) fn fontkit_style(style: FontStyle) -> FontkitStyle {
    match style {
        FontStyle::Normal => FontkitStyle::Normal,
        FontStyle::Italic => FontkitStyle::Italic,
        FontStyle::Oblique => FontkitStyle::Oblique,
    }
}

