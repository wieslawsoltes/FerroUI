//! What creating a surface of the worker gives the UI thread (the port of
//! `WaylandSurfaceCreateResult.cs`).

use crate::server::persistent::xdg_configure_batch::XdgConfigureBatch;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use std::sync::mpsc::Receiver;
use std::sync::Arc;

/// Result of creating a worker-side wayland surface from the UI thread.
/// Bundles the UI→worker proxy together with whatever extra UI-thread
/// accessors the caller needs to drive the surface.
///
/// `T` is the concrete proxy type (e.g. `WXdgTopLevelProxy`).
///
/// The reference has `GetRenderSurfaces` as a callback, because the render
/// surfaces are objects of the worker; here they are handles that threads
/// share, so the result has them as they are. `BasicInitCompleted` is the
/// receiving end of a channel the worker sends the first configure on.
pub struct WaylandSurfaceCreateResult<T> {
    pub proxy: T,
    pub render_surfaces: Vec<Arc<dyn IPlatformRenderSurface>>,
    /// The first configure of a top-level; a popup has none to wait for.
    pub basic_init_completed: Option<Receiver<Arc<XdgConfigureBatch>>>,
}
