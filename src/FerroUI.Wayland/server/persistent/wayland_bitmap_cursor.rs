//! A cursor of the pixels of a bitmap (the port of
//! `WaylandBitmapCursor.cs`).

use super::i_persistent_object::{ConnectionContext, IPersistentWaylandObject};
use super::w_surface::WlSurfaceData;
use super::wayland_cursor::WaylandCursorImage;
use crate::server::interop::unsafe_native_methods::{memfd_of_length, MemoryMapping};
use crate::server::transient::rendering::i_wayland_framebuffer_surface::{
    IWaylandFramebufferSurface, IWaylandSurfaceRenderTarget,
};
use crate::server::transient::rendering::wayland_framebuffer::{attach_frame, log_render_error};
use crate::server::transient::wayland_globals::WaylandGlobals;
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::PixelSize;
use std::os::fd::AsFd;
use std::rc::{Rc, Weak};
use wayland_client::protocol::wl_surface::WlSurface;

/// The name of the memory file of the image of a cursor.
const CURSOR_NAME: &std::ffi::CStr = c"ferroui-wayland-cursor";

/// Worker-side custom bitmap cursor. Persistent: it keeps the raw pixels captured on the UI
/// thread and re-creates its `wl_surface` + buffer on every (re)connect, so a custom cursor
/// survives a compositor restart.
///
/// The reference draws the pixels through the framebuffer render target of the surface, and
/// the render target makes a round trip after the commit. Here the cursor attaches the frame
/// itself, with the function the render target uses, and the worker makes the round trip
/// after it connected the cursor (`enforce_buffer_creation_roundtrip`).
pub struct WaylandBitmapCursor {
    // tightly packed Bgra8888 premultiplied, stride = Width * 4
    pixels: Vec<u8>,
    size: PixelSize,
    hotspot_x: i32,
    hotspot_y: i32,
    connected: bool,
    wl_surface: Option<WlSurface>,
    active_render_targets: Vec<Weak<dyn IWaylandSurfaceRenderTarget>>,
}

impl WaylandBitmapCursor {
    pub fn new(pixels: Vec<u8>, size: PixelSize, hotspot_x: i32, hotspot_y: i32) -> Self {
        Self { pixels, size, hotspot_x, hotspot_y, connected: false, wl_surface: None, active_render_targets: Vec::new() }
    }

    pub fn resolve(&self) -> Option<WaylandCursorImage> {
        let surface = self.wl_surface.as_ref()?;
        Some(WaylandCursorImage { surface: surface.clone(), hotspot_x: self.hotspot_x, hotspot_y: self.hotspot_y })
    }

    fn render(&mut self, globals: &WaylandGlobals) {
        if self.size.width <= 0 || self.size.height <= 0 {
            return;
        }
        let stride = self.size.width * 4;
        let length = stride as usize * self.size.height as usize;
        if self.pixels.len() < length {
            return;
        }

        // The frame is Bgra8888 premultiplied with stride == Width * 4, matching the
        // packed pixel data we captured on the UI thread.
        let frame = memfd_of_length(CURSOR_NAME, length).and_then(|fd| {
            let mut mapping = MemoryMapping::shared_read_write(fd.as_fd(), length)?;
            if let Some(data) = mapping.as_mut_slice() {
                data.copy_from_slice(&self.pixels[..length]);
            }
            Ok(fd)
        });
        let fd = match frame {
            Ok(fd) => fd,
            Err(error) => {
                log_render_error("Unable to allocate the image of a cursor: {Error}", &error);
                return;
            }
        };

        let scene_info = RenderTargetSceneInfo::new(self.size, 1.0, CompositionTransparencyLevel::None);
        let size = self.size;
        attach_frame(globals, self, fd.as_fd(), size, stride, &scene_info);
    }
}

impl IPersistentWaylandObject for WaylandBitmapCursor {
    fn on_connected(&mut self, cx: &ConnectionContext<'_>) {
        self.connected = true;
        self.wl_surface = Some(cx.globals.wl_compositor.create_surface(cx.queue_handle, WlSurfaceData::Cursor));
        self.render(cx.globals);
    }

    fn on_disconnected(&mut self) {
        for render_target in std::mem::take(&mut self.active_render_targets) {
            if let Some(render_target) = render_target.upgrade() {
                render_target.dispose_from_surface();
            }
        }
        if let Some(wl_surface) = self.wl_surface.take() {
            wl_surface.destroy();
        }
        self.connected = false;
    }
}

impl IWaylandFramebufferSurface for WaylandBitmapCursor {
    fn wl_surface(&self) -> Option<&WlSurface> {
        self.wl_surface.as_ref()
    }

    fn state(&self) -> PlatformRenderTargetState {
        if self.connected && self.wl_surface.is_some() {
            PlatformRenderTargetState::READY
        } else {
            PlatformRenderTargetState::default()
        }
    }

    fn register_render_target(&mut self, render_target: Weak<dyn IWaylandSurfaceRenderTarget>) {
        self.active_render_targets.push(render_target);
    }

    fn unregister_render_target(&mut self, render_target: &Rc<dyn IWaylandSurfaceRenderTarget>) {
        self.active_render_targets.retain(|target| match target.upgrade() {
            Some(target) => !Rc::ptr_eq(&target, render_target),
            None => false,
        });
    }

    // No xdg role and no frame-callback throttling: nothing to stage before a buffer attach.
    fn on_before_new_buffer_attached(&mut self, _globals: &WaylandGlobals, _scene_info: &RenderTargetSceneInfo) {}

    // Cursor buffers are created outside the throttled render loop, so flush their fds eagerly.
    fn enforce_buffer_creation_roundtrip(&self) -> bool {
        true
    }
}
