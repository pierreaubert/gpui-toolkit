//! Stable collection diff helpers for animated list and grid updates.

use std::collections::HashMap;

/// A single collection update operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionPatch<K> {
    Insert { key: K, index: usize },
    Delete { key: K, index: usize },
    Move { key: K, from: usize, to: usize },
    Update { key: K, index: usize },
}

/// Diff two ordered collections by stable identity.
///
/// The algorithm is intentionally small and deterministic. It favours clear
/// insert/delete/move/update patches for UI animation planning over minimal
/// edit-distance output.
///
/// Runs in `O(n + m)` time by hashing the keys of the old collection.
pub fn diff_by_key<T, K, F>(old: &[T], new: &[T], key: F) -> Vec<CollectionPatch<K>>
where
    K: Clone + Eq + std::hash::Hash,
    F: Fn(&T) -> K,
    T: PartialEq,
{
    let old_keys: Vec<K> = old.iter().map(&key).collect();
    let new_keys: Vec<K> = new.iter().map(&key).collect();

    let mut old_index_by_key: HashMap<&K, usize> = HashMap::with_capacity(old_keys.len());
    for (index, old_key) in old_keys.iter().enumerate() {
        old_index_by_key.insert(old_key, index);
    }

    let mut new_index_by_key: HashMap<&K, usize> = HashMap::with_capacity(new_keys.len());
    for (index, new_key) in new_keys.iter().enumerate() {
        new_index_by_key.insert(new_key, index);
    }

    let mut patches = Vec::new();

    for (old_index, old_key) in old_keys.iter().enumerate() {
        if !new_index_by_key.contains_key(old_key) {
            patches.push(CollectionPatch::Delete {
                key: old_key.clone(),
                index: old_index,
            });
        }
    }

    for (new_index, new_key) in new_keys.iter().enumerate() {
        match old_index_by_key.get(new_key) {
            None => patches.push(CollectionPatch::Insert {
                key: new_key.clone(),
                index: new_index,
            }),
            Some(&old_index) => {
                if old_index != new_index {
                    patches.push(CollectionPatch::Move {
                        key: new_key.clone(),
                        from: old_index,
                        to: new_index,
                    });
                }
                if old[old_index] != new[new_index] {
                    patches.push(CollectionPatch::Update {
                        key: new_key.clone(),
                        index: new_index,
                    });
                }
            }
        }
    }

    patches
}

/// Return true when patches only change item content, not order or membership.
pub fn is_content_only_update<K>(patches: &[CollectionPatch<K>]) -> bool {
    patches
        .iter()
        .all(|patch| matches!(patch, CollectionPatch::Update { .. }))
}

#[cfg(test)]
mod tests {
    use super::{CollectionPatch, diff_by_key, is_content_only_update};

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Row {
        id: u32,
        title: &'static str,
    }

    #[test]
    fn diff_reports_insert_delete_move_and_update() {
        let old = [
            Row { id: 1, title: "a" },
            Row { id: 2, title: "b" },
            Row { id: 3, title: "c" },
        ];
        let new = [
            Row { id: 2, title: "b2" },
            Row { id: 4, title: "d" },
            Row { id: 1, title: "a" },
        ];

        let patches = diff_by_key(&old, &new, |row| row.id);

        assert!(patches.contains(&CollectionPatch::Delete { key: 3, index: 2 }));
        assert!(patches.contains(&CollectionPatch::Insert { key: 4, index: 1 }));
        assert!(patches.contains(&CollectionPatch::Move {
            key: 2,
            from: 1,
            to: 0,
        }));
        assert!(patches.contains(&CollectionPatch::Update { key: 2, index: 0 }));
        assert!(!is_content_only_update(&patches));
    }

    #[test]
    fn diff_empty_and_identical_collections() {
        let rows = [Row { id: 1, title: "a" }, Row { id: 2, title: "b" }];
        assert!(diff_by_key::<_, u32, _>(&[], &[], |row: &Row| row.id).is_empty());
        assert!(diff_by_key(&rows, &rows, |row| row.id).is_empty());
        assert!(is_content_only_update(&diff_by_key(&rows, &rows, |row| {
            row.id
        })));
    }
}
