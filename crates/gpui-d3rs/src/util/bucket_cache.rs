//! Byte-budgeted hash-bucket cache used by the glyph paint paths.

use std::collections::{HashMap, VecDeque};

/// Retained-byte and activity counters for one glyph cache.
#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GlyphCacheStats {
    pub entries: usize,
    pub retained_bytes: usize,
    pub high_water_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

/// Byte-budgeted bucket cache behind the glyph paint paths.
///
/// Keys stay hash-bucketed with linear probing inside a bucket (the
/// borrowed-lookup fast path), but retention is bounded in entries
/// *and* charged bytes with bucket-granular LRU eviction instead of
/// the previous whole-cache clear at 512 buckets, whose payload was
/// unbounded (one huge raster could pin hundreds of MiB).
pub(crate) struct BucketCache<K, V> {
    pub(crate) buckets: HashMap<u64, Vec<(K, V)>>,
    order: VecDeque<u64>,
    entries: usize,
    retained_bytes: usize,
    high_water_bytes: usize,
    pub(crate) hits: u64,
    pub(crate) misses: u64,
    evictions: u64,
    capacity: usize,
    max_bytes: usize,
}

impl<K, V> Default for BucketCache<K, V> {
    fn default() -> Self {
        Self {
            buckets: HashMap::new(),
            order: std::collections::VecDeque::new(),
            entries: 0,
            retained_bytes: 0,
            high_water_bytes: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            capacity: 0,
            max_bytes: 0,
        }
    }
}

impl<K, V> BucketCache<K, V> {
    pub(crate) fn with_budgets(capacity: usize, max_bytes: usize) -> Self {
        Self {
            capacity,
            max_bytes,
            ..Self::default()
        }
    }

    #[cfg(test)]
    pub(crate) fn clear(&mut self) {
        self.buckets.clear();
        self.order.clear();
        self.entries = 0;
        self.retained_bytes = 0;
    }

    #[cfg(test)]
    pub(crate) fn stats(&self) -> GlyphCacheStats {
        GlyphCacheStats {
            entries: self.entries,
            retained_bytes: self.retained_bytes,
            high_water_bytes: self.high_water_bytes,
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
        }
    }

    #[cfg(test)]
    pub(crate) fn trim(&mut self) {
        self.clear();
        self.buckets.shrink_to_fit();
        self.order.shrink_to_fit();
    }

    fn evict_bucket(&mut self, hash: u64, charge: impl Fn(&K, &V) -> usize) {
        if let Some(bucket) = self.buckets.remove(&hash) {
            for (key, value) in &bucket {
                self.retained_bytes = self.retained_bytes.saturating_sub(charge(key, value));
                self.entries = self.entries.saturating_sub(1);
                self.evictions = self.evictions.saturating_add(1);
            }
        }
        self.order.retain(|candidate| *candidate != hash);
    }

    pub(crate) fn insert(
        &mut self,
        hash: u64,
        key: K,
        value: V,
        charge: usize,
        charge_of: impl Fn(&K, &V) -> usize,
    ) {
        if charge > self.max_bytes {
            self.evictions = self.evictions.saturating_add(1);
            return;
        }
        while !self.buckets.is_empty()
            && (self.entries >= self.capacity
                || self.retained_bytes.saturating_add(charge) > self.max_bytes)
        {
            let Some(victim) = self.order.front().copied() else {
                break;
            };
            if victim == hash && self.order.len() == 1 {
                break;
            }
            self.evict_bucket(victim, &charge_of);
        }
        self.retained_bytes = self.retained_bytes.saturating_add(charge);
        self.high_water_bytes = self.high_water_bytes.max(self.retained_bytes);
        self.entries = self.entries.saturating_add(1);
        if !self.buckets.contains_key(&hash) {
            self.order.push_back(hash);
        }
        self.buckets.entry(hash).or_default().push((key, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_cache_plateaus_in_entries_and_bytes() {
        fn charge(key: &u64, value: &usize) -> usize {
            let _ = key;
            *value
        }
        let mut cache: BucketCache<u64, usize> = BucketCache::with_budgets(8, 1000);
        for id in 0..100u64 {
            cache.insert(id, id, 100, 100, charge);
        }
        let stats = cache.stats();
        assert!(stats.entries <= 8, "{stats:?}");
        assert!(stats.retained_bytes <= 1000, "{stats:?}");
        assert!(stats.evictions > 0, "{stats:?}");
        // Oversized single entry bypasses retention.
        cache.insert(10_000, 10_000, 5000, 5000, charge);
        assert_eq!(cache.stats().entries, stats.entries);
        cache.trim();
        assert_eq!(cache.stats().retained_bytes, 0);
    }
}
