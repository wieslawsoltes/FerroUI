//! The token of an offer as the UI thread holds it (the port of
//! `WaylandOfferCookie.cs`).
//!
//! The reference cookie holds the data device and the offer wrapper, which
//! are objects of the Wayland thread. Here it holds the protocol object of
//! the offer (a handle any thread may hold) and the registry name of the
//! seat whose data device has the offer; the device is looked up on the
//! Wayland thread.

use super::wayland_data_offer;
use crate::server::cross_thread_result::CrossThreadResult;
use crate::server::wayland_worker::WaylandWorker;
use ferroui_base::input::DragDropEffects;
use std::os::fd::OwnedFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use wayland_client::protocol::wl_data_offer::WlDataOffer;

/// Opaque token representing a specific offer on the Wayland thread.
/// Passed to UI-thread code as a shared reference.
///
/// For data reads the UI thread sends this cookie back to the Wayland thread
/// via `invoke_oob_async`, and the cookie performs the receive internally.
/// For DnD sessions this cookie also carries feedback methods ([`post_update_feedback`],
/// [`post_finish`]) that route through `post_oob`.
///
/// The offer is used on the Wayland thread only — the UI thread only holds a reference.
///
/// [`post_update_feedback`]: Self::post_update_feedback
/// [`post_finish`]: Self::post_finish
pub struct WaylandOfferCookie {
    seat: u32,
    offer: WlDataOffer,
    worker: Option<Arc<WaylandWorker>>,
    invalidated: AtomicBool,
}

impl WaylandOfferCookie {
    pub(crate) fn new(seat: u32, offer: WlDataOffer, worker: Option<Arc<WaylandWorker>>) -> Arc<Self> {
        Arc::new(Self { seat, offer, worker, invalidated: AtomicBool::new(false) })
    }

    /// Called from the Wayland thread when this offer is superseded by a new selection
    /// or when the device is disposed.
    pub(crate) fn invalidate(&self) {
        self.invalidated.store(true, Ordering::Release);
    }

    pub(crate) fn is_invalidated(&self) -> bool {
        self.invalidated.load(Ordering::Acquire)
    }

    /// Attempts to receive data for a MIME type from the underlying offer.
    /// Returns the read-end pipe fd, or `None` if the offer has been invalidated.
    /// Must be called on the Wayland thread.
    pub(crate) fn try_receive(&self, mime_type: &str) -> Option<OwnedFd> {
        if self.is_invalidated() {
            return None;
        }
        wayland_data_offer::receive(&self.offer, mime_type)
    }

    /// Asynchronously receives data for a MIME type by posting to the Wayland thread.
    /// Returns the read-end pipe fd, or `None` if the offer has been invalidated.
    /// Called from the UI thread.
    pub(crate) fn try_receive_async(self: &Arc<Self>, mime_type: &str) -> CrossThreadResult<Option<OwnedFd>> {
        let Some(worker) = &self.worker else {
            return CrossThreadResult::ready(Some(None));
        };
        let this = self.clone();
        let mime_type = mime_type.to_string();
        worker.invoke_oob_async(move |_| this.try_receive(&mime_type))
    }

    /// Posts a DnD feedback update to the Wayland thread.
    /// Called from UI-thread code after processing a DragEnter or DragOver event.
    pub(crate) fn post_update_feedback(self: &Arc<Self>, effects: DragDropEffects) {
        let Some(worker) = &self.worker else {
            return;
        };
        let this = self.clone();
        worker.post_oob(Box::new(move |worker| {
            if !this.is_invalidated() {
                if let Some(device) = worker.state.globals.as_mut().and_then(|globals| globals.input_dispatcher.data_device_of_mut(this.seat)) {
                    device.update_dnd_feedback(effects);
                }
            }
        }));
    }

    /// Posts a DnD finish signal to the Wayland thread.
    /// Called from UI-thread code after a successful drop.
    pub(crate) fn post_finish(self: &Arc<Self>) {
        let Some(worker) = &self.worker else {
            return;
        };
        let this = self.clone();
        worker.post_oob(Box::new(move |worker| {
            if !this.is_invalidated() {
                if let Some(device) = worker.state.globals.as_mut().and_then(|globals| globals.input_dispatcher.data_device_of_mut(this.seat)) {
                    device.finish_dnd();
                }
            }
        }));
    }
}
