//! An object of the worker that outlives a connection (the port of
//! `IPersistentObject.cs`).

use crate::server::transient::wayland_globals::WaylandGlobals;
use crate::server::wayland_worker::WaylandWorkerState;
use wayland_client::QueueHandle;

/// What an object is connected with: the reference passes the connection and
/// the globals; the objects of the port create their protocol objects with
/// the handle of the queue and remember the connection by its number.
pub struct ConnectionContext<'a> {
    /// The number of the connection (`WaylandConnection::id`).
    pub connection_id: u64,
    pub queue_handle: &'a QueueHandle<WaylandWorkerState>,
    pub globals: &'a WaylandGlobals,
}

/// An object that is told when a connection with its globals exists, to
/// create what it has on the compositor, and when the connection is gone.
pub trait IPersistentWaylandObject {
    fn on_connected(&mut self, cx: &ConnectionContext<'_>);
    fn on_disconnected(&mut self);
}
