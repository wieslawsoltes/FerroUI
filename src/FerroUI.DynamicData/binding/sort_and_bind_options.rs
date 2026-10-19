//! `SortAndBindOptions` (`Binding/SortAndBindOptions.cs`).

use super::BindingOptions;
use ferroui_reactive::concurrency::IScheduler;
use std::fmt;
use std::rc::Rc;

/// The options of `SortAndBind` (`SortAndBindOptions`).
#[derive(Clone)]
pub struct SortAndBindOptions {
    /// The number of changes above which the target is loaded again; 0 never (`ResetThreshold`).
    pub reset_threshold: usize,
    /// Whether an update that keeps its position replaces the item in place
    /// (`UseReplaceForUpdates`).
    pub use_replace_for_updates: bool,
    /// Whether the positions are found by a binary search, which needs a comparer that tells every
    /// item apart (`UseBinarySearch`).
    pub use_binary_search: bool,
    /// The initial capacity of the collection that `SortAndBind` creates; `None` for the default
    /// (`InitialCapacity`, `-1`).
    pub initial_capacity: Option<usize>,
    /// Whether the first change set loads the target with a reset (`ResetOnFirstTimeLoad`).
    pub reset_on_first_time_load: bool,
    /// The scheduler on which the changes are observed, if any (`Scheduler`).
    pub scheduler: Option<Rc<dyn IScheduler>>,
}

impl Default for SortAndBindOptions {
    /// `new SortAndBindOptions()`.
    fn default() -> Self {
        Self {
            reset_threshold: BindingOptions::DEFAULT_RESET_THRESHOLD,
            use_replace_for_updates: BindingOptions::DEFAULT_USE_REPLACE_FOR_UPDATES,
            use_binary_search: false,
            initial_capacity: None,
            reset_on_first_time_load: false,
            scheduler: None,
        }
    }
}

impl PartialEq for SortAndBindOptions {
    /// The parts compare by value and the scheduler by identity (the record of upstream).
    fn eq(&self, other: &Self) -> bool {
        self.reset_threshold == other.reset_threshold
            && self.use_replace_for_updates == other.use_replace_for_updates
            && self.use_binary_search == other.use_binary_search
            && self.initial_capacity == other.initial_capacity
            && self.reset_on_first_time_load == other.reset_on_first_time_load
            && match (&self.scheduler, &other.scheduler) {
                (None, None) => true,
                (Some(a), Some(b)) => std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
                _ => false,
            }
    }
}

impl fmt::Debug for SortAndBindOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SortAndBindOptions")
            .field("reset_threshold", &self.reset_threshold)
            .field("use_replace_for_updates", &self.use_replace_for_updates)
            .field("use_binary_search", &self.use_binary_search)
            .field("initial_capacity", &self.initial_capacity)
            .field("reset_on_first_time_load", &self.reset_on_first_time_load)
            .field("scheduler", &self.scheduler.is_some())
            .finish()
    }
}
