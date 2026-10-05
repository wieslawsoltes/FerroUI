use super::{ClipboardType, IClipboard, IPlatformClipboardManagerImpl};
use std::rc::Rc;

/// A clipboard manager holding the clipboards it was created with.
pub struct PlatformClipboardManager {
    clipboard: Option<Rc<dyn IClipboard>>,
    primary_selection: Option<Rc<dyn IClipboard>>,
}

impl PlatformClipboardManager {
    /// Creates the manager from the default clipboard and the primary
    /// selection clipboard of the platform.
    pub fn new(clipboard: Option<Rc<dyn IClipboard>>, primary_selection: Option<Rc<dyn IClipboard>>) -> Self {
        Self { clipboard, primary_selection }
    }
}

impl IPlatformClipboardManagerImpl for PlatformClipboardManager {
    fn try_get_clipboard(&self, type_: ClipboardType) -> Option<Rc<dyn IClipboard>> {
        match type_ {
            ClipboardType::Default => self.clipboard.clone(),
            ClipboardType::PrimarySelection => self.primary_selection.clone(),
        }
    }
}
