use crate::clipboard_data_format_helper::to_data_formats;
use crate::clipboard_data_transfer_item::ClipboardDataTransferItem;
use crate::clipboard_read_session::ClipboardReadSession;
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::DataFormat;
use std::rc::Rc;

/// The data of a native clipboard (or dragging pasteboard) read through a
/// clipboard session, which the data transfer owns.
pub(crate) struct ClipboardDataTransfer {
    session: Rc<ClipboardReadSession>,
}

impl ClipboardDataTransfer {
    pub(crate) fn new(session: ClipboardReadSession) -> Rc<PlatformDataTransfer> {
        PlatformDataTransfer::new(ClipboardDataTransfer { session: Rc::new(session) })
    }
}

impl PlatformDataTransferImpl for ClipboardDataTransfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        let formats = self.session.get_formats()?;
        Ok(to_data_formats(formats.as_deref(), &|format| self.session.is_text_format(format)))
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        let item_count = self.session.get_item_count()?;
        Ok((0..item_count).map(|i| ClipboardDataTransferItem::new(self.session.clone(), i)).collect())
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {
        self.session.dispose();
    }
}
