use crate::input::{DragDropEffects, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
use std::rc::Rc;

/// Starts drag-and-drop operations on the platform.
///
/// This is an implementation detail of the platform backends.
pub trait IPlatformDragSource {
    /// Starts a drag-and-drop operation with the given data and returns a
    /// future that resolves to the effect of the operation when it ends.
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects>;
}
