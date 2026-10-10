//! The item of a drag that has every format but the files (the port of
//! `DragDropDataTransferItem.cs`).

use crate::selections::drag_drop::drag_drop_data_reader::DragDropDataReader;
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem, PlatformDataTransferItemImpl};
use ferroui_base::input::DataFormat;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// Implementation of the data transfer item for Xdnd: the reader is the
/// object used to read values.
pub struct DragDropDataTransferItem {
    reader: Rc<DragDropDataReader>,
    formats: Vec<DataFormat>,
    cached_values: RefCell<Vec<(DataFormat, Option<Rc<dyn Any>>)>>,
}

impl DragDropDataTransferItem {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(reader: Rc<DragDropDataReader>, formats: Vec<DataFormat>) -> Rc<PlatformDataTransferItem> {
        PlatformDataTransferItem::new(DragDropDataTransferItem { reader, formats, cached_values: RefCell::new(Vec::new()) })
    }
}

impl PlatformDataTransferItemImpl for DragDropDataTransferItem {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.formats.clone())
    }

    fn try_get_raw_core(&self, format: &DataFormat) -> Result<Option<Rc<dyn Any>>, ClipboardError> {
        let cached = self.cached_values.borrow().iter().find(|(cached, _)| cached == format).map(|(_, value)| value.clone());
        if let Some(value) = cached {
            return Ok(value);
        }

        let value = self.reader.clone().try_get(format)?;
        self.cached_values.borrow_mut().push((format.clone(), value.clone()));
        Ok(value)
    }
}
