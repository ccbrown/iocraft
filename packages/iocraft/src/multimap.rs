use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
};

/// This is a specialized map implementation which is optimized for tracking components across updates:
///
/// During updates, components are created and appended to the `AppendOnlyMultimap`. Once the
/// update is complete, the map is converted to a `RemoveOnlyMultimap`, which can be iterated in
/// insertion order. During the next update, components are removed from the map based on their key
/// in order to be recycled. If multiple elements have duplicate keys, they're recycled in the same
/// order they were first inserted.
///
/// All operations have O(1) time complexity.
pub(crate) struct AppendOnlyMultimap<K, V> {
    items: Vec<Option<V>>,
    m: HashMap<K, VecDeque<usize>>,
}

impl<K, V> Default for AppendOnlyMultimap<K, V> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            m: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq, V> AppendOnlyMultimap<K, V> {
    pub fn push_back(&mut self, key: K, value: V) {
        let index = self.items.len();
        self.items.push(Some(value));
        self.m.entry(key).or_default().push_back(index);
    }
}

pub(crate) struct RemoveOnlyMultimap<K, V> {
    items: Vec<Option<V>>,
    m: HashMap<K, VecDeque<usize>>,
}

impl<K, V> Default for RemoveOnlyMultimap<K, V> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            m: HashMap::new(),
        }
    }
}

impl<K: Hash + Eq, V> RemoveOnlyMultimap<K, V> {
    pub fn pop_front(&mut self, key: &K) -> Option<V> {
        let index = self.m.get_mut(key)?.pop_front()?;
        self.items[index].take()
    }

    pub fn iter(&self) -> impl Iterator<Item = &V> {
        self.items.iter().filter_map(|item| item.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.items.iter_mut().filter_map(|item| item.as_mut())
    }
}

impl<K, V> From<AppendOnlyMultimap<K, V>> for RemoveOnlyMultimap<K, V> {
    fn from(multimap: AppendOnlyMultimap<K, V>) -> Self {
        Self {
            items: multimap.items,
            m: multimap.m,
        }
    }
}
