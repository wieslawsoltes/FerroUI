//! A cursor of the worker as the UI thread holds it (the port of
//! `IWaylandCursor.cs`, with the proxy the reference generates).

use super::wayland_cursor::WaylandCursorId;
use crate::server::wayland_dispatch_priority::WaylandDispatchPriority;
use crate::server::wayland_worker::WorkerMarshaller;

/// UI→worker surface for a pointer cursor. The UI thread only ever holds the
/// proxy for this interface and never the worker-side cursor directly.
pub trait IWaylandCursor {
    /// Releases the worker-side resources of the cursor. A no-op for themed cursors;
    /// bitmap cursors destroy their surface and unregister from the worker.
    fn destroy(&self);
}

/// The proxy of a cursor: its number and the marshaller to the worker.
#[derive(Clone)]
pub struct WaylandCursorProxy {
    id: WaylandCursorId,
    marshaller: WorkerMarshaller,
}

impl WaylandCursorProxy {
    pub fn new(id: WaylandCursorId, marshaller: WorkerMarshaller) -> Self {
        Self { id, marshaller }
    }

    /// The number of the worker's cursor (`ProxyTarget` of the reference, as a value).
    pub fn id(&self) -> WaylandCursorId {
        self.id
    }
}

impl IWaylandCursor for WaylandCursorProxy {
    fn destroy(&self) {
        let id = self.id;
        (self.marshaller)(Box::new(move |worker| worker.destroy_cursor(id)), WaylandDispatchPriority::Normal);
    }
}
