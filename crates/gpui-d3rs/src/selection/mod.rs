//! Renderer-independent keyed data joins (d3-selection inspired).
//!
//! GPUI does not expose a DOM selection API. This module provides the stable
//! part of `d3-selection` that chart renderers still need: deterministic
//! enter/update/exit classification for keyed or index-based data joins.

use std::collections::HashMap;
use std::hash::Hash;

/// One newly-entered datum in a selection join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionEnter<K> {
    pub key: K,
    pub new_index: usize,
}

/// One datum that matches an existing keyed item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionUpdate<K> {
    pub key: K,
    pub old_index: usize,
    pub new_index: usize,
}

/// One old datum that no longer has a matching key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionExit<K> {
    pub key: K,
    pub old_index: usize,
}

/// Result of a keyed or index-based selection join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionJoin<K> {
    enter: Vec<SelectionEnter<K>>,
    update: Vec<SelectionUpdate<K>>,
    exit: Vec<SelectionExit<K>>,
}

impl<K> SelectionJoin<K> {
    /// Items present in the new data but absent from the old data.
    pub fn enter(&self) -> &[SelectionEnter<K>] {
        &self.enter
    }

    /// Items present in both old and new data.
    pub fn update(&self) -> &[SelectionUpdate<K>] {
        &self.update
    }

    /// Items present in the old data but absent from the new data.
    pub fn exit(&self) -> &[SelectionExit<K>] {
        &self.exit
    }

    /// Total number of join rows across enter/update/exit buckets.
    pub fn len(&self) -> usize {
        self.enter.len() + self.update.len() + self.exit.len()
    }

    /// Whether the join has no rows.
    pub fn is_empty(&self) -> bool {
        self.enter.is_empty() && self.update.is_empty() && self.exit.is_empty()
    }

    /// Whether the join contains entered or exited items.
    pub fn has_structural_changes(&self) -> bool {
        !self.enter.is_empty() || !self.exit.is_empty()
    }
}

/// Recoverable data-join errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionJoinError<K> {
    DuplicateOldKey {
        key: K,
        first_index: usize,
        duplicate_index: usize,
    },
    DuplicateNewKey {
        key: K,
        first_index: usize,
        duplicate_index: usize,
    },
}

/// Join old and new data by stable keys.
///
/// Updates and enters preserve new-data order; exits preserve old-data order.
/// Duplicate keys are rejected before returning a join so host renderers can
/// avoid ambiguous node/data ownership.
pub fn keyed_data_join<K, Old, New, OldKey, NewKey>(
    old_data: &[Old],
    new_data: &[New],
    mut old_key: OldKey,
    mut new_key: NewKey,
) -> Result<SelectionJoin<K>, SelectionJoinError<K>>
where
    K: Clone + Eq + Hash,
    OldKey: FnMut(&Old, usize) -> K,
    NewKey: FnMut(&New, usize) -> K,
{
    let mut old_by_key = HashMap::with_capacity(old_data.len());
    let mut old_keys = Vec::with_capacity(old_data.len());

    for (old_index, datum) in old_data.iter().enumerate() {
        let key = old_key(datum, old_index);
        if let Some(first_index) = old_by_key.insert(key.clone(), old_index) {
            return Err(SelectionJoinError::DuplicateOldKey {
                key,
                first_index,
                duplicate_index: old_index,
            });
        }
        old_keys.push(key);
    }

    let mut seen_new = HashMap::with_capacity(new_data.len());
    let mut matched_old = vec![false; old_data.len()];
    let mut enter = Vec::new();
    let mut update = Vec::new();

    for (new_index, datum) in new_data.iter().enumerate() {
        let key = new_key(datum, new_index);
        if let Some(first_index) = seen_new.insert(key.clone(), new_index) {
            return Err(SelectionJoinError::DuplicateNewKey {
                key,
                first_index,
                duplicate_index: new_index,
            });
        }

        if let Some(&old_index) = old_by_key.get(&key) {
            matched_old[old_index] = true;
            update.push(SelectionUpdate {
                key,
                old_index,
                new_index,
            });
        } else {
            enter.push(SelectionEnter { key, new_index });
        }
    }

    let mut exit = Vec::new();
    for (old_index, key) in old_keys.into_iter().enumerate() {
        if !matched_old[old_index] {
            exit.push(SelectionExit { key, old_index });
        }
    }

    Ok(SelectionJoin {
        enter,
        update,
        exit,
    })
}

/// Join old and new data by index, matching D3's default unkeyed data join.
pub fn index_data_join(old_len: usize, new_len: usize) -> SelectionJoin<usize> {
    let shared = old_len.min(new_len);
    let update = (0..shared)
        .map(|index| SelectionUpdate {
            key: index,
            old_index: index,
            new_index: index,
        })
        .collect();
    let enter = (shared..new_len)
        .map(|new_index| SelectionEnter {
            key: new_index,
            new_index,
        })
        .collect();
    let exit = (shared..old_len)
        .map(|old_index| SelectionExit {
            key: old_index,
            old_index,
        })
        .collect();

    SelectionJoin {
        enter,
        update,
        exit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct Row {
        id: &'static str,
        value: i32,
    }

    #[test]
    fn keyed_data_join_classifies_enter_update_and_exit() {
        let old = [
            Row { id: "a", value: 1 },
            Row { id: "b", value: 2 },
            Row { id: "c", value: 3 },
        ];
        let new = [
            Row { id: "b", value: 20 },
            Row { id: "d", value: 4 },
            Row { id: "a", value: 10 },
        ];

        let join = keyed_data_join(&old, &new, |row, _| row.id, |row, _| row.id).unwrap();

        assert_eq!(
            join.update(),
            &[
                SelectionUpdate {
                    key: "b",
                    old_index: 1,
                    new_index: 0,
                },
                SelectionUpdate {
                    key: "a",
                    old_index: 0,
                    new_index: 2,
                },
            ]
        );
        assert_eq!(
            join.enter(),
            &[SelectionEnter {
                key: "d",
                new_index: 1,
            }]
        );
        assert_eq!(
            join.exit(),
            &[SelectionExit {
                key: "c",
                old_index: 2,
            }]
        );
        assert!(join.has_structural_changes());
        assert_eq!(join.len(), 4);
    }

    #[test]
    fn keyed_data_join_rejects_duplicate_old_keys() {
        let old = [Row { id: "a", value: 1 }, Row { id: "a", value: 2 }];
        let new = [Row { id: "a", value: 3 }];

        assert_eq!(
            keyed_data_join(&old, &new, |row, _| row.id, |row, _| row.id),
            Err(SelectionJoinError::DuplicateOldKey {
                key: "a",
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn keyed_data_join_rejects_duplicate_new_keys() {
        let old = [Row { id: "a", value: 1 }];
        let new = [Row { id: "b", value: 2 }, Row { id: "b", value: 3 }];

        assert_eq!(
            keyed_data_join(&old, &new, |row, _| row.id, |row, _| row.id),
            Err(SelectionJoinError::DuplicateNewKey {
                key: "b",
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn index_data_join_matches_by_position() {
        let grow = index_data_join(2, 4);
        assert_eq!(grow.update().len(), 2);
        assert_eq!(
            grow.enter(),
            &[
                SelectionEnter {
                    key: 2,
                    new_index: 2
                },
                SelectionEnter {
                    key: 3,
                    new_index: 3
                },
            ]
        );
        assert!(grow.exit().is_empty());

        let shrink = index_data_join(4, 2);
        assert_eq!(shrink.update().len(), 2);
        assert_eq!(
            shrink.exit(),
            &[
                SelectionExit {
                    key: 2,
                    old_index: 2
                },
                SelectionExit {
                    key: 3,
                    old_index: 3
                },
            ]
        );
        assert!(shrink.enter().is_empty());
    }

    #[test]
    fn stable_keyed_join_has_no_structural_changes() {
        let old = [Row { id: "a", value: 1 }];
        let new = [Row { id: "a", value: 2 }];
        let join = keyed_data_join(&old, &new, |row, _| row.id, |row, _| row.id).unwrap();

        assert_eq!(join.update().len(), 1);
        assert!(!join.has_structural_changes());
        assert!(!join.is_empty());
    }
}
