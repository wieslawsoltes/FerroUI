//! ferroui-controls-icons
//!
//! [`BitmapIcon`] and [`ImageIcon`] of WinUI, with their sources [`BitmapIconSource`] and
//! [`ImageIconSource`]. The icons draw in the control itself and have no template, so they need no
//! theme, and an application that does not use them links none of this crate.

// The markup of other crates names the types of this crate by the name of the crate.
extern crate self as ferroui_controls_icons;

mod bitmap_icon;
mod icon_sources;
mod image_icon;
mod placement;
mod register_types;
mod rust_paths;
mod uri_bitmap;

pub use bitmap_icon::BitmapIcon;
pub use icon_sources::{BitmapIconSource, ImageIconSource};
pub use image_icon::ImageIcon;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
