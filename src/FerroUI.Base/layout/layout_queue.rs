use crate::logging::{LogArea, LogEventLevel, Logger};
use super::Layoutable;
use crate::Ref;
use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::hash::Hash;

/// An item of a [`LayoutQueue`]: what the queue needs of the C# `T` (a
/// reference type compared by identity, and its `ToString()` for the layout
/// cycle warning).
pub(crate) trait LayoutQueueItem: Clone + Eq + Hash + 'static {
    /// Writes the item as C# `ToString()` would: an object that does not
    /// override it is written as the full name of its type.
    fn fmt_item(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}

impl LayoutQueueItem for Ref<Layoutable> {
    fn fmt_item(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.get_type().full_name())
    }
}

impl LayoutQueueItem for String {
    fn fmt_item(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

/// Formats a [`LayoutQueueItem`] for the log.
struct ItemDisplay<'a, T>(&'a T);

impl<T: LayoutQueueItem> fmt::Display for ItemDisplay<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt_item(f)
    }
}

#[derive(Clone, Copy, Default)]
struct Info {
    active: bool,
    count: i32,
}

/// The per-loop information of the items of a [`LayoutQueue`], enumerated in
/// insertion order as the C# `Dictionary<T, Info>` is (entries are only added
/// and updated between two clears, never removed one by one).
struct LoopQueueInfo<T> {
    entries: Vec<(T, Info)>,
    index: HashMap<T, usize>,
}

impl<T: Clone + Eq + Hash> LoopQueueInfo<T> {
    fn new() -> Self {
        Self { entries: Vec::new(), index: HashMap::new() }
    }

    fn try_get_value(&self, item: &T) -> Option<Info> {
        self.index.get(item).map(|&i| self.entries[i].1)
    }

    fn set(&mut self, item: T, info: Info) {
        match self.index.get(&item) {
            Some(&i) => self.entries[i].1 = info,
            None => {
                self.index.insert(item.clone(), self.entries.len());
                self.entries.push((item, info));
            }
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
    }
}

/// A queue of items awaiting layout that limits how many times an item can be
/// enqueued within one layout loop, to break layout cycles.
pub(crate) struct LayoutQueue<T> {
    should_enqueue: fn(&T) -> bool,
    inner: VecDeque<T>,
    loop_queue_info: LoopQueueInfo<T>,
    not_finalized_buffer: Vec<(T, Info)>,
    max_enqueue_count_per_loop: i32,
}

impl<T: LayoutQueueItem> LayoutQueue<T> {
    pub fn new(should_enqueue: fn(&T) -> bool) -> Self {
        Self {
            should_enqueue,
            inner: VecDeque::new(),
            loop_queue_info: LoopQueueInfo::new(),
            not_finalized_buffer: Vec::new(),
            max_enqueue_count_per_loop: 1,
        }
    }

    pub fn count(&self) -> usize {
        self.inner.len()
    }

    /// Removes and returns the item at the head of the queue.
    ///
    /// Panics when the queue is empty, as the C# `Queue<T>.Dequeue` throws.
    pub fn dequeue(&mut self) -> T {
        let result = self.inner.pop_front().expect("Queue empty.");

        if let Some(mut info) = self.loop_queue_info.try_get_value(&result) {
            info.active = false;
            self.loop_queue_info.set(result.clone(), info);
        }

        result
    }

    pub fn enqueue(&mut self, item: T) {
        let info = self.loop_queue_info.try_get_value(&item).unwrap_or_default();

        if !info.active {
            if info.count < self.max_enqueue_count_per_loop {
                self.inner.push_back(item.clone());
                self.loop_queue_info.set(item, Info { active: true, count: info.count + 1 });
            } else if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::LAYOUT) {
                logger.log_with_values(
                    Some(self as &dyn Any),
                    "Layout cycle detected. Item {Item} was enqueued {Count} times.",
                    &[&ItemDisplay(&item), &info.count],
                );
            }
        }
    }

    pub fn begin_loop(&mut self, max_enqueue_count_per_loop: i32) {
        self.max_enqueue_count_per_loop = max_enqueue_count_per_loop;
    }

    pub fn end_loop(&mut self) {
        for (item, info) in &self.loop_queue_info.entries {
            if info.count >= self.max_enqueue_count_per_loop {
                self.not_finalized_buffer.push((item.clone(), *info));
            }
        }

        self.loop_queue_info.clear();

        // Prevent layout cycle but add to next layout the non arranged/measured
        // items that might have caused cycle one more time as a final attempt.
        let mut not_finalized_buffer = std::mem::take(&mut self.not_finalized_buffer);
        for (item, _) in &not_finalized_buffer {
            if (self.should_enqueue)(item) {
                self.loop_queue_info.set(item.clone(), Info { active: true, count: 0 });
                self.inner.push_back(item.clone());
            }
        }

        not_finalized_buffer.clear();
        self.not_finalized_buffer = not_finalized_buffer;
    }

    pub fn dispose(&mut self) {
        self.inner.clear();
        self.loop_queue_info.clear();
        self.not_finalized_buffer.clear();
    }
}

/// C# `GetEnumerator()`: the queued items, in queue order.
impl<'a, T> IntoIterator for &'a LayoutQueue<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}
