//! Dirty rectangle tracking for the server-side compositor.

mod debug_events_dirty_rect_collector_proxy;
mod i_dirty_rect_tracker;
mod multi_dirty_rect_tracker;
mod multi_dirty_rect_tracker_c_dirty_region;
mod partial_presentation_dirty_rect_history;
mod region_dirty_rect_tracker;
mod single_dirty_rect_tracker;

pub use debug_events_dirty_rect_collector_proxy::DebugEventsDirtyRectCollectorProxy;
pub use i_dirty_rect_tracker::{IDirtyRectCollector, IDirtyRectTracker};
pub use multi_dirty_rect_tracker::MultiDirtyRectTracker;
pub(crate) use partial_presentation_dirty_rect_history::PartialPresentationDirtyRectHistory;
pub use region_dirty_rect_tracker::RegionDirtyRectTracker;
pub use single_dirty_rect_tracker::SingleDirtyRectTracker;

#[cfg(test)]
pub(crate) use i_dirty_rect_tracker::tests as test_mocks;
