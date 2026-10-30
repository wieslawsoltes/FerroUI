//! Original dock selector contracts and immutable entries.
mod dock_selector_item;
mod dock_selector_mode;
mod i_dock_selector_service;
mod selector_items;
pub use dock_selector_item::DockSelectorItem;
pub use dock_selector_mode::DockSelectorMode;
pub use i_dock_selector_service::IDockSelectorService;
pub(crate) use selector_items::register_types;
