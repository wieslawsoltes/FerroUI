//! The data device of a seat (the port of `WaylandDataDevice.cs`): the
//! selection of the clipboard and the drags that enter the surfaces of the
//! application.
//!
//! The listener class of the reference (`WaylandDataDeviceListener`) is the
//! `Dispatch` implementation of this file; the user data of a data device is
//! the registry name of its seat. The source the device set as the selection
//! is kept by the globals (`WaylandGlobals::data_sources`), and the device
//! names it by the identity of its protocol object.

use super::wayland_data_offer::WaylandDataOffer;
use super::wayland_data_source::WaylandDataSource;
use super::wayland_offer_cookie::WaylandOfferCookie;
use crate::server::persistent::i_w_surface_event_sink::WSurfaceEventSinkProxy;
use crate::server::persistent::wayland_input_event_cookie::WaylandInputEventCookie;
use crate::server::transient::wayland_globals::WaylandGlobals;
use crate::server::transient::wayland_input_dispatcher::{find_surface_for_wl_surface, with_seat, InputContext};
use crate::server::wayland_worker::{WaylandWorker, WaylandWorkerState};
use ferroui_base::input::{DragDropEffects, RawInputModifiers};
use ferroui_base::Point;
use std::collections::VecDeque;
use std::sync::Arc;
use wayland_client::backend::ObjectId;
use wayland_client::protocol::wl_data_device::{self, WlDataDevice};
use wayland_client::protocol::wl_data_device_manager::DndAction;
use wayland_client::protocol::wl_data_offer::WlDataOffer;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{event_created_child, Connection, Dispatch, QueueHandle};

/// Pending drops: offer+cookie moved here on drop(), consumed by FinishDnd from UI thread
struct PendingDrop {
    offer: WaylandDataOffer,
    cookie: Arc<WaylandOfferCookie>,
}

/// Per-seat `wl_data_device` wrapper. Handles clipboard selection events and DnD events.
/// Lives on the Wayland thread.
pub struct WaylandDataDevice {
    device: WlDataDevice,
    seat: u32,
    worker: Arc<WaylandWorker>,

    /// The most recently announced data offer (not yet assigned to selection or DnD).
    /// Set by the `data_offer` event, consumed by `selection` or `enter`.
    pending_offer: Option<WaylandDataOffer>,

    /// Cookie for the current selection offer.
    /// Encapsulates the offer and validates staleness on reads.
    selection_cookie: Option<Arc<WaylandOfferCookie>>,

    // DnD session state
    dnd_offer: Option<WaylandDataOffer>,
    dnd_cookie: Option<Arc<WaylandOfferCookie>>,
    dnd_serial: u32,
    dnd_sink: Option<WSurfaceEventSinkProxy>,
    dnd_position: Point,

    pending_drop_operations: VecDeque<PendingDrop>,

    /// The current clipboard selection offer.
    /// Valid until a new `selection` event replaces it or keyboard focus is lost.
    selection_offer: Option<WaylandDataOffer>,

    /// The currently active source we set via `set_selection`.
    /// `None` if we don't own the clipboard.
    pub active_source: Option<ObjectId>,

    /// The serial from the most recent input event that can authorize `set_selection`
    /// (keyboard enter/key, pointer button, touch down).
    pub last_input_serial: u32,
}

impl WaylandDataDevice {
    pub fn new(device: WlDataDevice, seat: u32, worker: Arc<WaylandWorker>) -> Self {
        Self {
            device,
            seat,
            worker,
            pending_offer: None,
            selection_cookie: None,
            dnd_offer: None,
            dnd_cookie: None,
            dnd_serial: 0,
            dnd_sink: None,
            dnd_position: Point::default(),
            pending_drop_operations: VecDeque::new(),
            selection_offer: None,
            active_source: None,
            last_input_serial: 0,
        }
    }

    /// The registry name of the seat of the device.
    pub fn seat(&self) -> u32 {
        self.seat
    }

    pub fn selection_offer(&self) -> Option<&WaylandDataOffer> {
        self.selection_offer.as_ref()
    }

    /// The offer of the identity `id`, wherever the device keeps it.
    pub(crate) fn find_offer_mut(&mut self, id: &ObjectId) -> Option<&mut WaylandDataOffer> {
        let Self { pending_offer, selection_offer, dnd_offer, pending_drop_operations, .. } = self;
        pending_offer
            .iter_mut()
            .chain(selection_offer.iter_mut())
            .chain(dnd_offer.iter_mut())
            .chain(pending_drop_operations.iter_mut().map(|pending| &mut pending.offer))
            .find(|offer| offer.id() == *id)
    }

    /// Sets the clipboard selection to the given source: the request of the device. The
    /// source that was the selection before is returned for the caller to dispose (see
    /// [`set_selection`], which is the member of the reference).
    fn replace_selection(&mut self, source: Option<&WaylandDataSource>) -> Option<ObjectId> {
        let previous = std::mem::replace(&mut self.active_source, source.map(WaylandDataSource::id));
        self.device.set_selection(source.map(WaylandDataSource::wl_source), self.last_input_serial);
        previous
    }

    /// Gets the cookie and MIME type list for the current selection offer.
    /// Returns `None` if there is no valid selection.
    /// Called from the Wayland thread on behalf of UI-thread queries.
    pub fn get_selection_info(&self) -> Option<(Arc<WaylandOfferCookie>, Vec<String>)> {
        let selection_offer = self.selection_offer.as_ref()?;
        if selection_offer.mime_types().is_empty() {
            return None;
        }
        Some((self.selection_cookie.clone()?, selection_offer.mime_types().to_vec()))
    }

    fn on_data_offer(&mut self, wl_offer: WlDataOffer) {
        // An offer nobody took is replaced: the reference leaves it to the collector of its
        // runtime, here its protocol object is destroyed.
        if let Some(stale) = self.pending_offer.replace(WaylandDataOffer::new(wl_offer)) {
            stale.dispose();
        }
    }

    fn on_selection(&mut self, wl_offer: Option<WlDataOffer>) {
        if let Some(cookie) = self.selection_cookie.take() {
            cookie.invalidate();
        }
        if let Some(offer) = self.selection_offer.take() {
            offer.dispose();
        }

        match (wl_offer, self.pending_offer.take()) {
            (Some(_), Some(pending_offer)) => {
                self.selection_cookie =
                    Some(WaylandOfferCookie::new(self.seat, pending_offer.wl_offer().clone(), Some(self.worker.clone())));
                self.selection_offer = Some(pending_offer);
            }
            (_, pending_offer) => {
                if let Some(pending_offer) = pending_offer {
                    pending_offer.dispose();
                }
            }
        }
        // A new selection (or none): the clipboard's content changed.
        self.worker.notify_selection_changed();
    }

    fn on_dnd_enter(
        &mut self,
        serial: u32,
        surface: Option<&WlSurface>,
        x: f64,
        y: f64,
        keyboard_modifiers: RawInputModifiers,
        cx: &InputContext<'_>,
    ) {
        let Some(offer) = self.pending_offer.take() else {
            return;
        };

        let Some(sink) = surface.and_then(find_surface_for_wl_surface).and_then(|shell_surface| cx.sink_of(shell_surface)) else {
            offer.dispose();
            return;
        };

        // Clean up any lingering DnD session
        self.cleanup_dnd_state();

        let cookie = WaylandOfferCookie::new(self.seat, offer.wl_offer().clone(), Some(self.worker.clone()));
        let source_effects = actions_to_effects(offer.source_actions);
        let mime_types = offer.mime_types().to_vec();

        self.dnd_offer = Some(offer);
        self.dnd_cookie = Some(cookie.clone());
        self.dnd_serial = serial;
        self.dnd_position = Point::new(x, y);

        sink.on_drag_enter(self.dnd_position, mime_types, cookie, source_effects, keyboard_modifiers);
        self.dnd_sink = Some(sink);
    }

    fn on_dnd_motion(&mut self, x: f64, y: f64, keyboard_modifiers: RawInputModifiers) {
        self.dnd_position = Point::new(x, y);
        if let Some(sink) = &self.dnd_sink {
            sink.on_drag_motion(self.dnd_position, keyboard_modifiers);
        }
    }

    fn on_dnd_leave(&mut self) {
        // If active session exists (no preceding drop), notify and clean up.
        // After drop(), the sink is already gone so this is a no-op.
        if let Some(sink) = &self.dnd_sink {
            sink.on_drag_leave();
        }
        self.cleanup_dnd_state();
    }

    fn on_dnd_drop(&mut self, keyboard_modifiers: RawInputModifiers) {
        // Move offer+cookie to pending list so they survive `cleanup_dnd_state`.
        // The UI thread will consume them via `finish_dnd`.
        if self.dnd_offer.is_some() && self.dnd_cookie.is_some() {
            if let (Some(offer), Some(cookie)) = (self.dnd_offer.take(), self.dnd_cookie.take()) {
                self.pending_drop_operations.push_back(PendingDrop { offer, cookie });
            }
        }

        if let Some(sink) = &self.dnd_sink {
            sink.on_drop(self.dnd_position, keyboard_modifiers);
        }

        // Reset the active session state; pending drops keep the device alive.
        self.dnd_sink = None;
        self.dnd_serial = 0;
        self.dnd_position = Point::default();
    }

    /// Called from the UI thread (via `post_oob`) after processing a DragEnter or DragOver event.
    /// Updates the offer's accepted action and MIME type based on the UI's response.
    pub(crate) fn update_dnd_feedback(&mut self, effects: DragDropEffects) {
        let Some(dnd_offer) = &self.dnd_offer else {
            return;
        };

        let (actions, preferred) = effects_to_actions_with_preferred(effects);
        dnd_offer.set_actions(actions, preferred);

        if effects == DragDropEffects::NONE {
            dnd_offer.accept(self.dnd_serial, None);
        } else {
            for mime in dnd_offer.mime_types() {
                dnd_offer.accept(self.dnd_serial, Some(mime));
            }
        }
    }

    /// Called from the UI thread (via `post_oob`) after a successful drop.
    /// Signals to the source that the transfer is complete.
    pub(crate) fn finish_dnd(&mut self) {
        if let Some(pending) = self.pending_drop_operations.pop_front() {
            pending.offer.finish();
            pending.cookie.invalidate();
            pending.offer.dispose();
        }
    }

    fn cleanup_dnd_state(&mut self) {
        if let Some(cookie) = self.dnd_cookie.take() {
            cookie.invalidate();
        }
        if let Some(offer) = self.dnd_offer.take() {
            offer.dispose();
        }
        self.dnd_sink = None;
        self.dnd_serial = 0;
        self.dnd_position = Point::default();
    }

    /// Starts a drag-and-drop operation from the given origin surface using the
    /// trigger event's serial. Called from the Wayland thread.
    ///
    /// `origin_surface` is the surface that has the implicit pointer grab
    /// (`WaylandInputDispatcher::find_origin_surface`).
    pub(crate) fn start_drag(
        &self,
        source: &WaylandDataSource,
        cookie: &WaylandInputEventCookie,
        allowed_actions: DndAction,
        connection_id: u64,
        origin_surface: Option<&WlSurface>,
    ) -> bool {
        let Some((_, serial)) = cookie.try_consume(connection_id) else {
            return false;
        };

        source.set_actions(allowed_actions);

        // Find the origin surface — the surface that has the implicit pointer grab
        let Some(origin_surface) = origin_surface else {
            return false;
        };

        self.device.start_drag(Some(source.wl_source()), origin_surface, None, serial);
        true
    }

    /// Releases the device. The source that was the selection is returned for the caller
    /// to dispose.
    pub fn dispose(mut self) -> Option<ObjectId> {
        if let Some(cookie) = self.selection_cookie.take() {
            cookie.invalidate();
        }
        if let Some(offer) = self.selection_offer.take() {
            offer.dispose();
        }
        self.cleanup_dnd_state();
        for pending in self.pending_drop_operations.drain(..) {
            pending.cookie.invalidate();
            pending.offer.dispose();
        }
        if let Some(offer) = self.pending_offer.take() {
            offer.dispose();
        }
        let active_source = self.active_source.take();
        self.device.release();
        active_source
    }
}

/// Sets the clipboard selection of the first data device to the given source, which the
/// globals keep from then on (`WaylandDataDevice.SetSelection`). The source that was the
/// selection before is disposed. Without a data device the source is disposed.
pub fn set_selection(globals: &mut WaylandGlobals, mut source: Option<WaylandDataSource>) {
    let WaylandGlobals { input_dispatcher, data_sources, .. } = globals;
    let Some(device) = input_dispatcher.get_data_device_mut() else {
        if let Some(source) = &mut source {
            source.dispose();
        }
        return;
    };
    let previous = device.replace_selection(source.as_ref());
    if let Some(mut previous) = previous.and_then(|previous| data_sources.remove(&previous)) {
        previous.dispose();
    }
    if let Some(source) = source {
        data_sources.insert(source.id(), source);
    }
}

pub(crate) fn actions_to_effects(actions: DndAction) -> DragDropEffects {
    let mut effects = DragDropEffects::NONE;
    if actions.contains(DndAction::Copy) {
        effects |= DragDropEffects::COPY;
    }
    if actions.contains(DndAction::Move) {
        effects |= DragDropEffects::MOVE;
    }
    effects
}

pub(crate) fn effects_to_actions(effects: DragDropEffects) -> DndAction {
    let mut actions = DndAction::None;
    if effects.contains(DragDropEffects::COPY) {
        actions |= DndAction::Copy;
    }
    if effects.contains(DragDropEffects::MOVE) {
        actions |= DndAction::Move;
    }
    actions
}

fn effects_to_actions_with_preferred(effects: DragDropEffects) -> (DndAction, DndAction) {
    let actions = effects_to_actions(effects);

    // Prefer copy if both are available
    let preferred = if actions.contains(DndAction::Copy) { DndAction::Copy } else { actions };

    (actions, preferred)
}

impl Dispatch<WlDataDevice, u32> for WaylandWorkerState {
    fn event(
        state: &mut Self,
        _proxy: &WlDataDevice,
        event: wl_data_device::Event,
        data: &u32,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        with_seat(state, *data, |seat, cx| {
            let keyboard_modifiers = seat.keyboard_modifiers;
            let Some(device) = &mut seat.data_device else {
                return;
            };
            match event {
                wl_data_device::Event::DataOffer { id } => device.on_data_offer(id),
                wl_data_device::Event::Enter { serial, surface, x, y, .. } => {
                    device.on_dnd_enter(serial, Some(&surface), x, y, keyboard_modifiers, cx);
                }
                wl_data_device::Event::Leave => device.on_dnd_leave(),
                wl_data_device::Event::Motion { x, y, .. } => device.on_dnd_motion(x, y, keyboard_modifiers),
                wl_data_device::Event::Drop => device.on_dnd_drop(keyboard_modifiers),
                wl_data_device::Event::Selection { id } => device.on_selection(id),
                _ => {}
            }
        });
    }

    event_created_child!(WaylandWorkerState, WlDataDevice, [
        wl_data_device::EVT_DATA_OFFER_OPCODE => (WlDataOffer, ()),
    ]);
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_actions_of_the_protocol_are_the_effects_of_the_framework() {
        assert_eq!(actions_to_effects(DndAction::None), DragDropEffects::NONE);
        assert_eq!(actions_to_effects(DndAction::Copy | DndAction::Move), DragDropEffects::COPY | DragDropEffects::MOVE);
        // The framework has no effect for "ask".
        assert_eq!(actions_to_effects(DndAction::Ask), DragDropEffects::NONE);
        assert_eq!(effects_to_actions(DragDropEffects::COPY), DndAction::Copy);
        // The protocol has no action for a link.
        assert_eq!(effects_to_actions(DragDropEffects::LINK), DndAction::None);
        assert_eq!(effects_to_actions(DragDropEffects::MOVE | DragDropEffects::LINK), DndAction::Move);
    }

    #[test]
    fn copy_is_preferred_when_both_are_accepted() {
        assert_eq!(
            effects_to_actions_with_preferred(DragDropEffects::COPY | DragDropEffects::MOVE),
            (DndAction::Copy | DndAction::Move, DndAction::Copy)
        );
        assert_eq!(effects_to_actions_with_preferred(DragDropEffects::MOVE), (DndAction::Move, DndAction::Move));
        assert_eq!(effects_to_actions_with_preferred(DragDropEffects::NONE), (DndAction::None, DndAction::None));
    }
}
