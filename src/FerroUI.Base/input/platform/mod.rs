//! Platform-dependent input configuration.

mod key_gesture_format_info;
mod platform_hotkey_configuration;

pub use key_gesture_format_info::KeyGestureFormatInfo;
pub use platform_hotkey_configuration::PlatformHotkeyConfiguration;

mod clipboard;
mod clipboard_error;
mod clipboard_extensions;
mod clipboard_type;
mod i_clipboard;
mod i_clipboard_impl;
mod i_flushable_clipboard_impl;
mod i_owned_clipboard_impl;
mod i_platform_clipboard_manager_impl;
mod i_platform_drag_source;
mod platform_async_data_transfer;
mod platform_async_data_transfer_item;
mod platform_clipboard_manager;
mod platform_data_transfer;
mod platform_data_transfer_item;

pub use clipboard::Clipboard;
pub use clipboard_error::{ClipboardError, ClipboardErrorKind};
pub use clipboard_extensions::ClipboardExtensions;
pub use clipboard_type::ClipboardType;
pub use i_clipboard::IClipboard;
pub use i_clipboard_impl::IClipboardImpl;
pub use i_flushable_clipboard_impl::IFlushableClipboardImpl;
pub use i_owned_clipboard_impl::IOwnedClipboardImpl;
pub use i_platform_clipboard_manager_impl::IPlatformClipboardManagerImpl;
pub use i_platform_drag_source::IPlatformDragSource;
pub use platform_async_data_transfer::{PlatformAsyncDataTransfer, PlatformAsyncDataTransferImpl};
pub use platform_async_data_transfer_item::{PlatformAsyncDataTransferItem, PlatformAsyncDataTransferItemImpl};
pub use platform_clipboard_manager::PlatformClipboardManager;
pub use platform_data_transfer::{PlatformDataTransfer, PlatformDataTransferImpl};
pub use platform_data_transfer_item::{PlatformDataTransferItem, PlatformDataTransferItemImpl};
