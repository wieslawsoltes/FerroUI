//! The clipboard over a pasteboard of UIKit.

use crate::clipboard::clipboard_data_format_helper::to_system_type;
use crate::clipboard::pasteboard_to_data_transfer_wrapper::PasteboardToDataTransferWrapper;
use ferroui_base::input::platform::{ClipboardError, IClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{
    AsyncDataTransferItemExtensions, DataFormat, DataFormatKind, IAsyncDataTransfer, IAsyncDataTransferItem,
    LocalBoxFuture,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::reactive::IDisposable;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSData, NSDictionary, NSMutableDictionary, NSString};
use objc2_ui_kit::{UIImage, UIPasteboard};
use std::cell::Cell;
use std::rc::Rc;

struct State {
    pasteboard: Retained<UIPasteboard>,
    last_change_count: Cell<isize>,
}

impl State {
    fn change_count(&self) -> isize {
        // SAFETY: a property of the pasteboard, read on the thread of
        // the clipboard of the top-level, the main thread.
        unsafe { self.pasteboard.changeCount() }
    }

    fn set_items(&self, items: &NSArray<NSDictionary<NSString, AnyObject>>) {
        // SAFETY: the keys of the items are types and the values are
        // strings, data or images, which the pasteboard takes.
        unsafe { self.pasteboard.setItems(items) };
        self.last_change_count.set(self.change_count());
    }
}

/// The clipboard of iOS.
pub struct ClipboardImpl {
    state: Rc<State>,
}

impl ClipboardImpl {
    /// Creates the clipboard over `pasteboard`.
    pub fn new(pasteboard: Retained<UIPasteboard>) -> Self {
        Self { state: Rc::new(State { pasteboard, last_change_count: Cell::new(isize::MIN) }) }
    }

    fn try_get_data(&self) -> Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError> {
        let data_transfer =
            PasteboardToDataTransferWrapper::new(self.state.pasteboard.clone(), self.state.change_count());

        if data_transfer.try_formats()?.is_empty() {
            data_transfer.dispose();
            return Ok(None);
        }

        Ok(Some(data_transfer))
    }

    async fn try_create_pasteboard_item_async(
        data_transfer_item: &Rc<dyn IAsyncDataTransferItem>,
    ) -> Result<Option<Retained<NSDictionary<NSString, AnyObject>>>, ClipboardError> {
        let mut pasteboard_item: Option<Retained<NSMutableDictionary<NSString, AnyObject>>> = None;

        for data_format in data_transfer_item.try_formats()?.iter() {
            if data_format.kind() == DataFormatKind::InProcess {
                continue;
            }

            let Some(data) = Self::try_get_foundation_data_async(data_transfer_item, data_format).await? else {
                continue;
            };

            let type_ = to_system_type(data_format);
            pasteboard_item
                .get_or_insert_with(NSMutableDictionary::new)
                .insert(&*NSString::from_str(&type_), &*data);
        }

        Ok(pasteboard_item.map(Retained::into_super))
    }

    async fn try_get_foundation_data_async(
        data_transfer_item: &Rc<dyn IAsyncDataTransferItem>,
        format: &DataFormat,
    ) -> Result<Option<Retained<AnyObject>>, ClipboardError> {
        if *format == DataFormat::text() {
            let text = data_transfer_item.try_get_value_async(&DataFormat::text()).await?.unwrap_or_default();
            return Ok(Some(NSString::from_str(&text).into()));
        }

        if *format == DataFormat::file() {
            let file = data_transfer_item.try_get_value_async(&DataFormat::file()).await?;
            return Ok(file.map(|file| NSString::from_str(file.path().absolute_uri()).into()));
        }

        if *format == DataFormat::bitmap() {
            let Some(bitmap) = data_transfer_item.try_get_value_async(&DataFormat::bitmap()).await? else {
                return Ok(None);
            };
            let mut memory_stream = Vec::new();
            bitmap
                .save(&mut memory_stream, &PngBitmapEncoderOptions::default().into())
                .map_err(|error| ClipboardError::other(format!("Unable to save the bitmap: {error}")))?;
            let data = NSData::with_bytes(&memory_stream);
            return Ok(UIImage::imageWithData(&data).map(|image| Retained::into_super(Retained::into_super(image))));
        }

        // The reference asks the format for the type of its values; a
        // format of the port does not carry one, so the value decides.
        let Some(value) = data_transfer_item.try_get_raw_async(format).await? else {
            return Ok(None);
        };

        if let Some(string_value) = value.downcast_ref::<String>() {
            return Ok(Some(NSString::from_str(string_value).into()));
        }

        if let Some(bytes) = value.downcast_ref::<Rc<[u8]>>() {
            return Ok(Some(NSData::with_bytes(bytes).into()));
        }

        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::IOS_PLATFORM) {
            logger.log_with_values(None, "Unsupported data format {Format}", &[format]);
        }

        Ok(None)
    }
}

impl IClipboardImpl for ClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(std::future::ready(self.try_get_data()))
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let state = self.state.clone();
        Box::pin(async move {
            let mut pasteboard_items: Vec<Retained<NSDictionary<NSString, AnyObject>>> = Vec::new();

            for data_transfer_item in data_transfer.try_items()?.iter() {
                if let Some(pasteboard_item) = Self::try_create_pasteboard_item_async(data_transfer_item).await? {
                    pasteboard_items.push(pasteboard_item);
                }
            }

            state.set_items(&NSArray::from_retained_slice(&pasteboard_items));
            Ok(())
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.state.set_items(&NSArray::new());
        Box::pin(std::future::ready(Ok(())))
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }
}

impl IOwnedClipboardImpl for ClipboardImpl {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        Box::pin(std::future::ready(Ok(self.state.last_change_count.get() == self.state.change_count())))
    }
}
