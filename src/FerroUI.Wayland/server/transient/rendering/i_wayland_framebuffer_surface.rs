//! What a render surface needs of the worker's object it draws for (the
//! port of `IWaylandFramebufferSurface.cs`).

use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::persistent::wayland_cursor::WaylandCursorId;
use crate::server::transient::wayland_globals::WaylandGlobals;
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use std::rc::{Rc, Weak};
use wayland_client::protocol::wl_surface::WlSurface;

/// Which object of the worker a render surface draws for. A render surface
/// is a handle that threads share, so it names its object; the reference
/// holds the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaylandRenderSurfaceTarget {
    Surface(WSurfaceId),
    Cursor(WaylandCursorId),
}

/// A render target that a surface disposes when its connection is lost,
/// before the `wl_surface` is destroyed (the reference registers an
/// `IDisposable`).
pub trait IWaylandSurfaceRenderTarget {
    /// Releases what the render target has on the connection. Called by the surface, on the
    /// worker thread, while the state of the worker is borrowed: the target does not reach
    /// back into it.
    fn dispose_from_surface(&self);
}

/// The surface of a framebuffer: a `wl_surface` with the state a frame needs.
///
/// `Globals` of the reference is not a member: the object of the port does not hold the
/// globals, its callers have them.
pub trait IWaylandFramebufferSurface {
    fn wl_surface(&self) -> Option<&WlSurface>;
    fn state(&self) -> PlatformRenderTargetState;
    fn register_render_target(&mut self, render_target: Weak<dyn IWaylandSurfaceRenderTarget>);
    fn unregister_render_target(&mut self, render_target: &Rc<dyn IWaylandSurfaceRenderTarget>);
    fn on_before_new_buffer_attached(&mut self, globals: &WaylandGlobals, scene_info: &RenderTargetSceneInfo);

    /// When `true`, the framebuffer render target does a wayland roundtrip right after committing
    /// each new buffer, flushing the buffer's fd to the compositor immediately.
    ///
    /// libwayland-client has a hard, undocumented limit on the number of file descriptors it can
    /// send in a single `wl_display_flush` (around 28), and exceeding it corrupts the connection.
    /// A surface that creates a burst of buffers outside the throttled render loop (e.g. a
    /// one-shot cursor image) must enable this. Surfaces that attach one buffer per frame should
    /// leave it off — a blocking roundtrip there would stall the render loop.
    fn enforce_buffer_creation_roundtrip(&self) -> bool;
}
