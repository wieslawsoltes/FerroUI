//! The source of a drag that starts in the application
//! (`AndroidDragSource` of the reference).

use super::skia_platform::TopLevelImpl;
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::{DragDropEffects, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
use ferroui_base::Visual;
use ferroui_controls::TopLevel;
use std::future::ready;
use std::rc::Rc;

pub(crate) struct AndroidDragSource;

impl IPlatformDragSource for AndroidDragSource {
    /// # Panics
    /// Panics when the source of the trigger event is not in a top-level of
    /// this backend (the reference throws `InvalidOperationException`).
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        _allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        let source = trigger_event.source().and_then(|source| source.cast::<Visual>());
        let platform_impl = TopLevel::get_top_level(source.as_deref()).and_then(|top_level| top_level.platform_impl());
        let Some(drag_drop) = platform_impl
            .as_ref()
            .and_then(|platform_impl| platform_impl.as_any().downcast_ref::<TopLevelImpl>())
            .and_then(TopLevelImpl::drag_drop)
        else {
            panic!("TopLevelImpl is null");
        };

        let Some(async_transfer) = data_transfer.clone().as_async_data_transfer() else {
            data_transfer.dispose();
            return Box::pin(ready(DragDropEffects::NONE));
        };
        drag_drop.start_drag_drop(data_transfer, async_transfer)
    }
}
