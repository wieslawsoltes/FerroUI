//! The commands of a surface (the port of `IWSurface.cs`) and the proxy the
//! UI thread holds (`WSurfaceProxy`, which the reference generates): every
//! call is posted to the worker, with the next composition batch unless a
//! priority says otherwise, and finds the surface by its number there. A
//! surface that is gone ignores the call.
//!
//! `set_text_input_options` converts the options to the content type of the
//! protocol on the UI thread: the options hold a reference counted list (the
//! locale hints), which cannot cross to the worker.

use super::i_wayland_cursor::WaylandCursorProxy;
use super::w_surface::{shell_surface_of_mut, WSurfaceId};
use crate::server::transient::i_wayland_text_input_v3_events::WaylandTextInputV3EventsProxy;
use crate::server::wayland_dispatch_priority::WaylandDispatchPriority;
use crate::text_input_options_converter;
use ferroui_base::input::text_input::TextInputOptions;
use ferroui_base::input::InputScopeNameValue;
use ferroui_base::Rect;
use crate::server::wayland_worker::{WaylandWorkerState, WaylandWorkerThread, WorkerMarshaller};

/// Worker-side surface commands posted from UI thread.
pub trait IWSurface {
    fn disconnect(&self);

    /// Sets the pointer cursor shown while the pointer is over this surface. `None` selects the
    /// default arrow. The cursor is the UI-side proxy, which names the worker-side cursor.
    fn set_cursor(&self, cursor: Option<&WaylandCursorProxy>);

    /// Registers the UI-thread sink to receive worker→UI text-input v3 events
    /// (preedit/commit/delete-surrounding). Idempotent. Does nothing on
    /// compositors that lack zwp_text_input_manager_v3.
    fn register_text_input_sink(&self, sink: WaylandTextInputV3EventsProxy);

    /// Toggles the text-input client. Sets the compose-key gate
    /// (XKB-compose runs only when a client is attached) and, when v3 is
    /// available, drives enable/disable on all entered seats. The
    /// `session_token` stamps every subsequent worker→UI
    /// IME callback so the UI broker can drop callbacks emitted for a
    /// previous client.
    fn set_text_input_active(&self, has_client: bool, supports_preedit: bool, supports_surrounding_text: bool, session_token: i32);

    /// Aborts any in-flight IME composition (disable+commit on entered seats).
    fn abort_text_input_composition(&self);

    /// Updates the IME cursor rectangle (logical, surface-local coordinates).
    fn set_text_input_cursor_rect(&self, rect: Rect);

    /// Updates the IME content-type (hint+purpose).
    fn set_text_input_options(&self, options: &TextInputOptions);

    /// Updates surrounding text + selection. `cursor_char` / `anchor_char` are UTF-16
    /// code-unit offsets into the supplied string (the indexing of the framework).
    fn set_text_input_surrounding_text(&self, text: &str, cursor_char: i32, anchor_char: i32);

    /// Resets the IME state (clears any pending preedit/commit).
    fn reset_text_input(&self);

    /// Toggles whether this surface accepts pointer/touch input. When `false`
    /// the surface gets an empty wl_surface input region, so the compositor routes
    /// input to whatever is behind it. Sticky across reconnects.
    fn set_hit_test_visible(&self, value: bool);
}

/// A surface of the worker as the UI thread holds it.
#[derive(Clone)]
pub struct WSurfaceProxy {
    id: WSurfaceId,
    marshaller: WorkerMarshaller,
}

impl WSurfaceProxy {
    pub fn new(id: WSurfaceId, marshaller: WorkerMarshaller) -> Self {
        Self { id, marshaller }
    }

    /// The number of the worker's surface (`ProxyTarget` of the reference, as a value).
    pub fn id(&self) -> WSurfaceId {
        self.id
    }

    /// Posts a call with the default priority of the proxy (with the next commit).
    pub(crate) fn post(&self, call: impl FnOnce(&mut WaylandWorkerThread, WSurfaceId) + Send + 'static) {
        self.post_with_priority(call, WaylandDispatchPriority::Normal);
    }

    pub(crate) fn post_with_priority(
        &self,
        call: impl FnOnce(&mut WaylandWorkerThread, WSurfaceId) + Send + 'static,
        priority: WaylandDispatchPriority,
    ) {
        let id = self.id;
        (self.marshaller)(Box::new(move |worker| call(worker, id)), priority);
    }

    /// `set_text_input_options` with the input scope of a client of the exact-edit connection
    /// (not in the reference; `docs/porting/winui-rich-text.md`, decisions D5 and D8). Without a
    /// scope it is `set_text_input_options`.
    pub(crate) fn set_text_input_options_with_input_scope(&self, options: &TextInputOptions, scope: Option<InputScopeNameValue>) {
        let options = text_input_options_converter::convert_with_input_scope(options, scope);
        self.post(move |worker, id| worker.state.set_text_input_options(id, options));
    }
}

impl IWSurface for WSurfaceProxy {
    fn disconnect(&self) {
        self.post(|worker, id| worker.unregister_surface(id));
    }

    fn set_cursor(&self, cursor: Option<&WaylandCursorProxy>) {
        let cursor = cursor.map(WaylandCursorProxy::id);
        self.post(move |worker, id| {
            if let Some(shell) = worker.state.shell_mut(id) {
                shell.surface_mut().set_cursor(cursor);
            }
            worker.state.notify_cursor_changed(id);
        });
    }

    fn register_text_input_sink(&self, sink: WaylandTextInputV3EventsProxy) {
        self.post(move |worker, id| worker.state.register_text_input_sink(id, sink));
    }

    fn set_text_input_active(&self, has_client: bool, supports_preedit: bool, supports_surrounding_text: bool, session_token: i32) {
        self.post(move |worker, id| {
            worker.state.set_text_input_active(id, has_client, supports_preedit, supports_surrounding_text, session_token);
        });
    }

    fn abort_text_input_composition(&self) {
        self.post(|worker, id| worker.state.abort_text_input_composition(id));
    }

    fn set_text_input_cursor_rect(&self, rect: Rect) {
        self.post(move |worker, id| worker.state.set_text_input_cursor_rect(id, rect));
    }

    fn set_text_input_options(&self, options: &TextInputOptions) {
        self.set_text_input_options_with_input_scope(options, None);
    }

    fn set_text_input_surrounding_text(&self, text: &str, cursor_char: i32, anchor_char: i32) {
        let text = text.to_string();
        self.post(move |worker, id| worker.state.set_text_input_surrounding_text(id, text, cursor_char, anchor_char));
    }

    fn reset_text_input(&self) {
        self.post(|worker, id| worker.state.reset_text_input(id));
    }

    fn set_hit_test_visible(&self, value: bool) {
        self.post(move |worker, id| {
            let WaylandWorkerState { globals, top_levels, popups, .. } = &mut worker.state;
            if let Some(shell) = shell_surface_of_mut(top_levels, popups, id) {
                let can_commit_out_of_band = shell.can_commit_out_of_band();
                shell.surface_mut().set_hit_test_visible(value, globals.as_ref(), can_commit_out_of_band);
            }
        });
    }
}
