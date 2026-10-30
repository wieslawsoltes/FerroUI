//! Original native MDI layout contracts and classic window layout.
mod classic_mdi_layout_manager;
mod i_mdi_layout_manager;
mod mdi_layout_entry;
mod mdi_resize_direction;
pub use classic_mdi_layout_manager::ClassicMdiLayoutManager;
pub use i_mdi_layout_manager::IMdiLayoutManager;
pub use mdi_layout_entry::MdiLayoutEntry;
pub use mdi_resize_direction::MdiResizeDirection;

#[cfg(test)]
mod mdi_tests;
