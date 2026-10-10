//! The commands of a surface (the port of `IWSurface.cs`) and the proxy the
//! UI thread holds (`WSurfaceProxy`, which the reference generates): every
//! call is posted to the worker, with the next composition batch unless a
//! priority says otherwise, and finds the surface by its number there. A
//! surface that is gone ignores the call.
//!
//! The members of text input (`RegisterTextInputSink`, `SetTextInputActive`,
//! `AbortTextInputComposition`, `SetTextInputCursorRect`,
//! `SetTextInputOptions`, `SetTextInputSurroundingText`, `ResetTextInput`)
//! belong to stage 2 of `docs/porting/wayland-platform.md`.

use super::i_wayland_cursor::WaylandCursorProxy;
use super::w_surface::WSurfaceId;
use crate::server::wayland_dispatch_priority::WaylandDispatchPriority;
use crate::server::wayland_worker::{WaylandWorkerThread, WorkerMarshaller};

/// Worker-side surface commands posted from UI thread.
pub trait IWSurface {
    fn disconnect(&self);

    /// Sets the pointer cursor shown while the pointer is over this surface. `None` selects the
    /// default arrow. The cursor is the UI-side proxy, which names the worker-side cursor.
    fn set_cursor(&self, cursor: Option<&WaylandCursorProxy>);

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
}

impl IWSurface for WSurfaceProxy {
    fn disconnect(&self) {
        self.post(|worker, id| worker.unregister_top_level(id));
    }

    fn set_cursor(&self, cursor: Option<&WaylandCursorProxy>) {
        let cursor = cursor.map(WaylandCursorProxy::id);
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.shell_mut().surface_mut().set_cursor(cursor);
            }
            worker.state.notify_cursor_changed(id);
        });
    }

    fn set_hit_test_visible(&self, value: bool) {
        self.post(move |worker, id| {
            let globals = worker.state.globals.as_ref();
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                let can_commit_out_of_band = top_level.shell().can_commit_out_of_band();
                top_level.shell_mut().surface_mut().set_hit_test_visible(value, globals, can_commit_out_of_band);
            }
        });
    }
}
