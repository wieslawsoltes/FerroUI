//! The clipboard history of the system and the options of a copy (WinUI's `Clipboard`
//! history members). Windows has them; the other platforms report that they do not.

use super::{ClipboardError, IClipboardImpl};
use crate::input::{IAsyncDataTransfer, LocalBoxFuture};
use std::rc::Rc;
use std::time::SystemTime;

/// How data put on the clipboard may be kept (WinUI's `ClipboardContentOptions`). Both are
/// allowed by default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipboardContentOptions {
    /// Whether the data may appear in the clipboard history.
    pub is_allowed_in_history: bool,
    /// Whether the data may be synchronized to the user's other devices.
    pub is_roaming_allowed: bool,
}

impl Default for ClipboardContentOptions {
    fn default() -> Self {
        Self { is_allowed_in_history: true, is_roaming_allowed: true }
    }
}

/// What reading the clipboard history gave (WinUI's `ClipboardHistoryItemsResultStatus`, with
/// `NotSupported` for a platform without history).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipboardHistoryItemsResultStatus {
    /// The items were read.
    Success,
    /// The system denied access to the history.
    AccessDenied,
    /// The user turned the history off.
    ClipboardHistoryDisabled,
    /// The platform has no clipboard history.
    NotSupported,
}

/// One entry of the clipboard history (WinUI's `ClipboardHistoryItem`). Its data is read only
/// when asked for.
pub trait IClipboardHistoryItem {
    /// The identifier of the entry.
    fn id(&self) -> String;
    /// When the entry was copied.
    fn timestamp(&self) -> Option<SystemTime>;
    /// The formats of the entry, by the names of the platform.
    fn formats(&self) -> Vec<String>;
    /// The text of the entry, `None` when it has none.
    fn get_text_async(&self) -> LocalBoxFuture<Result<Option<String>, ClipboardError>>;
}

/// The clipboard history (WinUI's `ClipboardHistoryItemsResult`).
#[derive(Clone)]
pub struct ClipboardHistoryItemsResult {
    /// Whether the items could be read.
    pub status: ClipboardHistoryItemsResultStatus,
    /// The entries, the most recent first; empty unless `status` is `Success`.
    pub items: Vec<Rc<dyn IClipboardHistoryItem>>,
}

impl ClipboardHistoryItemsResult {
    /// No items, for `status`.
    pub fn empty(status: ClipboardHistoryItemsResultStatus) -> Self {
        Self { status, items: Vec::new() }
    }
}

/// A platform clipboard with a history of the system.
///
/// This is an implementation detail of the platform backends.
pub trait IClipboardHistoryImpl: IClipboardImpl {
    /// Whether the user has the clipboard history turned on.
    fn is_history_enabled(&self) -> bool;
    /// Whether the user has the synchronization of the clipboard to other devices turned on.
    fn is_roaming_enabled(&self) -> bool;
    /// Puts `data_transfer` on the clipboard with `options`.
    fn set_data_with_options_async(
        &self,
        data_transfer: Rc<dyn IAsyncDataTransfer>,
        options: ClipboardContentOptions,
    ) -> LocalBoxFuture<Result<(), ClipboardError>>;
    /// Reads the entries of the history.
    fn get_history_items_async(&self) -> LocalBoxFuture<Result<ClipboardHistoryItemsResult, ClipboardError>>;
}
