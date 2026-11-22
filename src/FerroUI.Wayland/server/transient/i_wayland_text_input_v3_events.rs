//! Worker → UI callback interface for `zwp_text_input_v3` (the port of
//! `IWaylandTextInputV3Events.cs`), and the proxy the worker holds
//! (`WaylandTextInputV3EventsProxy`, which the reference generates).
//!
//! All indices are UTF-16 code unit positions (the indices of the text of the
//! framework), not UTF-8 bytes — the worker performs the conversion before
//! invoking the sink.
//!
//! The worker invokes the method on the worker thread; the proxy marshals
//! each call to the UI thread. A single call corresponds to one wayland
//! `done` batch — the four operations (clear-preedit, delete-surrounding,
//! commit, new-preedit) are applied atomically by the UI broker so no other
//! UI work can interleave between them.

use crate::server::wayland_marshallers::UiThreadRef;
use ferroui_base::threading::Dispatcher;
use std::rc::Rc;
use std::sync::Arc;

pub trait IWaylandTextInputV3Events {
    /// Apply one IME update batch. Operations are applied in protocol
    /// order: clear current preedit, delete surrounding, insert commit,
    /// install new preedit.
    ///
    /// - `session_token`: the session token last received via
    ///   `IWSurface::set_text_input_active`; the UI broker drops batches whose
    ///   token doesn't match its current client epoch.
    /// - `delete_before_chars`: UTF-16 units to delete before the selection's start.
    /// - `delete_after_chars`: UTF-16 units to delete after the selection's end.
    /// - `commit_text`: text to commit; `None`/empty = no commit.
    /// - `preedit_text`: new preedit text; `None`/empty = clear preedit.
    /// - `preedit_cursor_begin_char`: preedit cursor start (UTF-16 chars; -1 = hidden).
    /// - `preedit_cursor_end_char`: preedit cursor end (UTF-16 chars; -1 = hidden).
    #[allow(clippy::too_many_arguments)]
    fn on_ime_update(
        &self,
        session_token: i32,
        delete_before_chars: i32,
        delete_after_chars: i32,
        commit_text: Option<String>,
        preedit_text: Option<String>,
        preedit_cursor_begin_char: i32,
        preedit_cursor_end_char: i32,
    );
}

/// The sink of the text input of a surface as the worker holds it.
#[derive(Clone)]
pub struct WaylandTextInputV3EventsProxy {
    target: UiThreadRef<dyn IWaylandTextInputV3Events>,
}

impl WaylandTextInputV3EventsProxy {
    /// A proxy of `target`, an object of the calling thread, whose dispatcher `dispatcher` is.
    pub fn new(target: Rc<dyn IWaylandTextInputV3Events>, dispatcher: Arc<Dispatcher>) -> Self {
        Self { target: UiThreadRef::new(target, dispatcher) }
    }
}

impl IWaylandTextInputV3Events for WaylandTextInputV3EventsProxy {
    fn on_ime_update(
        &self,
        session_token: i32,
        delete_before_chars: i32,
        delete_after_chars: i32,
        commit_text: Option<String>,
        preedit_text: Option<String>,
        preedit_cursor_begin_char: i32,
        preedit_cursor_end_char: i32,
    ) {
        self.target.post(move |sink| {
            sink.on_ime_update(
                session_token,
                delete_before_chars,
                delete_after_chars,
                commit_text,
                preedit_text,
                preedit_cursor_begin_char,
                preedit_cursor_end_char,
            )
        });
    }
}
