//! The clipboard over the selection of a data device (the port of
//! `WaylandClipboardImpl.cs`).

use super::wayland_data_transfer::WaylandDataTransfer;
use super::wayland_mime_mapper;
use super::wayland_outgoing_transfer::WaylandOutgoingTransfer;
use crate::server::wayland_worker::WaylandWorker;
use crate::server::wayland_marshallers::UiThreadRef;
use ferroui_base::input::platform::{ClipboardError, IClipboardChangeImpl, IClipboardImpl, IOwnedClipboardImpl};
use ferroui_base::input::{AsyncDataTransferExtensions, DataFormat, DataTransfer, IAsyncDataTransfer, LocalBoxFuture};
use ferroui_base::media::imaging::{Bitmap, PngBitmapEncoderOptions};
use ferroui_x11::selections::uri_list_helper;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::HandlerList;
use std::rc::Rc;
use std::sync::Arc;

/// Implements the owned clipboard for the Wayland backend.
/// All Wayland protocol calls happen on the Wayland thread via the worker.
/// Data I/O (pipe reads/writes) happens on threads of its own.
/// No shared mutable state between threads — all cross-thread communication via messaging.
pub struct WaylandClipboardImpl {
    worker: Arc<WaylandWorker>,
    changes: Rc<ClipboardChanges>,
}

impl WaylandClipboardImpl {
    pub fn new(worker: Arc<WaylandWorker>) -> Rc<Self> {
        Rc::new(Self { worker, changes: Rc::new(ClipboardChanges { handlers: HandlerList::new() }) })
    }
}

/// The handlers of the changes of the selection, on the UI thread; the worker reaches them
/// through a [`UiThreadRef`].
pub(crate) struct ClipboardChanges {
    handlers: HandlerList<dyn Fn()>,
}

impl ClipboardChanges {
    pub(crate) fn raise(&self) {
        for (_, handler) in self.handlers.snapshot().iter() {
            handler();
        }
    }
}

impl IClipboardChangeImpl for WaylandClipboardImpl {
    // The compositor sends the selection to a client that has the keyboard focus: a change made
    // while the application has no focus is reported when it gets it.
    fn subscribe_content_changed(&self, handler: Rc<dyn Fn()>) -> Subscription {
        let first = self.changes.handlers.is_empty();
        let token = self.changes.handlers.add(handler);
        if first {
            self.worker.set_clipboard_observer(Some(UiThreadRef::new(self.changes.clone(), Dispatcher::ui_thread())));
        }
        let changes = Rc::downgrade(&self.changes);
        let worker = self.worker.clone();
        Subscription::new(Disposable::create(move || {
            let Some(changes) = changes.upgrade() else { return };
            changes.handlers.remove(token);
            if changes.handlers.is_empty() {
                worker.set_clipboard_observer(None);
            }
        }))
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

            // Build the item layout up front (one item per file + a shared item for the rest), reading
            // the file URI list off the calling thread so item access later doesn't block.
            let transfer = WaylandDataTransfer::new(mime_types, cookie);
            transfer.preload_async().await;
            Ok(Some(transfer as Rc<dyn IAsyncDataTransfer>))
        })
    }

    fn set_data_async(&self, data_transfer: Rc<dyn IAsyncDataTransfer>) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let outgoing = WaylandOutgoingTransfer::new(data_transfer);
        let set = outgoing.set_as_selection(&self.worker);
        Box::pin(async move {
            set.await;
            Ok(())
        })
    }

    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.set_data_async(DataTransfer::new())
    }

    fn as_owned_clipboard_impl(&self) -> Option<&dyn IOwnedClipboardImpl> {
        Some(self)
    }

    fn as_clipboard_change_impl(&self) -> Option<&dyn IClipboardChangeImpl> {
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
