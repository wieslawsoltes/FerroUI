//! The clipboard over the selection of a data device (the port of
//! `WaylandClipboardImpl.cs`).

use super::wayland_data_transfer::WaylandDataTransfer;
use super::wayland_mime_mapper;
use super::wayland_outgoing_transfer::WaylandOutgoingTransfer;
use crate::server::wayland_worker::WaylandWorker;
use ferroui_base::input::platform::{with_chromium_clipboard_formats, ClipboardError, IClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{AsyncDataTransferExtensions, DataFormat, DataTransfer, IAsyncDataTransfer, LocalBoxFuture};
use ferroui_base::media::imaging::{Bitmap, PngBitmapEncoderOptions};
use ferroui_x11::selections::uri_list_helper;
use std::rc::Rc;
use std::sync::Arc;

/// Implements the owned clipboard for the Wayland backend.
/// All Wayland protocol calls happen on the Wayland thread via the worker.
/// Data I/O (pipe reads/writes) happens on threads of its own.
/// No shared mutable state between threads — all cross-thread communication via messaging.
pub struct WaylandClipboardImpl {
    worker: Arc<WaylandWorker>,
}

impl WaylandClipboardImpl {
    pub fn new(worker: Arc<WaylandWorker>) -> Rc<Self> {
        Rc::new(Self { worker })
    }
}

impl IClipboardImpl for WaylandClipboardImpl {
    fn try_get_data_async(&self) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        let selection_info = self.worker.invoke_oob_async(|worker| {
            let device = worker.state.globals.as_ref()?.input_dispatcher.get_data_device()?;
            device.get_selection_info()
        });

        Box::pin(async move {
            let Some((cookie, mime_types)) = selection_info.await.flatten() else {
                return Ok(None);
            };
            if mime_types.is_empty() {
                return Ok(None);
            }

            let transfer = WaylandDataTransfer::for_clipboard(mime_types, cookie).await;
            Ok(transfer.map(|transfer| transfer as Rc<dyn IAsyncDataTransfer>))
        })
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let worker = self.worker.clone();
        Box::pin(async move {
            let transfer = with_chromium_clipboard_formats(data_transfer)?;
            WaylandOutgoingTransfer::new(transfer).set_as_selection(&worker).await;
            Ok(())
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_data_async(DataTransfer::new())
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }
}

impl IOwnedClipboardImpl for WaylandClipboardImpl {
    fn is_current_owner_async(&self) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        let is_owner = self.worker.invoke_oob_async(|worker| {
            let device = worker.state.globals.as_ref().and_then(|globals| globals.input_dispatcher.get_data_device());
            device.is_some_and(|device| device.active_source.is_some())
        });
        Box::pin(async move { Ok(is_owner.await.unwrap_or(false)) })
    }
}

/// Serializes data from a data transfer for a specific MIME type. `None` if no item
/// provides the format.
///
/// The reference writes into the stream of the pipe as it goes, a bitmap from a pool
/// thread. A bitmap of the port belongs to the UI thread, so the bytes are made here and
/// the caller writes them (DEVIATIONS.md).
pub(crate) async fn serialize_for_mime_async(transfer: &Rc<dyn IAsyncDataTransfer>, mime_type: &str) -> Option<Vec<u8>> {
    let format = wayland_mime_mapper::from_mime_type(mime_type)?;

    // Files span multiple items; gather them all and serialize as a text/uri-list.
    if DataFormat::file() == format {
        return match transfer.try_get_files_async().await {
            Ok(Some(files)) if !files.is_empty() => Some(uri_list_helper::file_uri_list_to_utf8_bytes(&files)),
            _ => None,
        };
    }

    for item in transfer.items().iter() {
        let Ok(Some(value)) = item.try_get_raw_async(&format).await else {
            continue;
        };

        if DataFormat::text() == format {
            if let Some(text) = value.downcast_ref::<String>() {
                return Some(text.as_bytes().to_vec());
            }
        } else if DataFormat::bitmap() == format {
            if let Some(bitmap) = value.downcast_ref::<Rc<Bitmap>>() {
                let mut bytes = Vec::new();
                return bitmap.save(&mut bytes, &PngBitmapEncoderOptions::DEFAULT.into()).ok().map(|()| bytes);
            }
        } else if let Some(bytes) = value.downcast_ref::<Rc<[u8]>>() {
            return Some(bytes.to_vec());
        } else if let Some(bytes) = value.downcast_ref::<Vec<u8>>() {
            return Some(bytes.clone());
        } else if let Some(text) = value.downcast_ref::<String>() {
            return Some(text.as_bytes().to_vec());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::input::platform::chromium_clipboard::ChromiumClipboard;
    use ferroui_base::input::{DataFormatKind, DataTransferItem};
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    #[test]
    fn wayland_sends_the_map_and_exact_payload_through_the_mime_serializer() {
        let clipboard = ChromiumClipboard::Linux;
        let text = DataFormat::create_string_application_format("RectangularSelection");
        let bytes = DataFormat::create_bytes_application_format("Binary");
        let first_slot = r#"application/web;type="custom/format0""#;
        let item = DataTransferItem::new();
        item.set(&DataFormat::text(), Some("columns".to_owned()));
        item.set(&text, Some("hé".to_owned()));
        item.set(&bytes, Some(Rc::from([0, 2, 0].as_slice())));
        item.set(&DataFormat::create_bytes_platform_format(first_slot), Some(Rc::from([42].as_slice())));
        let transfer = DataTransfer::new();
        transfer.add(item);
        let transfer = with_chromium_clipboard_formats(transfer).unwrap();
        let serialize = |mime: &str| {
            let mut future = std::pin::pin!(serialize_for_mime_async(&transfer, mime));
            match future.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
                Poll::Ready(value) => value.unwrap(),
                Poll::Pending => panic!("an in-memory clipboard item should be ready"),
            }
        };
        let map = serialize(clipboard.map_format().identifier());
        // A browser offers only the map and slots; ignore the native application names
        // when decoding this fixture.
        let advertised = transfer.formats().iter().filter(|format| format.kind() == DataFormatKind::Platform).cloned().collect::<Vec<_>>();
        let aliases = clipboard.parse_map(&map, &advertised);
        assert_eq!(aliases.len(), 2);
        let payloads: [(DataFormat, &[u8]); 2] = [(text.into(), "hé".as_bytes()), (bytes.into(), &[0, 2, 0])];
        for (format, payload) in payloads {
            let alias = aliases.iter().find(|alias| alias.application == format).unwrap();
            assert_eq!(serialize(alias.slot.identifier()), payload);
        }
        assert_eq!(serialize(first_slot), [42]);
        assert_eq!(serialize("text/plain"), b"columns");
    }
}
