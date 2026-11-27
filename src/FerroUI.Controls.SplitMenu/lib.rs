//! ferroui-controls-split-menu
//!
//! `SplitMenuFlyoutItem` of WinUI on the `MenuItem` of FerroUI: the item is its primary action,
//! and the part at its trailing edge opens its submenu. It uses the menu item themes, and an
//! application that does not use it links none of this crate.

// The markup of other crates names the types of this crate by the name of the crate.
extern crate self as ferroui_controls_split_menu;

mod register_types;
mod rust_paths;
mod split_menu_flyout_item;

pub use register_types::{register_types, ASSEMBLY};
pub use split_menu_flyout_item::SplitMenuFlyoutItem;

#[cfg(test)]
mod tests;
