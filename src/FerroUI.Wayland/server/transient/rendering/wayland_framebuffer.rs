//! The software render surface of a surface of the worker (the port of
//! `WaylandFramebuffer.cs`): a frame is drawn into a file of memory, which
//! becomes a `wl_shm` buffer that is attached and committed when the frame
//! is done.

use super::i_wayland_framebuffer_surface::{
    IWaylandFramebufferSurface, IWaylandSurfaceRenderTarget, WaylandRenderSurfaceTarget,
};
use crate::server::interop::unsafe_native_methods::{memfd_of_length, MemoryMapping};
use crate::server::transient::wayland_globals::WaylandGlobals;
use crate::server::wayland_worker::{with_worker_thread, WaylandWorkerState, WaylandWorkerThread};
use ferroui_base::logging::{LogEventLevel, Logger};
use ferroui_base::platform::surfaces::{
    FramebufferLockProperties, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
    IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{AlphaFormat, ILockedFramebuffer, PixelFormat, PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_base::{PixelSize, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::rc::{Rc, Weak};
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::protocol::wl_shm;
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::{Connection, Dispatch, QueueHandle};

/// The name of the memory file of a frame.
const FRAMEBUFFER_NAME: &std::ffi::CStr = c"ferroui-wayland-framebuffer";

/// A surface of the worker as a target of software rendering.
///
/// The surface is a handle threads share: it names the worker's object. Its
/// render target is an object of the worker thread, where frames are drawn.
pub struct WaylandFramebuffer {
    target: WaylandRenderSurfaceTarget,
}

impl WaylandFramebuffer {
    pub fn new(target: WaylandRenderSurfaceTarget) -> Self {
        Self { target }
    }
}

/// The state of the object a render surface draws for; "not ready" on a thread other than the
/// worker and for an object that is gone.
pub(crate) fn target_state(target: WaylandRenderSurfaceTarget) -> PlatformRenderTargetState {
    with_worker_thread(|worker| worker.state.framebuffer_surface(target).map(|(_, surface)| surface.state()))
        .flatten()
        .unwrap_or_default()
}

impl IPlatformRenderSurface for WaylandFramebuffer {
    fn is_ready(&self) -> bool {
        target_state(self.target).is_ready
    }

    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for WaylandFramebuffer {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        let render_target =
            Rc::new_cyclic(|this| RenderTarget { this: this.clone(), target: self.target, disposed: Cell::new(false) });
        let registered: Weak<dyn IWaylandSurfaceRenderTarget> = render_target.this.clone();
        with_worker_thread(|worker| {
            if let Some((_, surface)) = worker.state.framebuffer_surface(self.target) {
                surface.register_render_target(registered);
            }
        });
        render_target
    }
}

struct RenderTarget {
    this: Weak<RenderTarget>,
    target: WaylandRenderSurfaceTarget,
    disposed: Cell<bool>,
}

impl IWaylandSurfaceRenderTarget for RenderTarget {
    fn dispose_from_surface(&self) {
        self.disposed.set(true);
    }
}

impl IPlatformRenderSurfaceRenderTarget for RenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        if self.disposed.get() {
            PlatformRenderTargetState::DISPOSED
        } else {
            target_state(self.target)
        }
    }
}

impl IFramebufferRenderTarget for RenderTarget {
    /// # Panics
    /// Panics when the memory of the frame cannot be allocated (the `OutOfMemoryException` of
    /// the reference).
    fn lock(&self, scene_info: &RenderTargetSceneInfo) -> (Rc<dyn ILockedFramebuffer>, FramebufferLockProperties) {
        // The reference throws `RenderTargetNotReadyException` for a target that is not
        // ready. The contract of the port has no error here and its renderer asks `state`
        // first; a frame that is locked anyway is drawn and not presented.
        let size = scene_info.size;
        let size = PixelSize::new(size.width.max(1), size.height.max(1));
        let buffer_len = size.width as usize * size.height as usize * 4;
        let stride = size.width * 4;

        let allocated = memfd_of_length(FRAMEBUFFER_NAME, buffer_len)
            .and_then(|fd| MemoryMapping::shared_read_write(fd.as_fd(), buffer_len).map(|mapping| (fd, mapping)));
        let (fd, mapping) = match allocated {
            Ok(allocated) => allocated,
            Err(error) => panic!("Unable to allocate framebuffer: {error}"),
        };

        let framebuffer = Rc::new(LockedFramebuffer {
            mapping: RefCell::new(Some(mapping)),
            fd,
            size,
            stride,
            dpi: Vector::new(96.0 * scene_info.scaling, 96.0 * scene_info.scaling),
            target: self.target,
            scene_info: scene_info.clone(),
            render_target: self.this.clone(),
        });
        (framebuffer, FramebufferLockProperties::default())
    }

    fn retains_frame_contents(&self) -> bool {
        false
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        let Some(this) = self.this.upgrade() else {
            return;
        };
        let this: Rc<dyn IWaylandSurfaceRenderTarget> = this;
        with_worker_thread(|worker| {
            if let Some((_, surface)) = worker.state.framebuffer_surface(self.target) {
                surface.unregister_render_target(&this);
            }
        });
    }
}

/// A frame while it is drawn: the mapped memory file. Disposing it presents the frame.
struct LockedFramebuffer {
    mapping: RefCell<Option<MemoryMapping>>,
    fd: OwnedFd,
    size: PixelSize,
    stride: i32,
    dpi: Vector,
    target: WaylandRenderSurfaceTarget,
    scene_info: RenderTargetSceneInfo,
    render_target: Weak<RenderTarget>,
}

impl ILockedFramebuffer for LockedFramebuffer {
    fn address(&self) -> *mut u8 {
        self.mapping.borrow().as_ref().map_or(std::ptr::null_mut(), MemoryMapping::address)
    }

    fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) {
        if let Some(data) = self.mapping.borrow_mut().as_mut().and_then(MemoryMapping::as_mut_slice) {
            access(data);
        }
    }

    fn size(&self) -> PixelSize {
        self.size
    }

    fn row_bytes(&self) -> i32 {
        self.stride
    }

    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn format(&self) -> PixelFormat {
        PixelFormat::BGRA8888
    }

    fn alpha_format(&self) -> AlphaFormat {
        AlphaFormat::Premul
    }

    fn dispose(&self) {
        // Unmapped first, as the reference: the compositor maps the file itself.
        if self.mapping.borrow_mut().take().is_none() {
            return;
        }
        let disposed = self.render_target.upgrade().is_none_or(|render_target| render_target.disposed.get());
        if disposed {
            return;
        }
        with_worker_thread(|worker| {
            present_frame(worker, self.target, self.fd.as_fd(), self.size, self.stride, &self.scene_info);
        });
    }
}

/// Makes a `wl_shm` buffer of a memory file with a frame and commits it to the surface of an
/// object of the worker, if the object is ready for a frame.
pub(crate) fn present_frame(
    worker: &mut WaylandWorkerThread,
    target: WaylandRenderSurfaceTarget,
    fd: BorrowedFd<'_>,
    size: PixelSize,
    stride: i32,
    scene_info: &RenderTargetSceneInfo,
) {
    let buffer = {
        let Some((globals, surface)) = worker.state.framebuffer_surface(target) else {
            return;
        };
        if !surface.state().is_ready {
            return;
        }
        create_frame_buffer(globals, fd, size, stride)
    };

    // Stage per-frame state (frame callback +
    // ack_configure + geometry + viewport/scale +
    // min/max) into the next commit BEFORE binding the
    // buffer, then commit explicitly after attach +
    // damage. Through the state of the worker: a surface that is mapped by this frame
    // attaches the popups that waited for it.
    worker.state.on_before_new_buffer_attached(target, scene_info);

    let enforce_roundtrip = {
        let Some((_, surface)) = worker.state.framebuffer_surface(target) else {
            buffer.destroy();
            return;
        };
        commit_frame_buffer(surface, buffer, size);
        surface.enforce_buffer_creation_roundtrip()
    };

    // Surfaces that allocate buffers outside the throttled render loop must
    // flush the buffer's fd immediately: libwayland-client caps the number of
    // fds per wl_display_flush (~28, undocumented), and a roundtrip flushes.
    if enforce_roundtrip {
        worker.roundtrip();
    }
}

/// The pool and the buffer over the memory file of a frame.
fn create_frame_buffer(globals: &WaylandGlobals, fd: BorrowedFd<'_>, size: PixelSize, stride: i32) -> WlBuffer {
    let queue_handle = globals.queue_handle();
    let pool = globals.wl_shm.create_pool(fd, stride * size.height, queue_handle, ());
    let buffer = pool.create_buffer(0, size.width, size.height, stride, wl_shm::Format::Argb8888, queue_handle, ());
    // The pool is destroyed at once: the buffer keeps the memory alive on the compositor.
    pool.destroy();
    buffer
}

/// The requests of a frame for an object that has no popups (a cursor): the pool and the
/// buffer over the memory file, the per-frame state of the surface, attach, damage, commit.
pub(crate) fn attach_frame(
    globals: &WaylandGlobals,
    surface: &mut dyn IWaylandFramebufferSurface,
    fd: BorrowedFd<'_>,
    size: PixelSize,
    stride: i32,
    scene_info: &RenderTargetSceneInfo,
) {
    let buffer = create_frame_buffer(globals, fd, size, stride);

    // Stage per-frame state (frame callback +
    // ack_configure + geometry + viewport/scale +
    // min/max) into the next commit BEFORE binding the
    // buffer, then commit explicitly after attach +
    // damage.
    surface.on_before_new_buffer_attached(globals, scene_info);
    commit_frame_buffer(surface, buffer, size);
}

/// Attach, damage, commit.
fn commit_frame_buffer(surface: &mut dyn IWaylandFramebufferSurface, buffer: WlBuffer, size: PixelSize) {
    let Some(wl_surface) = surface.wl_surface() else {
        buffer.destroy();
        return;
    };
    wl_surface.attach(Some(&buffer), 0, 0);
    // TODO: Support "damage" regions
    wl_surface.damage_buffer(0, 0, size.width, size.height);
    wl_surface.commit();
}

impl Dispatch<WlShmPool, ()> for WaylandWorkerState {
    fn event(_: &mut Self, _: &WlShmPool, _: <WlShmPool as wayland_client::Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

/// The buffer of a frame: destroyed when the compositor releases it.
impl Dispatch<WlBuffer, ()> for WaylandWorkerState {
    fn event(_: &mut Self, proxy: &WlBuffer, event: wl_buffer::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let wl_buffer::Event::Release = event {
            // TODO: Pool buffers
            proxy.destroy();
        }
    }
}

/// Logs a failure of a render surface.
pub(crate) fn log_render_error(message: &str, error: &dyn std::fmt::Display) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Wayland") {
        logger.log_with_values(None, message, &[error]);
    }
}
