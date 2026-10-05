use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

#[derive(Clone, Copy, Default)]
struct Info {
    active: bool,
    count: u32,
}

/// A queue of items awaiting layout that limits how many times an item can be
/// enqueued within one layout loop, to break layout cycles.
pub(crate) struct LayoutQueue<T> {
    should_enqueue: fn(&T) -> bool,
    inner: VecDeque<T>,
    loop_queue_info: HashMap<T, Info>,
    max_enqueue_count_per_loop: u32,
}

impl<T: Clone + Eq + Hash> LayoutQueue<T> {
    pub fn new(should_enqueue: fn(&T) -> bool) -> Self {
        Self {
            should_enqueue,
            inner: VecDeque::new(),
            loop_queue_info: HashMap::new(),
            max_enqueue_count_per_loop: 1,
        }
    }

    pub fn count(&self) -> usize {
        self.inner.len()
    }

    pub fn dequeue(&mut self) -> Option<T> {
        let result = self.inner.pop_front()?;
        if let Some(info) = self.loop_queue_info.get_mut(&result) {
            info.active = false;
        }
        Some(result)
    }

    /// Enqueues an item. Returns false if the item was rejected because a
    /// layout cycle was detected.
    pub fn enqueue(&mut self, item: T) -> bool {
        let info = self.loop_queue_info.get(&item).copied().unwrap_or_default();
        if info.active {
            return true;
        }
        if info.count < self.max_enqueue_count_per_loop {
            self.inner.push_back(item.clone());
            self.loop_queue_info.insert(item, Info { active: true, count: info.count + 1 });
            true
        } else {
            false
        }
    }

    pub fn begin_loop(&mut self, max_enqueue_count_per_loop: u32) {
        self.max_enqueue_count_per_loop = max_enqueue_count_per_loop;
    }

    pub fn end_loop(&mut self) {
        let max = self.max_enqueue_count_per_loop;
        let not_finalized: Vec<T> = self
            .loop_queue_info
            .iter()
            .filter(|(_, info)| info.count >= max)
            .map(|(item, _)| item.clone())
            .collect();
        self.loop_queue_info.clear();

        // Prevent a layout cycle but add to the next layout the non
        // arranged/measured items that might have caused the cycle one more
        // time as a final attempt.
        for item in not_finalized {
            if (self.should_enqueue)(&item) {
                self.loop_queue_info.insert(item.clone(), Info { active: true, count: 0 });
                self.inner.push_back(item);
            }
        }
    }

    pub fn clear(&mut self) {
        self.inner.clear();
        self.loop_queue_info.clear();
    }
}
