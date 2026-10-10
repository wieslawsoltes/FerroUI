//! The clipboard of the platform, over the general pasteboard of UIKit.

pub mod clipboard_data_format_helper;
pub mod pasteboard_item_to_data_transfer_item_wrapper;

#[cfg(target_os = "ios")]
pub mod clipboard_impl;
#[cfg(target_os = "ios")]
pub mod pasteboard_to_data_transfer_wrapper;
