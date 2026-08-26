use super::misc::find_nice_step;
use super::misc::format_value_abbrev;
use super::types::TickMark;
use crate::scale::Scale;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Calculate tick marks for linear scale
fn calculate_linear_ticks(min: f64, max: f64, track_height: f32) -> Vec<TickMark> {
    let range = max - min;
    if range <= 0.0 {
        return vec![
            TickMark {
                value: min,
                normalized_pos: 0.0,
                is_major: true,
                label: Some(format_value_abbrev(min)),
            },
            TickMark {
                value: max,
                normalized_pos: 1.0,
                is_major: true,
                label: Some(format_value_abbrev(max)),
            },
        ];
    }

    // Determine target labeled tick count based on height
    // Minimum 2 labels (min/max), up to 6 for very tall sliders
    let target_labels = ((track_height / 40.0) as usize).clamp(2, 6);

    // Find nice step for labels
    let label_step = find_nice_step(range, target_labels);

    // Minor ticks: more frequent, about twice as many
    let minor_step = label_step / 2.0;

    let mut ticks = Vec::new();

    // Always add min as major tick with label
    ticks.push(TickMark {
        value: min,
        normalized_pos: 0.0,
        is_major: true,
        label: Some(format_value_abbrev(min)),
    });

    // Add intermediate ticks
    let first_label_tick = (min / label_step).ceil() * label_step;
    let first_minor_tick = (min / minor_step).ceil() * minor_step;

    // Collect all tick positions
    let mut tick_value = first_minor_tick;
    while tick_value < max - minor_step * 0.1 {
        if (tick_value - min).abs() > minor_step * 0.1 {
            let normalized = (tick_value - min) / range;

            // Check if this is a label tick (on label_step boundary)
            let is_label_tick = ((tick_value - first_label_tick) / label_step).round().abs()
                * label_step
                + first_label_tick;
            let is_labeled = (tick_value - is_label_tick).abs() < label_step * 0.01;

            ticks.push(TickMark {
                value: tick_value,
                normalized_pos: normalized,
                is_major: is_labeled,
                label: if is_labeled {
                    Some(format_value_abbrev(tick_value))
                } else {
                    None
                },
            });
        }
        tick_value += minor_step;
    }

    // Always add max as major tick with label
    ticks.push(TickMark {
        value: max,
        normalized_pos: 1.0,
        is_major: true,
        label: Some(format_value_abbrev(max)),
    });

    ticks
}

/// Calculate tick marks for logarithmic scale
fn calculate_log_ticks(min: f64, max: f64, track_height: f32) -> Vec<TickMark> {
    let min = min.max(1e-10);
    let max = max.max(min + 1e-10);

    let log_min = min.ln();
    let log_max = max.ln();
    let log_range = log_max - log_min;

    if log_range <= 0.0 {
        return vec![
            TickMark {
                value: min,
                normalized_pos: 0.0,
                is_major: true,
                label: Some(format_value_abbrev(min)),
            },
            TickMark {
                value: max,
                normalized_pos: 1.0,
                is_major: true,
                label: Some(format_value_abbrev(max)),
            },
        ];
    }

    let mut ticks = Vec::new();

    // Always add min as major tick with label
    ticks.push(TickMark {
        value: min,
        normalized_pos: 0.0,
        is_major: true,
        label: Some(format_value_abbrev(min)),
    });

    // Calculate decade range
    let min_decade = min.log10().floor() as i32;
    let max_decade = max.log10().ceil() as i32;
    let num_decades = (max_decade - min_decade) as usize;

    // Determine how many labels we can fit based on height
    // About one label per 35-40 pixels, minimum 2
    let max_labels = ((track_height / 35.0) as usize).clamp(2, 8);

    // Decide which decade markers get labels
    // If few decades, label all of them; otherwise label every Nth
    let label_every_n = if num_decades <= max_labels {
        1
    } else {
        (num_decades / max_labels).max(1)
    };

    // Determine detail level based on height
    let include_sub_decades = track_height >= 80.0;

    // Add decade markers and sub-decade markers
    let mut decade_index = 0;
    for decade in min_decade..=max_decade {
        let decade_value = 10_f64.powi(decade);

        // Main decade marker (1, 10, 100, 1k, 10k, etc.)
        if decade_value > min * 1.05 && decade_value < max * 0.95 {
            let normalized = (decade_value.ln() - log_min) / log_range;
            let should_label = decade_index % label_every_n == 0;
            ticks.push(TickMark {
                value: decade_value,
                normalized_pos: normalized,
                is_major: should_label,
                label: if should_label {
                    Some(format_value_abbrev(decade_value))
                } else {
                    None
                },
            });
            decade_index += 1;
        }

        // Sub-decade markers (2, 5) if we have enough space
        if include_sub_decades {
            for multiplier in [2.0, 5.0] {
                let sub_value = decade_value * multiplier;
                if sub_value > min * 1.05 && sub_value < max * 0.95 {
                    let normalized = (sub_value.ln() - log_min) / log_range;
                    ticks.push(TickMark {
                        value: sub_value,
                        normalized_pos: normalized,
                        is_major: false,
                        label: None,
                    });
                }
            }
        }
    }

    // Always add max as major tick with label
    ticks.push(TickMark {
        value: max,
        normalized_pos: 1.0,
        is_major: true,
        label: Some(format_value_abbrev(max)),
    });

    // Sort by normalized position
    ticks.sort_by(|a, b| a.normalized_pos.partial_cmp(&b.normalized_pos).unwrap());

    ticks
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TickCacheKey {
    min: i64,
    max: i64,
    scale: Scale,
    track_height: i32,
}

impl TickCacheKey {
    fn new(min: f64, max: f64, scale: Scale, track_height: f32) -> Self {
        Self {
            min: (min * 1_000_000.0).round() as i64,
            max: (max * 1_000_000.0).round() as i64,
            scale,
            track_height: (track_height * 1_000.0).round() as i32,
        }
    }
}

thread_local! {
    static TICK_CACHE: RefCell<HashMap<TickCacheKey, Arc<[TickMark]>>> = RefCell::new(HashMap::new());
}

const TICK_CACHE_CAPACITY: usize = 64;

/// Calculate tick marks based on scale type
///
/// Results are cached by `(min, max, scale, track_height)` so repeated renders
/// with unchanged parameters avoid re-allocating tick label strings.
pub(super) fn calculate_ticks(
    min: f64,
    max: f64,
    scale: Scale,
    track_height: f32,
) -> Arc<[TickMark]> {
    let key = TickCacheKey::new(min, max, scale, track_height);
    TICK_CACHE.with(|cache| {
        if let Some(cached) = cache.borrow().get(&key) {
            return cached.clone();
        }

        let ticks = match scale {
            Scale::Linear => calculate_linear_ticks(min, max, track_height),
            Scale::Logarithmic => calculate_log_ticks(min, max, track_height),
        };

        let ticks: Arc<[TickMark]> = ticks.into();
        let mut cache = cache.borrow_mut();
        if cache.len() >= TICK_CACHE_CAPACITY {
            if let Some(evicted_key) = cache.keys().next().copied() {
                cache.remove(&evicted_key);
            }
        }
        cache.insert(key, ticks.clone());
        ticks
    })
}

#[cfg(test)]
mod tests {
    use super::{TICK_CACHE, TICK_CACHE_CAPACITY, calculate_ticks};
    use crate::AudioScale as Scale;

    #[test]
    fn linear_ticks_include_min_and_max() {
        let ticks = calculate_ticks(0.0, 100.0, Scale::Linear, 160.0);
        assert!(ticks.first().unwrap().normalized_pos == 0.0);
        assert!(ticks.last().unwrap().normalized_pos == 1.0);
        assert!(ticks.iter().any(|t| t.is_major));
    }

    #[test]
    fn logarithmic_ticks_include_min_and_max() {
        let ticks = calculate_ticks(20.0, 20_000.0, Scale::Logarithmic, 160.0);
        assert!(ticks.first().unwrap().normalized_pos == 0.0);
        assert!(ticks.last().unwrap().normalized_pos == 1.0);
    }

    #[test]
    fn calculate_ticks_is_cached() {
        let a = calculate_ticks(0.0, 100.0, Scale::Linear, 160.0);
        let b = calculate_ticks(0.0, 100.0, Scale::Linear, 160.0);
        assert_eq!(a.len(), b.len());
        assert!(std::ptr::eq(a.as_ptr(), b.as_ptr()));
    }

    #[test]
    fn tick_cache_is_bounded() {
        for index in 0..=(TICK_CACHE_CAPACITY as u32 + 1) {
            calculate_ticks(0.0, 100.0 + index as f64, Scale::Linear, 160.0);
        }
        TICK_CACHE.with(|cache| assert!(cache.borrow().len() <= TICK_CACHE_CAPACITY));
    }

    #[test]
    fn degenerate_ranges_still_produce_bounds() {
        let linear = calculate_ticks(10.0, 10.0, Scale::Linear, 80.0);
        assert_eq!(linear.len(), 2);
        assert_eq!(linear[0].normalized_pos, 0.0);
        assert_eq!(linear[1].normalized_pos, 1.0);

        let log = calculate_ticks(100.0, 100.0, Scale::Logarithmic, 80.0);
        assert_eq!(log.len(), 2);
    }
}
