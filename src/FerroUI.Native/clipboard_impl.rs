use crate::clipboard_data_transfer::ClipboardDataTransfer;
use crate::clipboard_read_session::ClipboardReadSession;
use crate::data_transfer_to_frn_clipboard_data_source_wrapper::DataTransferToFrnClipboardDataSourceWrapper;
use crate::helpers::ClipboardResultExt;
use crate::interop::*;
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{IAsyncDataTransfer, LocalBoxFuture};
use ferroui_base::logging::LogArea;
use ferroui_base::reactive::IDisposable;
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The system clipboard.
pub struct ClipboardImpl {
    native: RefCell<Option<ComPtr<IFrnClipboard>>>,
    last_clear_change_count: Cell<i64>,
}

/// Runs `operation` now and returns its outcome as a completed future: the
/// value or the error of the native clipboard call that failed (the COM
/// exception of the reference). A panic (the clipboard was disposed) is
/// raised when the future is awaited, like a faulted task.
fn completed<T: 'static>(
    operation: impl FnOnce() -> Result<T, ClipboardError>,
) -> LocalBoxFuture<Result<T, ClipboardError>> {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(value) => Box::pin(std::future::ready(value)),
        Err(payload) => Box::pin(async move { resume_unwind(payload) }),
    }
}

impl ClipboardImpl {
    pub fn new(native: ComPtr<IFrnClipboard>) -> ClipboardImpl {
        ClipboardImpl { native: RefCell::new(Some(native)), last_clear_change_count: Cell::new(i64::MIN) }
    }

    #[track_caller]
    pub(crate) fn native(&self) -> ComPtr<IFrnClipboard> {
        match self.native.borrow().clone() {
            Some(native) => native,
            None => panic!("Cannot access a disposed object: ClipboardImpl"),
        }
    }

    fn clear_core(&self) -> Result<(), ClipboardError> {
        self.last_clear_change_count.set(self.native().clear().clipboard_check()?);
        Ok(())
    }

    fn try_get_data(&self) -> Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError> {
        let native = self.native();
        let change_count = native.get_change_count().clipboard_check()?;
        let data_transfer = ClipboardDataTransfer::new(ClipboardReadSession::new(native, change_count));

        if data_transfer.try_formats()?.is_empty() {
            data_transfer.dispose();
            return Ok(None);
        }

        Ok(Some(data_transfer))
    }

    fn set_data(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> Result<(), ClipboardError> {
        self.clear_core()?;

        let source = IFrnClipboardDataSource::from_impl(DataTransferToFrnClipboardDataSourceWrapper::new(
            data_transfer.to_synchronous(LogArea::MACOS_PLATFORM),
        ));
        self.native().set_data(Some(&source)).clipboard_check()
    }

    /// Releases the native clipboard.
    pub fn dispose(&self) {
        let native = self.native.borrow_mut().take();
        drop(native);
    }
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        completed(|| self.try_get_data())
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        completed(|| self.set_data(data_transfer))
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        completed(|| self.clear_core())
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }
}

impl IOwnedClipboardImpl for ClipboardImpl {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        let is_current_owner = self
            .native()
            .get_change_count()
            .clipboard_check()
            .map(|change_count| change_count == self.last_clear_change_count.get());
        Box::pin(std::future::ready(is_current_owner))
    }
}
