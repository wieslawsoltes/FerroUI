// SPDX-License-Identifier: MIT
//! Fixed-size mutable reference storage shared by UI-independent engines and native array views.
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, RwLock, Weak},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrayIndexOutOfRange;
impl std::fmt::Display for ArrayIndexOutOfRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Index was outside the bounds of the array.")
    }
}
impl std::error::Error for ArrayIndexOutOfRange {}
fn empty_identity<T: 'static>() -> Arc<()> {
    static VALUES: OnceLock<Mutex<HashMap<TypeId, Arc<()>>>> = OnceLock::new();
    VALUES
        .get_or_init(Default::default)
        .lock()
        .expect("array empty identity lock poisoned")
        .entry(TypeId::of::<T>())
        .or_default()
        .clone()
}
enum Storage<T> {
    Empty(Arc<()>),
    Values(Arc<RwLock<Box<[T]>>>),
}
/// Cloning preserves the original array allocation; an explicit empty array remains fresh.
pub struct SharedArray<T: 'static>(Storage<T>);
impl<T: 'static> Clone for SharedArray<T> {
    fn clone(&self) -> Self {
        Self(match &self.0 {
            Storage::Empty(v) => Storage::Empty(v.clone()),
            Storage::Values(v) => Storage::Values(v.clone()),
        })
    }
}
impl<T: 'static> Default for SharedArray<T> {
    fn default() -> Self {
        Self(Storage::Empty(empty_identity::<T>()))
    }
}
impl<T: 'static> PartialEq for SharedArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}
impl<T: 'static> Eq for SharedArray<T> {}
impl<T: 'static> SharedArray<T> {
    pub fn new(values: Vec<T>) -> Self {
        assert!(
            i32::try_from(values.len()).is_ok(),
            "array exceeds Int32 length"
        );
        Self(Storage::Values(Arc::new(RwLock::new(
            values.into_boxed_slice(),
        ))))
    }
    pub fn from_generated(values: Vec<T>) -> Self {
        if values.is_empty() {
            Self::default()
        } else {
            Self::new(values)
        }
    }
    pub fn len(&self) -> usize {
        match &self.0 {
            Storage::Empty(_) => 0,
            Storage::Values(v) => v.read().expect("array lock poisoned").len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Storage::Empty(a), Storage::Empty(b)) => Arc::ptr_eq(a, b),
            (Storage::Values(a), Storage::Values(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
    pub fn get(&self, index: i32) -> Result<T, ArrayIndexOutOfRange>
    where
        T: Clone,
    {
        let Storage::Values(values) = &self.0 else {
            return Err(ArrayIndexOutOfRange);
        };
        let values = values.read().expect("array lock poisoned");
        usize::try_from(index)
            .ok()
            .and_then(|i| values.get(i))
            .cloned()
            .ok_or(ArrayIndexOutOfRange)
    }
    pub fn set(&self, index: i32, value: T) -> Result<(), ArrayIndexOutOfRange> {
        let Storage::Values(values) = &self.0 else {
            return Err(ArrayIndexOutOfRange);
        };
        let mut values = values.write().expect("array lock poisoned");
        let slot = usize::try_from(index)
            .ok()
            .and_then(|i| values.get_mut(i))
            .ok_or(ArrayIndexOutOfRange)?;
        let old = std::mem::replace(slot, value);
        drop(values);
        drop(old);
        Ok(())
    }
    pub fn to_vec(&self) -> Vec<T>
    where
        T: Clone,
    {
        match &self.0 {
            Storage::Empty(_) => Vec::new(),
            Storage::Values(v) => v.read().expect("array lock poisoned").to_vec(),
        }
    }
    pub fn downgrade(&self) -> WeakSharedArray<T> {
        WeakSharedArray(match &self.0 {
            Storage::Empty(v) => WeakStorage::Empty(Arc::downgrade(v)),
            Storage::Values(v) => WeakStorage::Values(Arc::downgrade(v)),
        })
    }
}
enum WeakStorage<T> {
    Empty(Weak<()>),
    Values(Weak<RwLock<Box<[T]>>>),
}
pub struct WeakSharedArray<T: 'static>(WeakStorage<T>);
impl<T: 'static> WeakSharedArray<T> {
    pub fn upgrade(&self) -> Option<SharedArray<T>> {
        Some(SharedArray(match &self.0 {
            WeakStorage::Empty(v) => Storage::Empty(v.upgrade()?),
            WeakStorage::Values(v) => Storage::Values(v.upgrade()?),
        }))
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (WeakStorage::Empty(a), WeakStorage::Empty(b)) => a.ptr_eq(b),
            (WeakStorage::Values(a), WeakStorage::Values(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn alias_mutation_fixed_bounds_and_empty_identity() {
        let a = SharedArray::new(vec![97]);
        let b = a.clone();
        b.set(0, 98).unwrap();
        assert_eq!(a.get(0), Ok(98));
        assert!(a.ptr_eq(&b));
        assert_eq!(a.set(-1, 0), Err(ArrayIndexOutOfRange));
        assert_eq!(a.get(1), Err(ArrayIndexOutOfRange));
        let empty = SharedArray::<i32>::default();
        assert!(empty.ptr_eq(&SharedArray::default()));
        assert!(!empty.ptr_eq(&SharedArray::new(vec![])));
    }
    #[test]
    fn weak_owner_and_global_empty_lifetimes() {
        let owner = SharedArray::new(vec![1]);
        let weak = owner.downgrade();
        assert!(weak.upgrade().unwrap().ptr_eq(&owner));
        drop(owner);
        assert!(weak.upgrade().is_none());
        let weak = SharedArray::<i32>::default().downgrade();
        assert!(weak.upgrade().is_some());
    }
    #[test]
    fn replaced_element_drops_after_write_guard_is_released() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct Item {
            owner: Option<WeakSharedArray<Item>>,
            dropped: Arc<AtomicBool>,
        }
        let dropped = Arc::new(AtomicBool::new(false));
        impl Drop for Item {
            fn drop(&mut self) {
                if let Some(owner) = self.owner.as_ref().and_then(WeakSharedArray::upgrade) {
                    assert_eq!(owner.len(), 1);
                    self.dropped.store(true, Ordering::SeqCst);
                }
            }
        }
        let a = SharedArray::new(vec![Item {
            owner: None,
            dropped: dropped.clone(),
        }]);
        a.set(
            0,
            Item {
                owner: Some(a.downgrade()),
                dropped: dropped.clone(),
            },
        )
        .unwrap();
        a.set(
            0,
            Item {
                owner: None,
                dropped: dropped.clone(),
            },
        )
        .unwrap();
        assert!(dropped.load(Ordering::SeqCst));
    }
}
