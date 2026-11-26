//! The clipboard history of Windows (`Windows.ApplicationModel.DataTransfer.Clipboard`, Windows 10
//! version 1809 and newer), and the options of a copy, which are the registered clipboard formats
//! `CanIncludeInClipboardHistory` and `CanUploadToCloudClipboard` of the system.
//!
//! The history is the user's data: it is read only when the application asks.

use crate::win_rt::{
    HStringInterop, IAsyncInfo, IClipboardHistoryItem, IClipboardStatics2, IDataPackageView, NativeWinRTMethods,
};
use ferroui_base::input::platform::{
    ClipboardContentOptions, ClipboardError, ClipboardErrorKind, ClipboardHistoryItemsResult,
    ClipboardHistoryItemsResultStatus, IClipboardHistoryItem as FerroHistoryItem,
};
use ferroui_base::input::{DataFormat, DataTransferItem, IAsyncDataTransfer, IAsyncDataTransferItem, LocalBoxFuture};
use ferroui_base::reactive::{IDisposable, Subscription};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_microcom::ComPtr;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CLIPBOARD_CLASS: &str = "Windows.ApplicationModel.DataTransfer.Clipboard";
/// How often a pending operation of the Windows Runtime is looked at, on the UI thread.
const POLL_INTERVAL: Duration = Duration::from_millis(16);
/// The 100-nanosecond intervals between 1601-01-01 (a `DateTime` of the Windows Runtime) and the
/// Unix epoch.
const WINDOWS_TO_UNIX_EPOCH: i64 = 116_444_736_000_000_000;

fn statics() -> Option<ComPtr<IClipboardStatics2>> {
    NativeWinRTMethods::create_activation_factory::<IClipboardStatics2>(CLIPBOARD_CLASS).ok()
}

/// Whether the user has the clipboard history turned on; false before Windows 10 version 1809.
pub(crate) fn is_history_enabled() -> bool {
    statics().and_then(|statics| statics.is_history_enabled().ok()).is_some_and(|enabled| enabled != 0)
}

/// Whether the user has the synchronization of the clipboard turned on.
pub(crate) fn is_roaming_enabled() -> bool {
    statics().and_then(|statics| statics.is_roaming_enabled().ok()).is_some_and(|enabled| enabled != 0)
}

fn platform_error(code: i32) -> ClipboardError {
    ClipboardError::platform(code, "A call of the clipboard history of Windows failed")
}

/// Waits on the UI thread for an operation of the Windows Runtime, whose completion handler may
/// run on another thread: the status is looked at on ticks of the dispatcher.
async fn completed(info: ComPtr<IAsyncInfo>) -> Result<(), ClipboardError> {
    loop {
        let status = info.status().map_err(|error| platform_error(error.0))?;
        match status.0 {
            // Started.
            0 => Delay::new(POLL_INTERVAL).await,
            // Completed.
            1 => return Ok(()),
            // Canceled.
            2 => return Err(ClipboardError::new(ClipboardErrorKind::Canceled, "The clipboard history operation was canceled.")),
            _ => return Err(platform_error(info.error_code().unwrap_or(0x8000_4005_u32 as i32))),
        }
    }
}

/// Completes after `interval`, on the UI thread.
struct Delay {
    interval: Duration,
    fired: Rc<Cell<bool>>,
    timer: Option<Subscription>,
}

impl Delay {
    fn new(interval: Duration) -> Self {
        Self { interval, fired: Rc::new(Cell::new(false)), timer: None }
    }
}

impl Future for Delay {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.fired.get() {
            return Poll::Ready(());
        }
        if self.timer.is_none() {
            let fired = self.fired.clone();
            let waker = cx.waker().clone();
            let interval = self.interval;
            self.timer = Some(DispatcherTimer::run_once(
                move || {
                    fired.set(true);
                    waker.wake_by_ref();
                },
                interval,
                DispatcherPriority::BACKGROUND,
            ));
        }
        Poll::Pending
    }
}

/// Reads the history.
pub(crate) fn get_history_items_async() -> LocalBoxFuture<Result<ClipboardHistoryItemsResult, ClipboardError>> {
    Box::pin(async {
        let Some(statics) = statics() else {
            return Ok(ClipboardHistoryItemsResult::empty(ClipboardHistoryItemsResultStatus::NotSupported));
        };
        let operation = statics
            .get_history_items_async()
            .map_err(|error| platform_error(error.0))?
            .ok_or_else(|| platform_error(0x8000_4003_u32 as i32))?;
        completed(operation.cast::<IAsyncInfo>().map_err(|error| platform_error(error.0))?).await?;
        let Some(result) = operation.get_results().map_err(|error| platform_error(error.0))? else {
            return Ok(ClipboardHistoryItemsResult::empty(ClipboardHistoryItemsResultStatus::Success));
        };
        // ClipboardHistoryItemsResultStatus: Success 0, AccessDenied 1, ClipboardHistoryDisabled 2.
        let status = match result.status().map_err(|error| platform_error(error.0))? {
            0 => ClipboardHistoryItemsResultStatus::Success,
            1 => ClipboardHistoryItemsResultStatus::AccessDenied,
            _ => ClipboardHistoryItemsResultStatus::ClipboardHistoryDisabled,
        };
        if status != ClipboardHistoryItemsResultStatus::Success {
            return Ok(ClipboardHistoryItemsResult::empty(status));
        }
        let mut items: Vec<Rc<dyn FerroHistoryItem>> = Vec::new();
        if let Some(list) = result.items().map_err(|error| platform_error(error.0))? {
            let size = list.size().map_err(|error| platform_error(error.0))?;
            for index in 0..size {
                if let Some(item) = list.get_at(index).map_err(|error| platform_error(error.0))? {
                    items.push(Rc::new(HistoryItem { item }));
                }
            }
        }
        Ok(ClipboardHistoryItemsResult { status, items })
    })
}

/// An entry of the history; its content is read when asked for.
struct HistoryItem {
    item: ComPtr<IClipboardHistoryItem>,
}

impl HistoryItem {
    fn content(&self) -> Option<ComPtr<IDataPackageView>> {
        self.item.content().ok().flatten()
    }
}

impl FerroHistoryItem for HistoryItem {
    fn id(&self) -> String {
        self.item.id().ok().and_then(|id| HStringInterop::from_handle(id, true).value()).unwrap_or_default()
    }

    fn timestamp(&self) -> Option<SystemTime> {
        let ticks = self.item.timestamp().ok()?.checked_sub(WINDOWS_TO_UNIX_EPOCH)?;
        let ticks = u64::try_from(ticks).ok()?;
        UNIX_EPOCH.checked_add(Duration::from_nanos(ticks.checked_mul(100)?))
    }

    fn formats(&self) -> Vec<String> {
        let Some(formats) = self.content().and_then(|content| content.available_formats().ok().flatten()) else {
            return Vec::new();
        };
        let size = formats.size().unwrap_or(0);
        (0..size)
            .filter_map(|index| formats.get_at(index).ok())
            .filter_map(|format| HStringInterop::from_handle(format, true).value())
            .collect()
    }

    fn get_text_async(&self) -> LocalBoxFuture<Result<Option<String>, ClipboardError>> {
        let content = self.content();
        Box::pin(async move {
            let Some(content) = content else { return Ok(None) };
            // StandardDataFormats.Text.
            let text_format = HStringInterop::new(Some("Text")).map_err(|error| platform_error(error.0))?;
            if content.contains(text_format.handle()).map_err(|error| platform_error(error.0))? == 0 {
                return Ok(None);
            }
            let operation = content
                .get_text_async()
                .map_err(|error| platform_error(error.0))?
                .ok_or_else(|| platform_error(0x8000_4003_u32 as i32))?;
            completed(operation.cast::<IAsyncInfo>().map_err(|error| platform_error(error.0))?).await?;
            let text = operation.get_results().map_err(|error| platform_error(error.0))?;
            Ok(HStringInterop::from_handle(text, true).value())
        })
    }
}

/// The data to copy with the formats of the system that keep it out of the history or the cloud.
pub(crate) fn with_options(
    data_transfer: Rc<dyn IAsyncDataTransfer>,
    options: ClipboardContentOptions,
) -> Rc<dyn IAsyncDataTransfer> {
    if options.is_allowed_in_history && options.is_roaming_allowed {
        return data_transfer;
    }
    let item = DataTransferItem::new();
    // A DWORD of 0 for a format the data must not go to.
    let no: Rc<[u8]> = Rc::from(0u32.to_le_bytes().as_slice());
    if !options.is_allowed_in_history {
        item.set(&DataFormat::create_bytes_platform_format("CanIncludeInClipboardHistory"), Some(no.clone()));
    }
    if !options.is_roaming_allowed {
        item.set(&DataFormat::create_bytes_platform_format("CanUploadToCloudClipboard"), Some(no));
    }
    Rc::new(WithOptions { inner: data_transfer, extra: item, items: RefCell::new(None) })
}

/// A data transfer with the item of the options added.
struct WithOptions {
    inner: Rc<dyn IAsyncDataTransfer>,
    extra: Rc<DataTransferItem>,
    items: RefCell<Option<Rc<[Rc<dyn IAsyncDataTransferItem>]>>>,
}

impl IDisposable for WithOptions {
    fn dispose(&self) {
        self.inner.dispose();
    }
}

impl IAsyncDataTransfer for WithOptions {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.inner.formats().iter().cloned().chain(self.extra.formats().iter().cloned()).collect()
    }

    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.items
            .borrow_mut()
            .get_or_insert_with(|| {
                let extra: Rc<dyn IAsyncDataTransferItem> = self.extra.clone();
                self.inner.items().iter().cloned().chain(std::iter::once(extra)).collect()
            })
            .clone()
    }

    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        Ok(self.inner.try_formats()?.iter().cloned().chain(self.extra.formats().iter().cloned()).collect())
    }

    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        let extra: Rc<dyn IAsyncDataTransferItem> = self.extra.clone();
        Ok(self.inner.try_items()?.iter().cloned().chain(std::iter::once(extra)).collect())
    }
}
