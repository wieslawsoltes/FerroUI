use super::{ClipboardType, IClipboard};
use std::rc::Rc;

/// Gives access to the clipboards of the platform.
///
/// This is an implementation detail of the platform backends.
pub trait IPlatformClipboardManagerImpl {
    /// Gets the clipboard of the given type, if the platform has one.
    fn try_get_clipboard(&self, type_: ClipboardType) -> Option<Rc<dyn IClipboard>>;
}
