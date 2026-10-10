//! The clipboard of the system: text.
//!
//! The reference puts an OLE data object on the clipboard and reads the
//! data object that is there, which carries every format a data transfer
//! can hold (text, files, bitmaps, the formats of applications). The data
//! objects need the COM interop of the backend, which is stage 2. This
//! stage builds the clipboard for the one format every application starts
//! with, Unicode text, through the clipboard functions of the system; a
//! data transfer without text cannot be put on the clipboard yet and fails
//! with a message that says so. The retry discipline of the reference (the
//! clipboard may be held open by another process for a moment) is kept.

use crate::interop::unmanaged_methods::{
    close_clipboard, empty_clipboard, get_clipboard_unicode_text, get_last_error, open_clipboard,
    set_clipboard_unicode_text,
};
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl};
use ferroui_base::input::{
    DataTransfer, DataTransferExtensions, DataTransferItem, IAsyncDataTransfer, IDataTransfer, LocalBoxFuture,
};
use ferroui_base::logging::LogArea;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

const OLE_RETRY_COUNT: i32 = 10;
const OLE_RETRY_DELAY: u64 = 100;

/// The system clipboard.
pub struct ClipboardImpl;

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
/// holds it. `owner` is the window that owns the clipboard once it is
/// emptied (0 for a caller that only reads or clears).
async fn open_clipboard_async(owner: isize) -> Result<OpenClipboard, ClipboardError> {
    let mut i = OLE_RETRY_COUNT;

    while !open_clipboard(owner) {
        i -= 1;
        if i == 0 {
            return Err(ClipboardError::timeout("Timeout opening clipboard."));
        }
        delay(OLE_RETRY_DELAY).await;
    }

    Ok(OpenClipboard)
}

impl ClipboardImpl {
    /// Creates the clipboard.
    #[allow(clippy::new_without_default)]
    pub fn new() -> ClipboardImpl {
        ClipboardImpl
    }
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(async {
            let text = {
                let _clipboard = open_clipboard_async(0).await?;
                get_clipboard_unicode_text()
            };

            // A clipboard without a format this stage reads has nothing to
            // offer, as a data object without formats in the reference.
            let Some(text) = text else {
                return Ok(None);
            };

            let data_transfer = DataTransfer::new();
            data_transfer.add(DataTransferItem::create_text(Some(&text)));
            let data_transfer: Rc<dyn IDataTransfer> = data_transfer;
            Ok(Some(data_transfer.to_asynchronous()))
        })
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(async move {
            let data_transfer = data_transfer.to_synchronous(LogArea::WIN32_PLATFORM);
            let Some(text) = data_transfer.try_get_text() else {
                return Err(ClipboardError::other(
                    "Only text can be put on the clipboard: the data objects of the Windows platform backend are \
                     stage 2 (docs/porting/win32-platform.md).",
                ));
            };

            // The message window of the platform owns the text: the system
            // takes no data from a thread that emptied the clipboard
            // without an owner.
            let owner = crate::win32_platform::Win32Platform::instance().handle();
            let _clipboard = open_clipboard_async(owner).await?;
            if !empty_clipboard() || !set_clipboard_unicode_text(&text) {
                return Err(ClipboardError::platform(get_last_error() as i32, "The clipboard could not be set."));
            }
            Ok(())
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(async {
            let _clipboard = open_clipboard_async(0).await?;
            empty_clipboard();
            Ok(())
        })
    }
}
