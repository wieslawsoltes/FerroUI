//! `PriorityQueue<T>` (`Internal/PriorityQueue.cs`): a binary heap that keeps items of equal
//! priority in the order they were added, which the schedulers rely on for work due at the same
//! time.

use std::cmp::Ordering;

pub(crate) struct PriorityQueue<K: Ord, T> {
    items: Vec<Entry<K, T>>,
    next_id: u64,
}

struct Entry<K, T> {
    key: K,
    id: u64,
    value: T,
}

impl<K: Ord, T> Entry<K, T> {
    fn cmp_order(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key).then(self.id.cmp(&other.id))
    }
}

impl<K: Ord, T> Default for PriorityQueue<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, T> PriorityQueue<K, T> {
    pub(crate) fn new() -> Self {
        Self { items: Vec::new(), next_id: 0 }
    }

    pub(crate) fn len(&self) -> usize {
        self.items.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub(crate) fn push(&mut self, key: K, value: T) {
        let id = self.next_id;
        self.next_id += 1;
        self.items.push(Entry { key, id, value });
        let mut index = self.items.len() - 1;
        while index > 0 {
            let parent = (index - 1) / 2;
            if self.items[index].cmp_order(&self.items[parent]) == Ordering::Less {
                self.items.swap(index, parent);
                index = parent;
            } else {
                break;
            }
        }
    }

    pub(crate) fn peek(&self) -> Option<(&K, &T)> {
        self.items.first().map(|e| (&e.key, &e.value))
    }

    pub(crate) fn pop(&mut self) -> Option<(K, T)> {
        if self.items.is_empty() {
            return None;
        }
        let last = self.items.len() - 1;
        self.items.swap(0, last);
        let entry = self.items.pop().expect("a non-empty queue");
        let mut index = 0;
        loop {
            let left = 2 * index + 1;
            let right = left + 1;
            let mut smallest = index;
            if left < self.items.len() && self.items[left].cmp_order(&self.items[smallest]) == Ordering::Less {
                smallest = left;
            }
            if right < self.items.len() && self.items[right].cmp_order(&self.items[smallest]) == Ordering::Less {
                smallest = right;
            }
            if smallest == index {
                break;
            }
            self.items.swap(index, smallest);
            index = smallest;
        }
        Some((entry.key, entry.value))
    }

    /// Removes the first item for which `matches` is true (`Remove`); false when there is none.
    pub(crate) fn remove_where(&mut self, mut matches: impl FnMut(&T) -> bool) -> bool {
        let Some(index) = self.items.iter().position(|e| matches(&e.value)) else {
            return false;
        };
        let removed = self.items.remove(index);
        drop(removed);
        // Rebuild the heap; removal is rare (cancelled virtual-time work is skipped instead).
        let mut items = std::mem::take(&mut self.items);
        items.sort_by(|a, b| a.cmp_order(b));
        self.items = items;
        true
    }
}
