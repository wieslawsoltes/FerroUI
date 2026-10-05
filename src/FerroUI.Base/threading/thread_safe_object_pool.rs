use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

/// A pool of reusable objects that can be shared between threads.
pub struct ThreadSafeObjectPool<T> {
    stack: Mutex<Vec<T>>,
}

impl<T: Default> Default for ThreadSafeObjectPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Default> ThreadSafeObjectPool<T> {
    pub fn new() -> Self {
        Self { stack: Mutex::new(Vec::new()) }
    }

    /// Takes an object out of the pool, creating one when the pool is empty.
    pub fn get(&self) -> T {
        let pooled = self.stack.lock().unwrap_or_else(PoisonError::into_inner).pop();
        pooled.unwrap_or_default()
    }

    /// Puts the object back into the pool and clears the slot it came from.
    pub fn return_and_set_null(&self, obj: &mut Option<T>) {
        let Some(obj) = obj.take() else {
            return;
        };
        self.stack.lock().unwrap_or_else(PoisonError::into_inner).push(obj);
    }
}

static DEFAULT_POOLS: Mutex<Option<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>> = Mutex::new(None);

impl<T: Default + Send + 'static> ThreadSafeObjectPool<T> {
    /// The process-wide pool for `T`.
    pub fn default_pool() -> Arc<ThreadSafeObjectPool<T>> {
        let mut pools = DEFAULT_POOLS.lock().unwrap_or_else(PoisonError::into_inner);
        let pool = pools
            .get_or_insert_with(HashMap::new)
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Arc::new(ThreadSafeObjectPool::<T>::new()))
            .clone();
        drop(pools);
        match pool.downcast::<ThreadSafeObjectPool<T>>() {
            Ok(pool) => pool,
            Err(_) => unreachable!("the pool is keyed by its element type"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Debug, PartialEq)]
    struct Buffer(Vec<u8>);

    #[test]
    fn get_creates_and_reuses() {
        let pool = ThreadSafeObjectPool::<Buffer>::new();
        let mut first = Some(pool.get());
        assert_eq!(first, Some(Buffer(Vec::new())));
        first.as_mut().unwrap().0.push(7);

        pool.return_and_set_null(&mut first);
        assert!(first.is_none());
        // Returning an empty slot does nothing.
        pool.return_and_set_null(&mut first);

        assert_eq!(pool.get(), Buffer(vec![7]));
        assert_eq!(pool.get(), Buffer(Vec::new()));
    }

    #[test]
    fn default_pool_is_shared_per_type_across_threads() {
        #[derive(Default)]
        struct Marker(u32);

        let pool = ThreadSafeObjectPool::<Marker>::default_pool();
        let mut slot = Some(Marker(42));
        pool.return_and_set_null(&mut slot);

        let value = std::thread::spawn(|| ThreadSafeObjectPool::<Marker>::default_pool().get().0).join().unwrap();
        assert_eq!(value, 42);
        assert!(Arc::ptr_eq(&pool, &ThreadSafeObjectPool::<Marker>::default_pool()));
    }
}
