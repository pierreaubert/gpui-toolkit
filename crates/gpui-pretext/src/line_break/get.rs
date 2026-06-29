pub(super) fn get_tab_advance(line_width: f64, tab_stop_advance: f64) -> f64 {
    if tab_stop_advance <= 0.0 {
        return 0.0;
    }
    let remainder = line_width % tab_stop_advance;
    if remainder.abs() <= 1e-6 {
        tab_stop_advance
    } else {
        tab_stop_advance - remainder
    }
}

pub(super) fn get_breakable_advance(
    grapheme_widths: &[f64],
    grapheme_prefix_widths: Option<&[f64]>,
    grapheme_index: usize,
    prefer_prefix_widths: bool,
) -> f64 {
    if !prefer_prefix_widths || grapheme_prefix_widths.is_none() {
        return grapheme_widths[grapheme_index];
    }
    let pw = grapheme_prefix_widths.unwrap();
    pw[grapheme_index]
        - if grapheme_index > 0 {
            pw[grapheme_index - 1]
        } else {
            0.0
        }
}
