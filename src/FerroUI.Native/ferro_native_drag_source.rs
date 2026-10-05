use crate::data_transfer_to_frn_clipboard_data_source_wrapper::DataTransferToFrnClipboardDataSourceWrapper;
use crate::helpers::to_frn_point;
use crate::interop::*;
use crate::top_level_impl::top_level_of;
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::{DragDropEffects, IDataTransfer, LocalBoxFuture, PointerPressedEventArgs};
use ferroui_base::{Ref, Visual};
use ferroui_controls::TopLevel;
use std::cell::RefCell;
use std::ffi::c_void;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// Starts native drag-and-drop operations.
#[derive(Default)]
pub struct FerroNativeDragSource;

impl FerroNativeDragSource {
    pub fn new() -> Self {
        Self
    }
}

/// The outcome of a drag-and-drop operation, set once by the native side.
#[derive(Default)]
struct Completion {
    result: Option<DragDropEffects>,
    waker: Option<Waker>,
}

struct CompletionFuture(Rc<RefCell<Completion>>);

impl Future for CompletionFuture {
    type Output = DragDropEffects;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<DragDropEffects> {
        let mut completion = self.0.borrow_mut();
        match completion.result {
            Some(result) => Poll::Ready(result),
            None => {
                completion.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

struct DndCallback {
    completion: Rc<RefCell<Completion>>,
}

impl IFrnDndResultCallbackImpl for DndCallback {
    fn on_drag_and_drop_complete(&self, effect: FrnDragDropEffects) {
        crate::callback_base::guard((), || {
            let waker = {
                let mut completion = self.completion.borrow_mut();
                if completion.result.is_some() {
                    return;
                }
                completion.result = Some(DragDropEffects::from_bits_retain(effect.0));
                completion.waker.take()
            };
            if let Some(waker) = waker {
                waker.wake();
            }
        })
    }
}

/// The handle a data transfer travels through native code with: it comes
/// back in the drag events of the top-level under the pointer and is freed
/// through the handle deallocator of the platform.
pub(crate) fn data_transfer_to_handle(data_transfer: Rc<dyn IDataTransfer>) -> *mut c_void {
    Box::into_raw(Box::new(data_transfer)) as *mut c_void
}

/// The data transfer behind a handle made by [`data_transfer_to_handle`].
///
/// # Safety
/// `handle` must be null or a live handle made by `data_transfer_to_handle`.
pub(crate) unsafe fn data_transfer_from_handle(handle: *mut c_void) -> Option<Rc<dyn IDataTransfer>> {
    // SAFETY: per the contract, a non-null handle points at a live box.
    unsafe { (handle as *const Rc<dyn IDataTransfer>).as_ref() }.cloned()
}

/// Frees a handle made by [`data_transfer_to_handle`].
///
/// # Safety
/// `handle` must be null or a live handle made by `data_transfer_to_handle`
/// that is not used afterwards.
pub(crate) unsafe fn free_data_transfer_handle(handle: *mut c_void) {
    if !handle.is_null() {
        // SAFETY: per the contract, the handle is a box that is freed once.
        drop(unsafe { Box::from_raw(handle as *mut Rc<dyn IDataTransfer>) });
    }
}

impl IPlatformDragSource for FerroNativeDragSource {
    /// # Panics
    /// Panics when the source of the trigger event is not hosted by a
    /// top-level of this backend.
    fn do_drag_drop_async(
        &self,
        trigger_event: &PointerPressedEventArgs,
        data_transfer: Rc<dyn IDataTransfer>,
        allowed_effects: DragDropEffects,
    ) -> LocalBoxFuture<DragDropEffects> {
        // Sanity check
        let source = trigger_event.source().and_then(|source| source.cast::<Visual>());
        let tl = TopLevel::get_top_level(source.as_deref());
        let view = tl
            .as_ref()
            .and_then(|tl| tl.platform_impl())
            .and_then(|platform_impl| top_level_of(&*platform_impl).cloned());
        let (Some(view), Some(tl)) = (view, tl) else {
            panic!("Value does not fall within the expected range.");
        };
        let tl: Ref<Visual> = tl.upcast();

        trigger_event.pointer().capture(None);

        let completion = Rc::new(RefCell::new(Completion::default()));

        let cb = IFrnDndResultCallback::from_impl(DndCallback { completion: completion.clone() });
        let data_source = IFrnClipboardDataSource::from_impl(DataTransferToFrnClipboardDataSourceWrapper::new(
            data_transfer.clone(),
        ));
        view.begin_dragging_session(
            FrnDragDropEffects(allowed_effects.bits()),
            to_frn_point(trigger_event.get_position(Some(&tl))),
            &data_source,
            &cb,
            data_transfer_to_handle(data_transfer),
        );

        Box::pin(CompletionFuture(completion))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::platform::{PlatformDataTransfer, PlatformDataTransferImpl, PlatformDataTransferItem};
    use ferroui_base::input::DataFormat;
    use std::task::Wake;

    struct Empty;
    impl PlatformDataTransferImpl for Empty {
        fn provide_formats(&self) -> Result<Vec<DataFormat>, ferroui_base::input::platform::ClipboardError> {
            Ok(Vec::new())
        }
        fn provide_items(
            &self,
        ) -> Result<Vec<Rc<PlatformDataTransferItem>>, ferroui_base::input::platform::ClipboardError> {
            Ok(Vec::new())
        }
        fn dispose(&self, _owner: &PlatformDataTransfer) {}
    }

    #[test]
    fn data_transfer_handles_round_trip() {
        let data_transfer: Rc<dyn IDataTransfer> = PlatformDataTransfer::new(Empty);
        let handle = data_transfer_to_handle(data_transfer.clone());
        assert_eq!(Rc::strong_count(&data_transfer), 2);

        // SAFETY: the handle was just made and is freed once below.
        let recovered = unsafe { data_transfer_from_handle(handle) }.expect("data transfer");
        assert!(Rc::ptr_eq(&recovered, &data_transfer));
        drop(recovered);
        unsafe { free_data_transfer_handle(handle) };
        assert_eq!(Rc::strong_count(&data_transfer), 1);

        // SAFETY: null handles are accepted.
        assert!(unsafe { data_transfer_from_handle(std::ptr::null_mut()) }.is_none());
        unsafe { free_data_transfer_handle(std::ptr::null_mut()) };
    }

    struct NoopWaker;
    impl Wake for NoopWaker {
        fn wake(self: std::sync::Arc<Self>) {}
    }

    #[test]
    fn the_operation_completes_with_the_first_reported_effect() {
        let completion = Rc::new(RefCell::new(Completion::default()));
        let callback = IFrnDndResultCallback::from_impl(DndCallback { completion: completion.clone() });
        let mut future = Box::pin(CompletionFuture(completion));
        let waker = Waker::from(std::sync::Arc::new(NoopWaker));
        let mut context = Context::from_waker(&waker);

        assert!(future.as_mut().poll(&mut context).is_pending());
        callback.on_drag_and_drop_complete(FrnDragDropEffects::Move);
        callback.on_drag_and_drop_complete(FrnDragDropEffects::Copy);
        assert_eq!(future.as_mut().poll(&mut context), Poll::Ready(DragDropEffects::MOVE));
    }
}
