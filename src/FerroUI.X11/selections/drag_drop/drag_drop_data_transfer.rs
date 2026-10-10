//! The data of a drag that comes from another client, and the state of
//! that drag over a window (the port of `DragDropDataTransfer.cs`).

use crate::selections::drag_drop::drag_drop_data_reader::IDragDropItemsSource;
use crate::xlib::{Time, XID};
use ferroui_base::input::platform::{
    ClipboardError, PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem,
};
use ferroui_base::input::{DataFormat, DragDropEffects, IDataTransfer, IInputRoot};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Point;
use std::cell::Cell;
use std::rc::Rc;

/// The data transfer of the base library over the reader of the drag.
struct Transfer {
    reader: Rc<dyn IDragDropItemsSource>,
    data_formats: Vec<DataFormat>,
}

impl PlatformDataTransferImpl for Transfer {
    fn provide_formats(&self) -> Result<Vec<DataFormat>, ClipboardError> {
        Ok(self.data_formats.clone())
    }

    fn provide_items(&self) -> Result<Vec<Rc<PlatformDataTransferItem>>, ClipboardError> {
        self.reader.clone().create_items()
    }

    fn dispose(&self, _owner: &PlatformDataTransfer) {
        self.reader.dispose();
    }
}

/// Implementation of the data transfer for data being dragged into a window of the framework via Xdnd.
///
/// The reference class derives from the data transfer of the base
/// library; here it holds one (`data_transfer`), which is what the drag
/// events carry.
pub struct DragDropDataTransfer {
    data_transfer: Rc<PlatformDataTransfer>,
    source_window: XID,
    target_window: XID,
    input_root: Rc<dyn IInputRoot>,
    last_position: Cell<Option<Point>>,
    last_timestamp: Cell<Time>,
    result_effects: Cell<DragDropEffects>,
    dropped: Cell<bool>,
}

impl DragDropDataTransfer {
    pub fn new(
        reader: Rc<dyn IDragDropItemsSource>,
        data_formats: Vec<DataFormat>,
        source_window: XID,
        target_window: XID,
        input_root: Rc<dyn IInputRoot>,
    ) -> Self {
        Self {
            data_transfer: PlatformDataTransfer::new(Transfer { reader, data_formats }),
            source_window,
            target_window,
            input_root,
            last_position: Cell::new(None),
            last_timestamp: Cell::new(0),
            result_effects: Cell::new(DragDropEffects::NONE),
            dropped: Cell::new(false),
        }
    }

    /// The data, as the drag events carry it.
    pub fn data_transfer(&self) -> Rc<dyn IDataTransfer> {
        self.data_transfer.clone()
    }

    pub fn source_window(&self) -> XID {
        self.source_window
    }

    pub fn target_window(&self) -> XID {
        self.target_window
    }

    pub fn input_root(&self) -> &Rc<dyn IInputRoot> {
        &self.input_root
    }

    pub fn last_position(&self) -> Option<Point> {
        self.last_position.get()
    }

    pub fn set_last_position(&self, value: Option<Point>) {
        self.last_position.set(value);
    }

    pub fn last_timestamp(&self) -> Time {
        self.last_timestamp.get()
    }

    pub fn set_last_timestamp(&self, value: Time) {
        self.last_timestamp.set(value);
    }

    pub fn result_effects(&self) -> DragDropEffects {
        self.result_effects.get()
    }

    pub fn set_result_effects(&self, value: DragDropEffects) {
        self.result_effects.set(value);
    }

    pub fn dropped(&self) -> bool {
        self.dropped.get()
    }

    pub fn set_dropped(&self, value: bool) {
        self.dropped.set(value);
    }

    pub fn dispose(&self) {
        self.data_transfer.dispose();
    }
}
