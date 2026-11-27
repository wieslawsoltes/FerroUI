//! Shared state-object members with native ownership and deterministic wire order.
//! Only Labels and GroupOrder consume this string-keyed representation.
use crate::RibbonString;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Mutable members of a persisted ribbon state object. Clone its `Rc` to share
/// changes; use [`Rc::ptr_eq`] for identity. Snapshots own their values.
///
/// Removed member positions are reused for source-compatible JSON output. This
/// is a state-format boundary, not a general Dictionary/enumerator implementation.
pub struct RibbonStateEntries<V> {
    data: RefCell<StateMembers<V>>,
}
struct StateMembers<V> {
    values: HashMap<RibbonString, (usize, V)>,
    order: Vec<Option<RibbonString>>,
    vacant: Vec<usize>,
}
impl<V> Default for StateMembers<V> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            order: Vec::new(),
            vacant: Vec::new(),
        }
    }
}
impl<V> Default for RibbonStateEntries<V> {
    fn default() -> Self {
        Self {
            data: RefCell::default(),
        }
    }
}
impl<V> PartialEq for RibbonStateEntries<V> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<V> Eq for RibbonStateEntries<V> {}
impl<V> RibbonStateEntries<V> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    pub fn len(&self) -> usize {
        self.data.borrow().values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.borrow().values.is_empty()
    }
    pub fn contains_key(&self, key: &RibbonString) -> bool {
        self.data.borrow().values.contains_key(key)
    }
    pub fn insert(&self, key: RibbonString, value: V) -> Option<V> {
        let mut data = self.data.borrow_mut();
        if let Some((_, current)) = data.values.get_mut(&key) {
            return Some(std::mem::replace(current, value));
        }
        let index = if let Some(index) = data.vacant.pop() {
            data.order[index] = Some(key.clone());
            index
        } else {
            let index = data.order.len();
            data.order.push(Some(key.clone()));
            index
        };
        data.values.insert(key, (index, value));
        None
    }
    pub fn remove(&self, key: &RibbonString) -> Option<V> {
        let (stored_key, ordered_key, value) = {
            let mut data = self.data.borrow_mut();
            let (stored_key, (index, value)) = data.values.remove_entry(key)?;
            let ordered_key = data.order[index].take();
            data.vacant.push(index);
            (stored_key, ordered_key, value)
        };
        drop(stored_key);
        drop(ordered_key);
        Some(value)
    }
    pub fn clear(&self) {
        let old = std::mem::take(&mut *self.data.borrow_mut());
        drop(old);
    }
}
impl<V: Clone> RibbonStateEntries<V> {
    pub fn get(&self, key: &RibbonString) -> Option<V> {
        self.data
            .borrow()
            .values
            .get(key)
            .map(|(_, value)| value.clone())
    }
    pub fn entries_snapshot(&self) -> Vec<(RibbonString, V)> {
        let data = self.data.borrow();
        data.order
            .iter()
            .flatten()
            .map(|key| {
                (
                    key.clone(),
                    data.values
                        .get(key)
                        .expect("state member position")
                        .1
                        .clone(),
                )
            })
            .collect()
    }
}
