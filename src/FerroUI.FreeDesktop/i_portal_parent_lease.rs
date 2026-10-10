//! The parent window of a portal dialog, as the portal wants it named
//! (the port of `IPortalParentLease.cs`).

use ferroui_base::input::LocalBoxFuture;

/// Backend-provided lease on a `parent_window` handle string suitable for passing
/// to `org.freedesktop.portal.FileChooser` et al.
///
/// The handle string is in the platform-prefixed format defined by xdg-desktop-portal:
/// `x11:HEX` for X11, `wayland:HANDLE` for wayland.
///
/// Disposal frees any platform resources backing the handle. For wayland this destroys
/// the `zxdg_exported_v2` object that provided the handle string; the imported handle
/// on the portal side becomes invalid afterwards, so the lease MUST be held until the
/// portal call completes.
pub trait IPortalParentLease {
    /// Prefixed parent-window handle string (e.g. `"x11:1A2B"`).
    fn handle(&self) -> String;

    /// Releases the lease (`IAsyncDisposable.DisposeAsync`).
    fn dispose_async(&self) -> LocalBoxFuture<()>;
}

pub struct TrivialPortalParentLease {
    handle: String,
}

impl TrivialPortalParentLease {
    pub fn new(handle: impl Into<String>) -> Self {
        Self { handle: handle.into() }
    }
}

impl IPortalParentLease for TrivialPortalParentLease {
    fn handle(&self) -> String {
        self.handle.clone()
    }

    fn dispose_async(&self) -> LocalBoxFuture<()> {
        Box::pin(std::future::ready(()))
    }
}
