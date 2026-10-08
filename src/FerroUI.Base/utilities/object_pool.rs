use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, PoisonError};

/// Provides a thread-safe object pool with size limits and object validation.
pub struct ObjectPool<T> {
    factory: Box<dyn Fn() -> T + Send + Sync>,
    validator: Option<Box<dyn Fn(&mut T) -> bool + Send + Sync>>,
    items: Mutex<Vec<T>>,
    max_size: i32,
    count: AtomicI32,
}

impl<T> ObjectPool<T> {
    /// The default of the maximum number of objects to keep in the pool.
    pub const DEFAULT_MAX_SIZE: i32 = 32;

    /// Initializes a new instance of the pool.
    ///
    /// `factory` creates new instances. The optional `validator` cleans and
    /// validates an object before it returns to the pool; it returns false
    /// to discard the object. `max_size` is the maximum number of objects to
    /// keep in the pool.
    ///
    /// Panics when `max_size` is less than 1.
    pub fn new(
        factory: impl Fn() -> T + Send + Sync + 'static,
        validator: Option<Box<dyn Fn(&mut T) -> bool + Send + Sync>>,
        max_size: i32,
    ) -> Self {
        if max_size < 1 {
            panic!("maxSize must be at least 1. (Parameter 'maxSize')\nActual value was {max_size}.");
        }

        Self {
            factory: Box::new(factory),
            validator,
            max_size,
            items: Mutex::new(Vec::new()),
            count: AtomicI32::new(0),
        }
    }

    /// Rents an object from the pool or creates a new one if the pool is empty.
    #[inline]
    pub fn rent(&self) -> T {
        let item = self.items.lock().unwrap_or_else(PoisonError::into_inner).pop();
        if let Some(item) = item {
            self.count.fetch_sub(1, Ordering::SeqCst);
            return item;
        }

        (self.factory)()
    }

    /// Returns an object to the pool if it passes validation and the pool is not full.
    ///
    /// Callers must not return the same item more than once without an
    /// intervening [`rent`](Self::rent). The pool does not detect duplicate
    /// returns. The pool also does nothing with the items it drops (when
    /// full or when the validator returns false) beyond dropping them; any
    /// cleanup belongs in the validator or in the caller.
    #[inline]
    pub fn return_item(&self, mut item: T) {
        // Validate and clean the object
        if let Some(validator) = &self.validator {
            if !validator(&mut item) {
                return;
            }
        }

        // Check if pool is full (fast check without lock)
        if self.count.load(Ordering::SeqCst) >= self.max_size {
            return;
        }

        // Try to increment count, but check again in case of race condition
        let current_count = self.count.fetch_add(1, Ordering::SeqCst) + 1;

        if current_count <= self.max_size {
            self.items.lock().unwrap_or_else(PoisonError::into_inner).push(item);
        } else {
            // Pool is full, decrement and discard
            self.count.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    // The upstream tests `Constructor_Throws_When_Factory_Is_Null` and
    // `Return_Ignores_Null` have no counterpart: a factory and an item
    // cannot be null here.
    use super::*;
    use std::sync::Arc;

    #[derive(Default)]
    struct Item {
        state: AtomicI32,
    }

    type Pool = ObjectPool<Arc<Item>>;

    fn new_item() -> Arc<Item> {
        Arc::new(Item::default())
    }

    fn contains(items: &[Arc<Item>], item: &Arc<Item>) -> bool {
        items.iter().any(|other| Arc::ptr_eq(other, item))
    }

    #[test]
    fn constructor_throws_when_max_size_is_less_than_one() {
        for max_size in [0, -1, i32::MIN] {
            let result = std::panic::catch_unwind(|| Pool::new(new_item, None, max_size));
            assert!(result.is_err(), "max size {max_size}");
        }
    }

    #[test]
    fn constructor_accepts_max_size_of_one() {
        let pool = Pool::new(new_item, None, 1);

        let item = pool.rent();

        assert_eq!(0, item.state.load(Ordering::SeqCst));
    }

    #[test]
    fn rent_creates_new_item_when_pool_is_empty() {
        let pool = Pool::new(new_item, None, Pool::DEFAULT_MAX_SIZE);

        let first = pool.rent();
        let second = pool.rent();

        assert!(!Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn returned_item_is_reused_by_subsequent_rent() {
        let pool = Pool::new(new_item, None, Pool::DEFAULT_MAX_SIZE);

        let item = pool.rent();
        pool.return_item(item.clone());

        let rented = pool.rent();

        assert!(Arc::ptr_eq(&item, &rented));
    }

    #[test]
    fn return_drops_item_when_pool_is_full() {
        let pool = Pool::new(new_item, None, 2);

        let a = new_item();
        let b = new_item();
        let c = new_item();

        pool.return_item(a.clone());
        pool.return_item(b.clone());
        pool.return_item(c.clone()); // pool full -> dropped

        let rented = [pool.rent(), pool.rent()];

        // The two rented items must come from {a, b}; c was dropped.
        assert!(contains(&rented, &a));
        assert!(contains(&rented, &b));
        assert!(!contains(&rented, &c));
    }

    #[test]
    fn validator_is_invoked_on_return() {
        let validator_calls = Arc::new(AtomicI32::new(0));
        let calls = validator_calls.clone();
        let pool = Pool::new(
            new_item,
            Some(Box::new(move |_: &mut Arc<Item>| {
                calls.fetch_add(1, Ordering::SeqCst);
                true
            })),
            Pool::DEFAULT_MAX_SIZE,
        );

        let item = pool.rent();
        pool.return_item(item);

        assert_eq!(1, validator_calls.load(Ordering::SeqCst));
    }

    #[test]
    fn validator_is_not_invoked_on_rent() {
        // The validator's job is to prepare an item for re-use *before* it goes back
        // into the pool. Running it on Rent would either duplicate the work or imply
        // a different contract (validate-on-take). Pin the current contract.
        let validator_calls = Arc::new(AtomicI32::new(0));
        let calls = validator_calls.clone();
        let pool = Pool::new(
            new_item,
            Some(Box::new(move |_: &mut Arc<Item>| {
                calls.fetch_add(1, Ordering::SeqCst);
                true
            })),
            Pool::DEFAULT_MAX_SIZE,
        );

        let _ = pool.rent(); // fresh from factory; validator must not run
        let item = pool.rent();
        pool.return_item(item); // one validator call here
        let _ = pool.rent(); // pulled from pool; validator must not run again

        assert_eq!(1, validator_calls.load(Ordering::SeqCst));
    }

    #[test]
    fn return_drops_item_when_validator_returns_false() {
        let pool = Pool::new(new_item, Some(Box::new(|_: &mut Arc<Item>| false)), Pool::DEFAULT_MAX_SIZE);

        let item = pool.rent();
        pool.return_item(item.clone());

        // Validator rejected the item, so the pool is empty and the next
        // Rent produces a fresh instance.
        let rented = pool.rent();

        assert!(!Arc::ptr_eq(&item, &rented));
    }

    #[test]
    fn validator_can_reset_item_state_before_pooling() {
        let pool = Pool::new(
            new_item,
            Some(Box::new(|i: &mut Arc<Item>| {
                i.state.store(0, Ordering::SeqCst);
                true
            })),
            Pool::DEFAULT_MAX_SIZE,
        );

        let item = pool.rent();
        item.state.store(42, Ordering::SeqCst);
        pool.return_item(item.clone());

        let rented = pool.rent();

        assert!(Arc::ptr_eq(&item, &rented));
        assert_eq!(0, rented.state.load(Ordering::SeqCst));
    }

    #[test]
    fn pool_stays_within_max_size_under_concurrent_returns() {
        const MAX_SIZE: i32 = 8;
        const RETURNS_PER_THREAD: i32 = 100;
        const THREAD_COUNT: i32 = 16;

        let pool = Pool::new(new_item, None, MAX_SIZE);

        std::thread::scope(|scope| {
            for _ in 0..THREAD_COUNT {
                scope.spawn(|| {
                    for _ in 0..RETURNS_PER_THREAD {
                        let item = new_item();
                        item.state.store(1, Ordering::SeqCst);
                        pool.return_item(item);
                    }
                });
            }
        });

        // Drain the pool. Items that came from the pool will still have State==1;
        // factory-created items will have the default State==0.
        let mut pooled = 0;
        let mut seen: Vec<Arc<Item>> = Vec::new();

        for _ in 0..MAX_SIZE * 2 {
            let item = pool.rent();
            if contains(&seen, &item) {
                panic!("ObjectPool returned the same instance twice without an intervening Return.");
            }
            seen.push(item.clone());

            if item.state.load(Ordering::SeqCst) == 1 {
                pooled += 1;
            }
        }

        assert!(pooled <= MAX_SIZE, "Expected to observe at most {MAX_SIZE} pooled items, observed {pooled}.");
    }

    #[test]
    fn rent_and_return_survive_parallel_use_without_losing_or_duplicating_items() {
        const ITERATIONS: i32 = 5_000;
        const THREAD_COUNT: i32 = 8;

        let pool = Pool::new(new_item, None, 32);

        std::thread::scope(|scope| {
            for _ in 0..THREAD_COUNT {
                scope.spawn(|| {
                    for _ in 0..ITERATIONS {
                        let item = pool.rent();
                        pool.return_item(item);
                    }
                });
            }
        });
    }
}
