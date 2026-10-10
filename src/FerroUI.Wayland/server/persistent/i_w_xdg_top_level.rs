//! The commands of a shell surface and of a top-level (the port of
//! `IWXdgTopLevel.cs`) and the proxies the UI thread holds
//! (`WXdgShellSurfaceProxy`, `WXdgTopLevelProxy`, which the reference
//! generates).
//!
//! `IWXdgPopup` with its proxy and `ExportToplevel` belong to stage 2 of
//! `docs/porting/wayland-platform.md`.

use super::i_w_surface::WSurfaceProxy;
use super::i_w_surface_event_sink::PlatformInputEventCookie;
use super::w_surface::WSurfaceId;
use crate::server::wayland_dispatch_priority::WaylandDispatchPriority;
use crate::server::wayland_worker::WorkerMarshaller;
use ferroui_base::{Size, Thickness};
use std::ops::Deref;
use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

/// Worker-side XDG shell surface commands posted from UI thread.
pub trait IWXdgShellSurface {
    /// Sets shadow extents (CSD shadows) so the worker can compute
    /// `xdg_surface.set_window_geometry` excluding shadow margins.
    fn set_shadow_extents(&self, extents: Thickness);
    fn set_pending_ack_serial(&self, serial: u32);
}

/// Worker-side XDG toplevel commands posted from UI thread.
pub trait IWXdgTopLevel: IWXdgShellSurface {
    fn set_maximized(&self);
    fn unset_maximized(&self);
    fn set_fullscreen(&self);
    fn unset_fullscreen(&self);
    fn set_minimized(&self);
    fn set_parent(&self, parent: Option<&WXdgTopLevelProxy>);
    fn move_(&self, platform_cookie: Option<PlatformInputEventCookie>, priority: WaylandDispatchPriority);
    fn resize(&self, platform_cookie: Option<PlatformInputEventCookie>, edge: ResizeEdge, priority: WaylandDispatchPriority);

    /// Sets the toplevel's minimum and maximum size constraints atomically.
    /// Either argument may be `None` to mean "no constraint on this side".
    /// Values are in surface-local logical pixels and exclude any shadow
    /// extents. Double-buffered state: cached and re-applied to the next commit.
    fn set_min_max_size(&self, min_size: Option<Size>, max_size: Option<Size>);

    /// Sets the toplevel's title (xdg_toplevel.set_title). The compositor uses it for
    /// the server-side titlebar, taskbar, and alt-tab. `None` is treated as empty.
    /// Cached on the worker so it is re-applied on reconnect.
    fn set_title(&self, title: Option<&str>);

    /// Destroys the decoration object and disables its creation completely.
    fn destroy_decoration(&self);
}

/// A shell surface of the worker as the UI thread holds it.
#[derive(Clone)]
pub struct WXdgShellSurfaceProxy {
    surface: WSurfaceProxy,
}

impl WXdgShellSurfaceProxy {
    pub fn new(id: WSurfaceId, marshaller: WorkerMarshaller) -> Self {
        Self { surface: WSurfaceProxy::new(id, marshaller) }
    }
}

impl Deref for WXdgShellSurfaceProxy {
    type Target = WSurfaceProxy;

    fn deref(&self) -> &WSurfaceProxy {
        &self.surface
    }
}

impl IWXdgShellSurface for WXdgShellSurfaceProxy {
    fn set_shadow_extents(&self, extents: Thickness) {
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.shell_mut().set_shadow_extents(extents);
            }
        });
    }

    fn set_pending_ack_serial(&self, serial: u32) {
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.shell_mut().set_pending_ack_serial(serial);
            }
        });
    }
}

/// A top-level of the worker as the UI thread holds it.
#[derive(Clone)]
pub struct WXdgTopLevelProxy {
    shell: WXdgShellSurfaceProxy,
}

impl WXdgTopLevelProxy {
    pub fn new(id: WSurfaceId, marshaller: WorkerMarshaller) -> Self {
        Self { shell: WXdgShellSurfaceProxy::new(id, marshaller) }
    }

    /// The proxy as the proxy of its shell surface (the base interface).
    pub fn as_shell_surface(&self) -> &WXdgShellSurfaceProxy {
        &self.shell
    }
}

impl Deref for WXdgTopLevelProxy {
    type Target = WXdgShellSurfaceProxy;

    fn deref(&self) -> &WXdgShellSurfaceProxy {
        &self.shell
    }
}

impl IWXdgShellSurface for WXdgTopLevelProxy {
    fn set_shadow_extents(&self, extents: Thickness) {
        self.shell.set_shadow_extents(extents);
    }

    fn set_pending_ack_serial(&self, serial: u32) {
        self.shell.set_pending_ack_serial(serial);
    }
}

impl IWXdgTopLevel for WXdgTopLevelProxy {
    fn set_maximized(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_maximized();
            }
        });
    }

    fn unset_maximized(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.unset_maximized();
            }
        });
    }

    fn set_fullscreen(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_fullscreen();
            }
        });
    }

    fn unset_fullscreen(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.unset_fullscreen();
            }
        });
    }

    fn set_minimized(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_minimized();
            }
        });
    }

    fn set_parent(&self, parent: Option<&WXdgTopLevelProxy>) {
        let parent = parent.map(|parent| parent.id());
        self.post(move |worker, id| {
            let parent = parent
                .and_then(|parent| worker.state.top_levels.get(&parent))
                .and_then(|parent| parent.xdg_top_level().cloned());
            if let Some(top_level) = worker.state.top_levels.get(&id) {
                top_level.set_parent(parent.as_ref());
            }
        });
    }

    fn move_(&self, platform_cookie: Option<PlatformInputEventCookie>, priority: WaylandDispatchPriority) {
        self.post_with_priority(
            move |worker, id| {
                if let Some(top_level) = worker.state.top_levels.get(&id) {
                    top_level.move_(platform_cookie.as_ref());
                }
            },
            priority,
        );
    }

    fn resize(&self, platform_cookie: Option<PlatformInputEventCookie>, edge: ResizeEdge, priority: WaylandDispatchPriority) {
        self.post_with_priority(
            move |worker, id| {
                if let Some(top_level) = worker.state.top_levels.get(&id) {
                    top_level.resize(platform_cookie.as_ref(), edge);
                }
            },
            priority,
        );
    }

    fn set_min_max_size(&self, min_size: Option<Size>, max_size: Option<Size>) {
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.set_min_max_size(min_size, max_size);
            }
        });
    }

    fn set_title(&self, title: Option<&str>) {
        let title = title.map(str::to_string);
        self.post(move |worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.set_title(title);
            }
        });
    }

    fn destroy_decoration(&self) {
        self.post(|worker, id| {
            if let Some(top_level) = worker.state.top_levels.get_mut(&id) {
                top_level.destroy_decoration();
            }
        });
    }
}
