//! Wraps a Win32 `IDataObject` into an item of a data transfer.

use crate::ole_data_object_helper;
use crate::web_clipboard;
use crate::win32_com::IDataObject;
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem, PlatformDataTransferItemImpl};
use ferroui_base::input::DataFormat;
use ferroui_base::input::platform::chromium_clipboard::FormatAlias;
use ferroui_microcom::ComPtr;
use std::any::Any;
use std::rc::Rc;

/// Wraps a Win32 `IDataObject` into an item of a data transfer.
pub(crate) struct OleDataObjectToDataTransferItemWrapper {
    ole_data_object: ComPtr<IDataObject>,
    formats: Vec<DataFormat>,
    web_formats: Rc<[FormatAlias]>,
}

impl OleDataObjectToDataTransferItemWrapper {
    /// `ole_data_object` is the wrapped OLE data object and `formats` the
    /// formats for this item.
    pub(crate) fn new(
        ole_data_object: ComPtr<IDataObject>,
        formats: Vec<DataFormat>,
        web_formats: Rc<[FormatAlias]>,
    ) -> Rc<PlatformDataTransferItem> {
        PlatformDataTransferItem::new(OleDataObjectToDataTransferItemWrapper { ole_data_object, formats, web_formats })
    }
}

impl PlatformDataTransferItemImpl for OleDataObjectToDataTransferItemWrapper {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.formats.clone())
    }

    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        if let Some(alias) = self.web_formats.iter().find(|alias| alias.application == *format) {
            return web_clipboard::read_alias(&self.ole_data_object, alias);
        }
        ole_data_object_helper::try_get(&self.ole_data_object, format)
    }
}
