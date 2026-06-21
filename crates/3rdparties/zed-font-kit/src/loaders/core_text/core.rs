use std::f32;
use crate :: properties :: { Stretch , Weight } ;
use super::consts::FONT_WEIGHT_MAPPING;
use super::piecewise::piecewise_linear_find_index;
use super::piecewise::piecewise_linear_lookup;

pub(super) fn core_text_to_css_font_weight(core_text_weight: f32) -> Weight {
    let index = piecewise_linear_find_index(core_text_weight, &FONT_WEIGHT_MAPPING);

    Weight(index * 100.0 + 100.0)
}

pub(super) fn core_text_width_to_css_stretchiness(core_text_width: f32) -> Stretch {
    Stretch(piecewise_linear_lookup(
        (core_text_width + 1.0) * 4.0,
        &Stretch::MAPPING,
    ))
}

