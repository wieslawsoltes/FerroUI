//! The content of the pasteboard as a data transfer, valid while the
//! change count of the pasteboard is the one it was created at.

use crate::clipboard::clipboard_data_format_helper::{is_text_uti, to_data_format};
use crate::clipboard::pasteboard_item_to_data_transfer_item_wrapper::PasteboardItemToDataTransferItemWrapper;
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::DataFormat;
use objc2::rc::Retained;
use objc2_ui_kit::UIPasteboard;
use std::rc::Rc;

pub(crate) struct PasteboardToDataTransferWrapper {
    pasteboard: Retained<UIPasteboard>,
    change_count: isize,
}

impl PasteboardToDataTransferWrapper {
    pub(crate) fn new(pasteboard: Retained<UIPasteboard>, change_count: isize) -> Rc<PlatformDataTransfer> {
        PlatformDataTransfer::new(PasteboardToDataTransferWrapper { pasteboard, change_count })
    }

    fn is_current(&self) -> bool {
        // SAFETY: a property of the pasteboard, read on the thread of
        // the clipboard of the top-level, the main thread.
        self.change_count == unsafe { self.pasteboard.changeCount() }
    }
}

impl PlatformDataTransferImpl for PasteboardToDataTransferWrapper {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        if !self.is_current() {
            return Ok(Vec::new());
        }

        // SAFETY: as above.
        let types = unsafe { self.pasteboard.pasteboardTypes() };
        Ok(types.iter().map(|type_| to_data_format(&type_.to_string(), &is_text_uti)).collect())
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        if !self.is_current() {
            return Ok(Vec::new());
        }

        // SAFETY: as above.
        let pasteboard_items = unsafe { self.pasteboard.items() };
        Ok(pasteboard_items.iter().map(PasteboardItemToDataTransferItemWrapper::new).collect())
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {}
}
