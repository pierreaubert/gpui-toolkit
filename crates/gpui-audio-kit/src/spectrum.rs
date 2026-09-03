//! Spectrum analyzer primitives for audio UIs.

mod meter_data;
mod meter_fifo;
mod misc;
mod render;
mod spectrum_axis_theme;
mod spectrum_colors;
mod spectrum_element;
#[cfg(test)]
mod tests;
mod types;

pub use meter_data::*;
pub use meter_fifo::*;
pub use misc::*;
pub use render::*;
pub use spectrum_axis_theme::*;
pub use spectrum_colors::*;
pub use spectrum_element::*;
pub use types::*;

use misc::SPECTRUM_STANDARD_FREQUENCIES;
use misc::valid_frequency_range;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use types::SPECTRUM_DB_AXIS_LABELS;

thread_local! {
    static FREQUENCY_AXIS_LABEL_CACHE: RefCell<HashMap<(u32, u32), Arc<[SpectrumAxisLabel]>>> =
        RefCell::new(HashMap::new());
}

const FREQUENCY_AXIS_LABEL_CACHE_CAPACITY: usize = 64;

/// Generate non-overlapping frequency labels for a logarithmic spectrum axis.
///
/// Results are cached keyed by `(min_freq, max_freq)` so repeated renders with
/// the same range reuse the same allocation.
pub fn spectrum_frequency_axis_labels(min_freq: f32, max_freq: f32) -> Arc<[SpectrumAxisLabel]> {
    let (min_freq, max_freq) = valid_frequency_range(min_freq, max_freq);
    let key = (min_freq.to_bits(), max_freq.to_bits());

    FREQUENCY_AXIS_LABEL_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(&key) {
            return cached.clone();
        }

        let mut labels = Vec::new();

        labels.push(SpectrumAxisLabel {
            label: format_spectrum_frequency_label(min_freq),
            position: 0.0,
        });

        for freq in SPECTRUM_STANDARD_FREQUENCIES {
            if freq > min_freq * 1.1 && freq < max_freq * 0.9 {
                labels.push(SpectrumAxisLabel {
                    label: format_spectrum_frequency_label(freq),
                    position: logarithmic_frequency_position(freq, min_freq, max_freq),
                });
            }
        }

        labels.push(SpectrumAxisLabel {
            label: format_spectrum_frequency_label(max_freq),
            position: 1.0,
        });

        let mut filtered = Vec::new();
        for label in labels {
            if filtered.is_empty()
                || filtered
                    .last()
                    .map(|last: &SpectrumAxisLabel| label.position - last.position > 0.08)
                    .unwrap_or(true)
            {
                filtered.push(label);
            }
        }

        let arc: Arc<[SpectrumAxisLabel]> = filtered.into();
        let mut cache = cache.borrow_mut();
        if cache.len() >= FREQUENCY_AXIS_LABEL_CACHE_CAPACITY
            && let Some(evicted_key) = cache.keys().next().copied()
        {
            cache.remove(&evicted_key);
        }
        cache.insert(key, arc.clone());
        arc
    })
}

/// Fixed dB-axis labels used by spectrum analyzer views.
pub fn spectrum_db_axis_labels() -> &'static [SpectrumDbAxisLabel] {
    &SPECTRUM_DB_AXIS_LABELS
}
