//! The behaviors of selecting items controls (`SelectingItemsControl/`).

mod selecting_items_control_events_behavior;
mod selecting_items_control_search_behavior;

pub use selecting_items_control_events_behavior::{
    SelectingItemsControlEventsBehavior, SelectingItemsControlEventsBehaviorImpl,
    SelectingItemsControlEventsBehaviorImplExt, SelectingItemsControlEventsBehaviorVTable,
};
pub use selecting_items_control_search_behavior::{SelectingItemsControlSearchBehavior, SortDirection};
