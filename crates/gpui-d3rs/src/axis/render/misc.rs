use gpui::SharedString;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

type TickLabelCacheKey = (u64, Option<fn(f64) -> String>);

// Keep per-thread metadata small and cap variable-sized formatter output.
// A second-chance queue gives O(1) cache hits without a linear LRU search.
const MAX_LABELS: usize = 512;
const MAX_LABEL_BYTES: usize = 64 * 1024;

#[derive(Default)]
struct TickLabelCache {
    labels: HashMap<TickLabelCacheKey, (SharedString, bool)>,
    order: VecDeque<TickLabelCacheKey>,
    bytes: usize,
}

impl TickLabelCache {
    fn get(&mut self, key: &TickLabelCacheKey) -> Option<SharedString> {
        self.labels.get_mut(key).map(|(label, referenced)| {
            *referenced = true;
            label.clone()
        })
    }

    fn insert(&mut self, key: TickLabelCacheKey, label: SharedString) {
        if label.len() > MAX_LABEL_BYTES || self.labels.contains_key(&key) {
            return;
        }
        while self.labels.len() >= MAX_LABELS || self.bytes + label.len() > MAX_LABEL_BYTES {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            let Some((_, referenced)) = self.labels.get_mut(&oldest) else {
                continue;
            };
            if *referenced {
                *referenced = false;
                self.order.push_back(oldest);
            } else if let Some((removed, _)) = self.labels.remove(&oldest) {
                self.bytes -= removed.len();
            }
        }
        self.bytes += label.len();
        self.labels.insert(key, (label, false));
        self.order.push_back(key);
    }
}

thread_local! {
    static TICK_LABEL_CACHE: RefCell<TickLabelCache> = RefCell::new(TickLabelCache::default());
}

/// Format a tick value using the optional custom formatter.
///
/// Returns a `SharedString` so formatted labels can be cached and passed to
/// renderers without repeated per-frame `String` allocations. Labels are
/// memoised by `(value, formatter)` so repeated renders of the same axis do
/// not re-format unchanged ticks.
pub(super) fn format_tick(value: f64, formatter: Option<&fn(f64) -> String>) -> SharedString {
    let key = (value.to_bits(), formatter.copied());
    if let Some(label) = TICK_LABEL_CACHE.with(|cache| cache.borrow_mut().get(&key)) {
        return label;
    }
    // Custom formatters execute outside the borrow and may reenter rendering.
    let label = format_tick_uncached(value, formatter);
    TICK_LABEL_CACHE.with(|cache| cache.borrow_mut().insert(key, label.clone()));
    label
}

/// Drops cached tick labels on the current UI thread.
///
/// Call after changing global state used by a custom formatter, such as locale,
/// or when handling memory pressure. Formatter output must otherwise be stable.
pub(super) fn clear_tick_label_cache() {
    TICK_LABEL_CACHE.with(|cache| *cache.borrow_mut() = TickLabelCache::default());
}

fn format_tick_uncached(value: f64, formatter: Option<&fn(f64) -> String>) -> SharedString {
    match formatter {
        Some(f) => f(value).into(),
        None => {
            // Default formatting: remove trailing zeros
            if value.abs() < 1e-10 {
                "0".into()
            } else if value.abs() >= 1000.0 || value.abs() < 0.01 {
                format!("{value:.1e}").into()
            } else if value.fract().abs() < 1e-10 {
                format!("{value:.0}").into()
            } else {
                format!("{value:.1}").into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tick_default() {
        assert_eq!(format_tick(0.0, None), "0");
        assert_eq!(format_tick(10.0, None), "10");
        assert_eq!(format_tick(10.5, None), "10.5");
        assert_eq!(format_tick(1000.5, None), "1.0e3");
    }

    #[test]
    fn test_format_tick_custom() {
        let formatter: fn(f64) -> String = |v| format!("{v:.2}Hz");
        assert_eq!(format_tick(440.0, Some(&formatter)), "440.00Hz");
    }

    #[test]
    fn test_format_tick_cache_reuses_shared_string() {
        // Repeated calls with the same arguments must return the same value.
        assert_eq!(format_tick(42.5, None), "42.5");
        assert_eq!(format_tick(42.5, None), "42.5");

        let formatter: fn(f64) -> String = |v| format!("{v:.0}%");
        assert_eq!(format_tick(99.0, Some(&formatter)), "99%");
        assert_eq!(format_tick(99.0, Some(&formatter)), "99%");
    }

    #[test]
    fn streaming_labels_and_oversized_formatters_have_bounded_retention() {
        clear_tick_label_cache();
        for value in 0..(MAX_LABELS * 10) {
            format_tick(value as f64, None);
        }
        let oversized: fn(f64) -> String = |_| "x".repeat(MAX_LABEL_BYTES + 1);
        format_tick(1.0, Some(&oversized));
        TICK_LABEL_CACHE.with(|cache| {
            let cache = cache.borrow();
            assert!(cache.labels.len() <= MAX_LABELS);
            assert!(cache.bytes <= MAX_LABEL_BYTES);
            assert_eq!(cache.order.len(), cache.labels.len());
        });
        clear_tick_label_cache();
        TICK_LABEL_CACHE.with(|cache| assert!(cache.borrow().labels.is_empty()));
    }

    #[test]
    fn warm_hits_reuse_storage_and_clear_invalidates_custom_formats() {
        clear_tick_label_cache();
        // SharedString stores short labels inline; use heap-backed output to
        // verify sharing rather than comparing addresses of inline copies.
        let long: fn(f64) -> String = |_| "shared label ".repeat(32);
        let first = format_tick(42.5, Some(&long));
        let second = format_tick(42.5, Some(&long));
        assert_eq!(first.as_ptr(), second.as_ptr());
        thread_local! { static GENERATION: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
        let formatter: fn(f64) -> String = |_| GENERATION.with(|g| g.get().to_string());
        assert_eq!(format_tick(1.0, Some(&formatter)), "0");
        GENERATION.with(|g| g.set(1));
        clear_tick_label_cache();
        assert_eq!(format_tick(1.0, Some(&formatter)), "1");
    }
}
