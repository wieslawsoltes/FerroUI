//! The drag source of the platform: starts a drag and drop operation of
//! OLE with the data of a data transfer.

use crate::data_transfer_to_ole_data_object_wrapper::DataTransferToOleDataObjectWrapper;
use crate::interop::unmanaged_methods;
use crate::ole_drag_source::OleDragSource;
use crate::ole_drop_target::{convert_drop_effect, convert_drop_effect_to_ole};
use crate::win32_com::{DropEffect, IDropSource};
use crate::wnd_proc_guard;
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::{DragDropEffects, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
use ferroui_base::threading::Dispatcher;
use std::rc::Rc;

/// The drag source of the platform.
#[derive(Default)]
pub(crate) struct DragSource;

impl IPlatformDragSource for DragSource {
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        Dispatcher::ui_thread().verify_access();

        trigger_event.pointer().capture(None);

        let (wrapper, data_object) = DataTransferToOleDataObjectWrapper::new(data_transfer);
        let src = IDropSource::from_impl(OleDragSource);
        let allowed = convert_drop_effect_to_ole(allowed_effects);

        // SAFETY: two live COM objects of the interfaces named, held for
        // the duration of the call. The call runs a message loop of its
        // own until the operation ends.
        let (_, final_effect) =
            unsafe { unmanaged_methods::do_drag_drop(data_object.as_ptr().cast(), src.as_ptr().cast(), allowed.0) };

        // Force releasing of internal wrapper to avoid memory leak, if drop target keeps com reference.
        wrapper.release_data_transfer();
        drop(data_object);
        drop(src);

        // A panic of a handler the operation called (a drop target of this
        // process, a window procedure) was kept until the system returned.
        wnd_proc_guard::resume_pending();

        Box::pin(std::future::ready(convert_drop_effect(DropEffect(final_effect))))
    }
}
