//! The clipboard of the system, through OLE: a data transfer is put on the
//! clipboard as a data object the system asks for its formats and their
//! data, and what is on the clipboard is read as the data object of the
//! system. The retry discipline of the reference (the clipboard may be held
//! open by another process for a moment) is kept.

use crate::data_transfer_to_ole_data_object_wrapper::DataTransferToOleDataObjectWrapper;
use crate::interop::unmanaged_methods::{
    self, close_clipboard, empty_clipboard, ole_flush_clipboard, ole_get_clipboard, open_clipboard, HRESULT,
};
use crate::ole_data_object_to_data_transfer_wrapper::OleDataObjectToDataTransferWrapper;
use crate::win32_com::IDataObject;
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, IFlushableClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{IAsyncDataTransfer, LocalBoxFuture};
use ferroui_base::logging::LogArea;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

const OLE_RETRY_COUNT: i32 = 10;
const OLE_RETRY_DELAY: u64 = 100;

/// The amount of time in milliseconds to sleep before flushing the clipboard after a set.
///
/// This is mitigation for clipboard listener issues.
const OLE_FLUSH_DELAY: u64 = 10;

/// The data object this clipboard stored last, while the system holds it.
#[derive(Default)]
struct LastStored {
    data_object: RefCell<Option<Rc<DataTransferToOleDataObjectWrapper>>>,
    /// The address of the COM object of `data_object`; 0 when there is
    /// none.
    data_object_int_ptr: Cell<usize>,
}

impl LastStored {
    fn clear(&self) {
        self.data_object.borrow_mut().take();
        self.data_object_int_ptr.set(0);
    }
}

/// The system clipboard.
pub struct ClipboardImpl {
    last_stored: Rc<LastStored>,
}

struct DelayState {
    done: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// Completes after a time, measured by a timer of the dispatcher.
struct Delay {
    duration: Duration,
    state: Option<Rc<DelayState>>,
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let state = match &self.state {
            Some(state) => state.clone(),
            None => {
                let state = Rc::new(DelayState { done: Cell::new(false), waker: RefCell::new(None) });
                let shared = state.clone();
                DispatcherTimer::run_once(
                    move || {
                        shared.done.set(true);
                        let waker = shared.waker.borrow_mut().take();
                        if let Some(waker) = waker {
                            waker.wake();
                        }
                    },
                    self.duration,
                    DispatcherPriority::default(),
                );
                self.state = Some(state.clone());
                state
            }
        };

        if state.done.get() {
            Poll::Ready(())
        } else {
            *state.waker.borrow_mut() = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn delay(milliseconds: u64) -> Delay {
    Delay { duration: Duration::from_millis(milliseconds), state: None }
}

/// The clipboard, open for the calling thread until the value is dropped.
struct OpenClipboard;

impl Drop for OpenClipboard {
    fn drop(&mut self) {
        close_clipboard();
    }
}

/// Opens the clipboard, trying again for a second while another process
/// holds it.
async fn open_clipboard_async() -> Result<OpenClipboard, ClipboardError> {
    let mut i = OLE_RETRY_COUNT;

    while !open_clipboard(0) {
        i -= 1;
        if i == 0 {
            return Err(ClipboardError::timeout("Timeout opening clipboard."));
        }
        delay(100).await;
    }

    Ok(OpenClipboard)
}

fn hresult_error(hr: u32, message: &str) -> ClipboardError {
    ClipboardError::platform(hr as i32, format!("{message} (result code 0x{hr:08X})"))
}

impl ClipboardImpl {
    /// Creates the clipboard.
    #[allow(clippy::new_without_default)]
    pub fn new() -> ClipboardImpl {
        ClipboardImpl { last_stored: Rc::default() }
    }
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(async {
            Dispatcher::ui_thread().verify_access();
            let mut i = OLE_RETRY_COUNT;

            loop {
                let (hr, data_object) = ole_get_clipboard();

                if hr == 0 {
                    // SAFETY: the system returned a data object, of which
                    // this code owns the reference.
                    let Some(proxy) = (unsafe { ComPtr::from_raw(data_object as *mut IDataObject) }) else {
                        return Ok(None);
                    };
                    let wrapper = OleDataObjectToDataTransferWrapper::new(&proxy);

                    if wrapper.try_formats()?.is_empty() {
                        wrapper.dispose();
                        return Ok(None);
                    }

                    let wrapper: Rc<dyn IAsyncDataTransfer> = wrapper;
                    return Ok(Some(wrapper));
                }

                i -= 1;
                if i == 0 {
                    return Err(hresult_error(hr, "The clipboard could not be read."));
                }

                delay(OLE_RETRY_DELAY).await;
            }
        })
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let last_stored = self.last_stored.clone();
        Box::pin(async move {
            Dispatcher::ui_thread().verify_access();

            let (wrapper, data_object) =
                DataTransferToOleDataObjectWrapper::new(data_transfer.to_synchronous(LogArea::WIN32_PLATFORM));
            let mut i = OLE_RETRY_COUNT;

            loop {
                let ptr = data_object.as_ptr();
                // SAFETY: a live data object, held by this future.
                let hr = unsafe { unmanaged_methods::ole_set_clipboard(ptr.cast()) };

                if hr == 0 {
                    *last_stored.data_object.borrow_mut() = Some(wrapper.clone());
                    last_stored.data_object_int_ptr.set(ptr as usize);
                    let stored = Rc::downgrade(&last_stored);
                    wrapper.on_destroyed(move || {
                        if let Some(stored) = stored.upgrade() {
                            if stored.data_object_int_ptr.get() == ptr as usize {
                                stored.clear();
                            }
                        }
                    });
                    break;
                }

                i -= 1;
                if i == 0 {
                    return Err(hresult_error(hr, "The clipboard could not be set."));
                }

                delay(OLE_RETRY_DELAY).await;
            }

            // The reference of this future is released here; the system
            // holds its own while the data is on the clipboard.
            drop(data_object);
            Ok(())
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let last_stored = self.last_stored.clone();
        Box::pin(async move {
            let _clipboard = open_clipboard_async().await?;
            empty_clipboard();
            last_stored.clear();
            Ok(())
        })
    }

    fn as_flushable_clipboard_impl(&self) -> Option<&dyn IFlushableClipboardImpl> {
        Some(self)
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }
}

impl IOwnedClipboardImpl for ClipboardImpl {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        let last_stored = &self.last_stored;
        let stored = last_stored.data_object.borrow().clone();
        let ptr = last_stored.data_object_int_ptr.get();
        let is_current = stored.is_some_and(|stored| !stored.is_disposed())
            && ptr != 0
            // SAFETY: the wrapper still has its data transfer, which it
            // gives up when its COM object is destroyed: the object at the
            // address is alive.
            && unsafe { unmanaged_methods::ole_is_current_clipboard(ptr as *mut std::ffi::c_void) } == HRESULT::S_OK;

        if !is_current {
            last_stored.clear();
        }

        Box::pin(std::future::ready(Ok(is_current)))
    }
}

impl IFlushableClipboardImpl for ClipboardImpl {
    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(async {
            delay(OLE_FLUSH_DELAY).await;

            // Retry OLE operations several times as mitigation for clipboard locking issues in TS sessions.

            let mut i = OLE_RETRY_COUNT;

            loop {
                let hr = ole_flush_clipboard();

                if hr == 0 {
                    break;
                }

                i -= 1;
                if i == 0 {
                    return Err(hresult_error(hr, "The clipboard could not be flushed."));
                }

                delay(OLE_RETRY_DELAY).await;
            }
            Ok(())
        })
    }
}
