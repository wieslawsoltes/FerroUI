use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::{Mutex, MutexGuard, PoisonError};

const USAGE_STATISTICS_LENGTH: usize = 10;

/// How the items of a [`BatchStreamPoolBase`] are created, cleared and
/// destroyed (the abstract and virtual members of the C# base class).
pub trait IBatchStreamPoolItems<T>: Send + Sync {
    /// Creates a new item; called when the pool has none to hand out.
    fn create_item(&self) -> T;

    /// Clears an item that is being returned to the pool.
    fn clear_item(&self, _item: &mut T) {}

    /// Destroys an item the pool does not keep.
    fn destroy_item(&self, _item: T) {}
}

/// The callback that asks the owner of a pool to start its update timer.
pub type BatchStreamPoolStartTimer = Box<dyn Fn() + Send + Sync>;

struct PoolState<T> {
    pool: Vec<T>,
    disposed: bool,
    usage: i32,
    usage_statistics: [i32; USAGE_STATISTICS_LENGTH],
    usage_statistics_slot: usize,
    timer_is_running: bool,
    current_update_tick: u64,
    last_activity_tick: u64,
}

/// A pool that keeps a number of elements that was used in the last 10 seconds
///
/// # Trimming
///
/// The C# class trims itself from a one second timer that it starts on the
/// dispatcher of the thread it was created on (or through a `startTimer`
/// delegate that receives the timer procedure). This port has no timer of
/// its own; the owner drives it instead:
///
/// * while [`timer_is_running`](Self::timer_is_running) is `true`, the owner
///   calls [`update_timer_tick`](Self::update_timer_tick) once per second;
/// * when `update_timer_tick` returns `false` the timer has stopped (the
///   pool saw no activity for 21 ticks) and the owner stops calling it;
/// * the timer starts again on the next [`get`](Self::get) or
///   [`return_item`](Self::return_item). The optional `start_timer` callback
///   given to the constructor is invoked each time that happens (and once
///   from the constructor), so the owner does not have to poll
///   `timer_is_running`. The callback is invoked on the thread that used the
///   pool, outside of the pool lock.
///
/// A pool created with `reclaim_immediately` never keeps returned items and
/// never asks for a timer.
pub struct BatchStreamPoolBase<T, I: IBatchStreamPoolItems<T>> {
    items: I,
    start_timer: Option<BatchStreamPoolStartTimer>,
    reclaim_immediately: bool,
    state: Mutex<PoolState<T>>,
}

impl<T, I: IBatchStreamPoolItems<T>> BatchStreamPoolBase<T, I> {
    pub fn new(items: I, reclaim_immediately: bool, start_timer: Option<BatchStreamPoolStartTimer>) -> Self {
        let pool = Self {
            items,
            start_timer,
            reclaim_immediately,
            state: Mutex::new(PoolState {
                pool: Vec::new(),
                disposed: false,
                usage: 0,
                usage_statistics: [0; USAGE_STATISTICS_LENGTH],
                usage_statistics_slot: 0,
                timer_is_running: false,
                current_update_tick: 0,
                last_activity_tick: 0,
            }),
        };
        let start = pool.ensure_update_timer(&mut pool.lock());
        pool.notify_start_timer(start);
        pool
    }

    fn lock(&self) -> MutexGuard<'_, PoolState<T>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The number of items currently handed out.
    pub fn current_usage(&self) -> i32 {
        self.lock().usage
    }

    /// The number of items currently kept in the pool.
    pub fn current_pool(&self) -> i32 {
        self.lock().pool.len() as i32
    }

    /// Whether the owner is expected to call
    /// [`update_timer_tick`](Self::update_timer_tick) once per second.
    pub fn timer_is_running(&self) -> bool {
        self.lock().timer_is_running
    }

    /// Marks the timer as running when it is needed and is not running yet.
    /// Returns whether the owner has to be asked to start it.
    fn ensure_update_timer(&self, state: &mut PoolState<T>) -> bool {
        if state.timer_is_running || !self.needs_timer(state) {
            return false;
        }

        state.timer_is_running = true;
        true
    }

    fn notify_start_timer(&self, start: bool) {
        if start {
            if let Some(start_timer) = &self.start_timer {
                start_timer();
            }
        }
    }

    fn needs_timer(&self, state: &PoolState<T>) -> bool {
        !self.reclaim_immediately
            && state.current_update_tick.wrapping_sub(state.last_activity_tick)
                < USAGE_STATISTICS_LENGTH as u64 * 2 + 1
    }

    /// The timer procedure: trims the pool down to the number of items that
    /// were in use at the same time during the last 10 ticks (but keeps at
    /// least 10), and advances the usage statistics.
    ///
    /// To be called by the owner once per second while the timer is running.
    /// Returns whether the timer keeps running.
    pub fn update_timer_tick(&self) -> bool {
        let mut state = self.lock();
        state.current_update_tick = state.current_update_tick.wrapping_add(1);
        let maximum_usage = state.usage_statistics.iter().copied().max().unwrap_or(0);
        let recently_used_pooled_slots = maximum_usage - state.usage;
        let keep_slots = recently_used_pooled_slots.max(10);
        while (keep_slots as usize) < state.pool.len() {
            if let Some(item) = state.pool.pop() {
                self.items.destroy_item(item);
            }
        }

        state.usage_statistics_slot = (state.usage_statistics_slot + 1) % USAGE_STATISTICS_LENGTH;
        let slot = state.usage_statistics_slot;
        state.usage_statistics[slot] = 0;

        state.timer_is_running = self.needs_timer(&state);
        state.timer_is_running
    }

    fn on_activity(&self, state: &mut PoolState<T>) -> bool {
        state.last_activity_tick = state.current_update_tick;
        self.ensure_update_timer(state)
    }

    /// Takes an item from the pool, creating one when the pool is empty.
    pub fn get(&self) -> T {
        let (item, start) = {
            let mut state = self.lock();
            state.usage += 1;
            let slot = state.usage_statistics_slot;
            if state.usage_statistics[slot] < state.usage {
                state.usage_statistics[slot] = state.usage;
            }

            let start = self.on_activity(&mut state);

            (state.pool.pop(), start)
        };
        self.notify_start_timer(start);

        match item {
            Some(item) => item,
            None => self.items.create_item(),
        }
    }

    /// Gives an item back (C# `Return`). The item is cleared, and then kept
    /// for reuse unless the pool is disposed or reclaims immediately.
    pub fn return_item(&self, mut item: T) {
        self.items.clear_item(&mut item);
        let result = {
            let mut state = self.lock();
            state.usage -= 1;
            if !state.disposed && !self.reclaim_immediately {
                state.pool.push(item);
                Ok(self.on_activity(&mut state))
            } else {
                Err(item)
            }
        };

        match result {
            Ok(start) => self.notify_start_timer(start),
            Err(item) => self.items.destroy_item(item),
        }
    }

    /// Destroys the pooled items; items returned from now on are destroyed
    /// instead of being kept.
    pub fn dispose(&self) {
        let mut state = self.lock();
        state.disposed = true;
        for item in state.pool.drain(..) {
            self.items.destroy_item(item);
        }
    }
}

impl<T, I: IBatchStreamPoolItems<T>> Drop for BatchStreamPoolBase<T, I> {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// Creates and clears the arrays of a [`BatchStreamObjectPool`].
pub struct BatchStreamObjectPoolItems<T> {
    array_size: usize,
    marker: PhantomData<fn() -> T>,
}

impl<T> IBatchStreamPoolItems<Box<[Option<T>]>> for BatchStreamObjectPoolItems<T> {
    fn create_item(&self) -> Box<[Option<T>]> {
        std::iter::repeat_with(|| None).take(self.array_size).collect()
    }

    fn clear_item(&self, item: &mut Box<[Option<T>]>) {
        item.fill_with(|| None);
    }
}

/// A pool of fixed-size arrays of optional object references.
pub struct BatchStreamObjectPool<T> {
    base: BatchStreamPoolBase<Box<[Option<T>]>, BatchStreamObjectPoolItems<T>>,
}

impl<T> BatchStreamObjectPool<T> {
    /// The array size used when none is specified.
    pub const DEFAULT_ARRAY_SIZE: usize = 128;

    pub fn new(reclaim_immediately: bool, array_size: usize, start_timer: Option<BatchStreamPoolStartTimer>) -> Self {
        Self {
            base: BatchStreamPoolBase::new(
                BatchStreamObjectPoolItems { array_size, marker: PhantomData },
                reclaim_immediately,
                start_timer,
            ),
        }
    }

    pub fn array_size(&self) -> usize {
        self.base.items.array_size
    }
}

impl<T> Deref for BatchStreamObjectPool<T> {
    type Target = BatchStreamPoolBase<Box<[Option<T>]>, BatchStreamObjectPoolItems<T>>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

/// Creates the buffers of a [`BatchStreamMemoryPool`].
pub struct BatchStreamMemoryPoolItems {
    buffer_size: usize,
}

impl IBatchStreamPoolItems<Box<[u8]>> for BatchStreamMemoryPoolItems {
    fn create_item(&self) -> Box<[u8]> {
        vec![0u8; self.buffer_size].into_boxed_slice()
    }
}

/// A pool of fixed-size byte buffers.
///
/// Buffers are zeroed when they are created; like the unmanaged blocks of
/// the C# class they are not cleared when they are returned.
pub struct BatchStreamMemoryPool {
    base: BatchStreamPoolBase<Box<[u8]>, BatchStreamMemoryPoolItems>,
}

impl BatchStreamMemoryPool {
    /// The buffer size used when none is specified.
    pub const DEFAULT_BUFFER_SIZE: usize = 1024;

    pub fn new(reclaim_immediately: bool, buffer_size: usize, start_timer: Option<BatchStreamPoolStartTimer>) -> Self {
        Self {
            base: BatchStreamPoolBase::new(
                BatchStreamMemoryPoolItems { buffer_size },
                reclaim_immediately,
                start_timer,
            ),
        }
    }

    pub fn buffer_size(&self) -> usize {
        self.base.items.buffer_size
    }
}

impl Deref for BatchStreamMemoryPool {
    type Target = BatchStreamPoolBase<Box<[u8]>, BatchStreamMemoryPoolItems>;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn pools_are_send_and_sync() {
        assert_send_sync::<BatchStreamMemoryPool>();
        assert_send_sync::<BatchStreamObjectPool<Box<dyn std::any::Any + Send>>>();
        assert_send_sync::<BatchStreamObjectPool<Arc<str>>>();
    }

    /// Counts created and destroyed items.
    #[derive(Default)]
    struct Counting {
        created: AtomicUsize,
        cleared: AtomicUsize,
        destroyed: AtomicUsize,
    }

    impl IBatchStreamPoolItems<usize> for Arc<Counting> {
        fn create_item(&self) -> usize {
            self.created.fetch_add(1, Ordering::SeqCst)
        }
        fn clear_item(&self, _item: &mut usize) {
            self.cleared.fetch_add(1, Ordering::SeqCst);
        }
        fn destroy_item(&self, _item: usize) {
            self.destroyed.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn counting_pool(reclaim_immediately: bool) -> (BatchStreamPoolBase<usize, Arc<Counting>>, Arc<Counting>) {
        let counting = Arc::new(Counting::default());
        (BatchStreamPoolBase::new(counting.clone(), reclaim_immediately, None), counting)
    }

    #[test]
    fn get_creates_and_return_pools_items() {
        let (pool, counting) = counting_pool(false);
        let a = pool.get();
        let b = pool.get();
        assert_eq!((0, 1), (a, b));
        assert_eq!(2, pool.current_usage());
        assert_eq!(0, pool.current_pool());

        pool.return_item(a);
        assert_eq!(1, pool.current_usage());
        assert_eq!(1, pool.current_pool());
        assert_eq!(1, counting.cleared.load(Ordering::SeqCst));

        // The pooled item is reused (last in, first out) instead of creating one.
        pool.return_item(b);
        assert_eq!(1, pool.get());
        assert_eq!(0, pool.get());
        assert_eq!(2, counting.created.load(Ordering::SeqCst));
        assert_eq!(0, counting.destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn reclaim_immediately_destroys_returned_items_and_needs_no_timer() {
        let (pool, counting) = counting_pool(true);
        assert!(!pool.timer_is_running());
        let a = pool.get();
        pool.return_item(a);
        assert_eq!(0, pool.current_usage());
        assert_eq!(0, pool.current_pool());
        assert_eq!(1, counting.cleared.load(Ordering::SeqCst));
        assert_eq!(1, counting.destroyed.load(Ordering::SeqCst));
        assert!(!pool.timer_is_running());
        assert!(!pool.update_timer_tick());
    }

    #[test]
    fn dispose_destroys_pooled_items_and_later_returns() {
        let (pool, counting) = counting_pool(false);
        let items: Vec<usize> = (0..3).map(|_| pool.get()).collect();
        pool.return_item(items[0]);
        pool.return_item(items[1]);
        pool.dispose();
        assert_eq!(0, pool.current_pool());
        assert_eq!(2, counting.destroyed.load(Ordering::SeqCst));

        pool.return_item(items[2]);
        assert_eq!(0, pool.current_pool());
        assert_eq!(0, pool.current_usage());
        assert_eq!(3, counting.destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn dropping_the_pool_destroys_pooled_items() {
        let (pool, counting) = counting_pool(false);
        let a = pool.get();
        pool.return_item(a);
        drop(pool);
        assert_eq!(1, counting.destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn tick_keeps_at_least_ten_items() {
        let (pool, counting) = counting_pool(false);
        let items: Vec<usize> = (0..8).map(|_| pool.get()).collect();
        for item in items {
            pool.return_item(item);
        }
        for _ in 0..30 {
            pool.update_timer_tick();
        }
        assert_eq!(8, pool.current_pool());
        assert_eq!(0, counting.destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn tick_trims_to_the_peak_usage_of_the_last_ten_ticks() {
        let (pool, counting) = counting_pool(false);
        // Peak usage of 30, all returned.
        let items: Vec<usize> = (0..30).map(|_| pool.get()).collect();
        for item in items {
            pool.return_item(item);
        }
        assert_eq!(30, pool.current_pool());

        // The peak stays in the statistics for 10 ticks (the slot it was
        // recorded in is the last one to be overwritten).
        for _ in 0..9 {
            assert!(pool.update_timer_tick());
            assert_eq!(30, pool.current_pool());
        }
        // The 10th tick still sees the peak before clearing its slot ...
        assert!(pool.update_timer_tick());
        assert_eq!(30, pool.current_pool());
        // ... and the 11th trims down to the minimum of 10.
        assert!(pool.update_timer_tick());
        assert_eq!(10, pool.current_pool());
        assert_eq!(20, counting.destroyed.load(Ordering::SeqCst));
    }

    #[test]
    fn items_in_use_count_against_the_slots_to_keep() {
        let (pool, _counting) = counting_pool(false);
        // Peak usage of 40; 25 stay in use, 15 are pooled.
        let mut items: Vec<usize> = (0..40).map(|_| pool.get()).collect();
        for item in items.drain(25..) {
            pool.return_item(item);
        }
        assert_eq!(15, pool.current_pool());
        // keep = max(40 - 25, 10) = 15: nothing to trim.
        pool.update_timer_tick();
        assert_eq!(15, pool.current_pool());

        // 15 more are taken from the pool and returned: usage peaks at 40 again.
        let more: Vec<usize> = (0..15).map(|_| pool.get()).collect();
        assert_eq!(0, pool.current_pool());
        for item in more {
            pool.return_item(item);
        }
        // Return everything: 40 pooled, usage 0, recent peak 40 -> keep 40.
        for item in items {
            pool.return_item(item);
        }
        pool.update_timer_tick();
        assert_eq!(40, pool.current_pool());
    }

    #[test]
    fn timer_stops_after_21_idle_ticks_and_restarts_on_activity() {
        let starts = Arc::new(AtomicUsize::new(0));
        let counting = Arc::new(Counting::default());
        let pool = {
            let starts = starts.clone();
            BatchStreamPoolBase::new(
                counting,
                false,
                Some(Box::new(move || {
                    starts.fetch_add(1, Ordering::SeqCst);
                })),
            )
        };
        // The timer is requested from the constructor.
        assert_eq!(1, starts.load(Ordering::SeqCst));
        assert!(pool.timer_is_running());

        // Activity while the timer runs does not request it again.
        let item = pool.get();
        pool.return_item(item);
        assert_eq!(1, starts.load(Ordering::SeqCst));

        for _ in 0..20 {
            assert!(pool.update_timer_tick());
        }
        assert!(!pool.update_timer_tick());
        assert!(!pool.timer_is_running());

        let item = pool.get();
        assert_eq!(2, starts.load(Ordering::SeqCst));
        assert!(pool.timer_is_running());
        pool.return_item(item);
        assert_eq!(2, starts.load(Ordering::SeqCst));

        // Activity resets the idle countdown.
        for _ in 0..15 {
            assert!(pool.update_timer_tick());
        }
        let item = pool.get();
        pool.return_item(item);
        for _ in 0..20 {
            assert!(pool.update_timer_tick());
        }
        assert!(!pool.update_timer_tick());
    }

    #[test]
    fn start_timer_callback_may_use_the_pool() {
        // The callback runs outside of the pool lock.
        let pool: Arc<Mutex<Option<Arc<BatchStreamMemoryPool>>>> = Arc::new(Mutex::new(None));
        let observed = Arc::new(AtomicUsize::new(0));
        let created = {
            let pool = pool.clone();
            let observed = observed.clone();
            Arc::new(BatchStreamMemoryPool::new(
                false,
                16,
                Some(Box::new(move || {
                    if let Some(pool) = pool.lock().unwrap().as_ref() {
                        observed.store(pool.current_usage() as usize, Ordering::SeqCst);
                    }
                })),
            ))
        };
        *pool.lock().unwrap() = Some(created.clone());
        for _ in 0..21 {
            created.update_timer_tick();
        }
        let buffer = created.get();
        assert_eq!(1, observed.load(Ordering::SeqCst));
        created.return_item(buffer);
        *pool.lock().unwrap() = None;
    }

    #[test]
    fn object_pool_hands_out_cleared_arrays() {
        let pool = BatchStreamObjectPool::<Arc<str>>::new(false, BatchStreamObjectPool::<Arc<str>>::DEFAULT_ARRAY_SIZE, None);
        assert_eq!(128, pool.array_size());
        let mut array = pool.get();
        assert_eq!(128, array.len());
        assert!(array.iter().all(|slot| slot.is_none()));

        let value: Arc<str> = Arc::from("value");
        array[3] = Some(value.clone());
        assert_eq!(2, Arc::strong_count(&value));
        pool.return_item(array);
        // Returning clears the references.
        assert_eq!(1, Arc::strong_count(&value));
        assert_eq!(1, pool.current_pool());

        let array = pool.get();
        assert!(array.iter().all(|slot| slot.is_none()));
    }

    #[test]
    fn memory_pool_hands_out_buffers_of_the_requested_size() {
        let pool = BatchStreamMemoryPool::new(false, BatchStreamMemoryPool::DEFAULT_BUFFER_SIZE, None);
        assert_eq!(1024, pool.buffer_size());
        let mut buffer = pool.get();
        assert_eq!(1024, buffer.len());
        buffer[0] = 42;
        pool.return_item(buffer);
        // Buffers are not cleared when returned.
        assert_eq!(42, pool.get()[0]);
    }

    #[test]
    fn pool_can_be_shared_between_threads() {
        let pool = Arc::new(BatchStreamMemoryPool::new(false, 64, None));
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let pool = pool.clone();
                std::thread::spawn(move || {
                    for _ in 0..200 {
                        let buffer = pool.get();
                        pool.return_item(buffer);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(0, pool.current_usage());
        assert!(pool.current_pool() >= 1 && pool.current_pool() <= 4);
    }
}
