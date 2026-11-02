//! The render surface of the dmabuf swapchain (the port of
//! `WaylandEglDmaBufSurface.cs`): a frame is drawn into an `EGLImage` over
//! a GBM buffer object, which is handed to the compositor as a dma-buf
//! (`zwp_linux_buffer_params_v1`) and attached and committed when the frame
//! is done. Four buffers are cycled; a buffer is busy from its commit until
//! the compositor releases it.

use super::i_wayland_framebuffer_surface::{IWaylandSurfaceRenderTarget, WaylandRenderSurfaceTarget};
use super::wayland_egl_display::WaylandEglDisplay;
use super::wayland_egl_dma_buf_platform_graphics::WaylandEglDmaBufPlatformGraphics;
use super::wayland_framebuffer::target_state;
use crate::server::interop::drm_gbm_unsafe_native_methods::{DrmGbmUnsafeNativeMethods, GbmBoFlags};
use crate::server::persistent::w_surface::WSurfaceId;
use crate::server::wayland_platform_graphics::WaylandPlatformGraphics;
use crate::server::wayland_worker::{with_worker_thread, WaylandWorkerState};
use ferroui_base::platform::surfaces::{IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetError, RenderTargetSceneInfo};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::ThreadBound;
use ferroui_base::{PixelSize, RenderTargetNotReadyError};
use ferroui_opengl::egl::egl_consts::*;
use ferroui_opengl::egl::{
    EglContext, EglGlPlatformImageSurfaceBase, EglImage, EglPlatformImageSurfaceRenderTarget,
    EglPlatformImageSurfaceRenderTargetBase,
};
use ferroui_opengl::surfaces::{IGlPlatformSurface, IGlPlatformSurfaceRenderTarget, IGlPlatformSurfaceRenderingSession};
use ferroui_opengl::{IGlContext, OpenGlError};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex, PoisonError};
use wayland_client::backend::protocol::{Argument, Message};
use wayland_client::backend::{Backend, ObjectData, ObjectId};
use wayland_client::protocol::wl_buffer::{self, WlBuffer};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_buffer_params_v1::{self, ZwpLinuxBufferParamsV1};
use wayland_protocols::wp::linux_dmabuf::zv1::client::zwp_linux_dmabuf_v1;

/// The dmabuf surface of a surface of the worker.
///
/// The surface is a handle threads share: it names the worker's surface and holds the
/// platform graphics, through which it answers as an OpenGL surface only while the backend of
/// the worker is the dmabuf one (the reference makes the render surfaces of the backend that
/// is bound; the port makes the handles once, on the UI thread). Its render target is an
/// object of the worker thread.
#[derive(Clone)]
pub struct WaylandEglDmaBufSurface {
    surface: WSurfaceId,
    graphics: Arc<WaylandPlatformGraphics>,
}

impl WaylandEglDmaBufSurface {
    pub fn new(surface: WSurfaceId, graphics: Arc<WaylandPlatformGraphics>) -> Self {
        Self { surface, graphics }
    }

    pub fn is_ready(&self) -> bool {
        target_state(WaylandRenderSurfaceTarget::Surface(self.surface)).is_ready
    }
}

impl IPlatformRenderSurface for WaylandEglDmaBufSurface {
    fn is_ready(&self) -> bool {
        WaylandEglDmaBufSurface::is_ready(self)
    }

    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        if kind == TypeId::of::<dyn IGlPlatformSurface>() && WaylandEglDmaBufPlatformGraphics::is_current(&self.graphics) {
            let this: Rc<dyn IGlPlatformSurface> = Rc::new(self.clone());
            return Some(Rc::new(this));
        }
        None
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EglGlPlatformImageSurfaceBase for WaylandEglDmaBufSurface {}

impl IGlPlatformSurface for WaylandEglDmaBufSurface {
    /// # Panics
    /// Panics when `context` is not an EGL context of the display of the dmabuf backend (the
    /// failing casts of the reference).
    fn create_gl_render_target(&self, context: &Rc<dyn IGlContext>) -> Rc<dyn IGlPlatformSurfaceRenderTarget> {
        let Some(egl_context) = context.as_any().downcast_ref::<EglContext>() else {
            panic!("Unable to cast the context to type 'EglContext'.");
        };
        let wayland_display = self
            .graphics
            .current_graphics()
            .and_then(|graphics| {
                graphics.as_any().downcast_ref::<WaylandEglDmaBufPlatformGraphics>().map(|graphics| graphics.display().clone())
            })
            .filter(|display| Rc::ptr_eq(display.display(), egl_context.display()));
        let Some(wayland_display) = wayland_display else {
            panic!("Unable to cast the display of the context to type 'WaylandEglDisplay'.");
        };
        let native = match DrmGbmUnsafeNativeMethods::get() {
            Ok(native) => native,
            Err(error) => panic!("{error}"),
        };
        let render_target = Rc::new_cyclic(|this| RenderTarget {
            this: this.clone(),
            base: EglPlatformImageSurfaceRenderTargetBase::new(egl_context.this()),
            surface: self.surface,
            wayland_display,
            native,
            slots: RefCell::new(None),
            pending_destroy: RefCell::new(None),
            current_size: Cell::new(PixelSize::default()),
            frame_counter: Cell::new(0),
            disposed: Cell::new(false),
        });
        let registered: Weak<dyn IWaylandSurfaceRenderTarget> = render_target.this.clone();
        with_worker_thread(|worker| {
            if let Some((_, surface)) = worker.state.framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.surface)) {
                surface.register_render_target(registered);
            }
        });
        render_target
    }
}

// TODO: Reduce to 2 or 3 if explicit sync is supported (once we implement it)
const SWAPCHAIN_SIZE: usize = 4;

struct RenderTarget {
    this: Weak<RenderTarget>,
    base: EglPlatformImageSurfaceRenderTargetBase,
    surface: WSurfaceId,
    wayland_display: Rc<WaylandEglDisplay>,
    native: &'static DrmGbmUnsafeNativeMethods,
    slots: RefCell<Option<Vec<Rc<BufferSlot>>>>,
    pending_destroy: RefCell<Option<Vec<Rc<BufferSlot>>>>,
    current_size: Cell<PixelSize>,
    frame_counter: Cell<u64>,
    disposed: Cell<bool>,
}

impl RenderTarget {
    fn target(&self) -> WaylandRenderSurfaceTarget {
        WaylandRenderSurfaceTarget::Surface(self.surface)
    }

    fn ensure_swapchain(&self, size: PixelSize) -> Result<(), OpenGlError> {
        if self.slots.borrow().is_some() && self.current_size.get() == size {
            return Ok(());
        }

        self.destroy_swapchain();

        *self.slots.borrow_mut() = Some(Vec::with_capacity(SWAPCHAIN_SIZE));
        self.current_size.set(size);

        let format = DrmGbmUnsafeNativeMethods::DRM_FORMAT_ARGB8888;

        // Collect modifiers for the chosen format
        let modifiers: Vec<u64> = self
            .wayland_display
            .supported_formats()
            .iter()
            .filter(|pair| pair.format == format)
            .map(|pair| pair.modifier)
            .collect();

        // The connection of the protocol object, for the round trip, and the queue of the
        // worker, which the buffers are put on.
        let connection = self
            .wayland_display
            .linux_dmabuf()
            .backend()
            .upgrade()
            .map(Connection::from_backend)
            .ok_or_else(|| OpenGlError::new("The Wayland connection of the swapchain is gone"))?;
        let queue_handle = with_worker_thread(|worker| worker.state.globals.as_ref().map(|globals| globals.queue_handle().clone()))
            .flatten()
            .ok_or_else(|| OpenGlError::new("The Wayland connection of the swapchain is gone"))?;

        for _ in 0..SWAPCHAIN_SIZE {
            let slot = Rc::new(BufferSlot::new(self.native, format));
            match self.create_slot(&slot, size, format, &modifiers, &connection, &queue_handle) {
                Ok(()) => {
                    if let Some(slots) = self.slots.borrow_mut().as_mut() {
                        slots.push(slot);
                    }
                }
                Err(error) => {
                    slot.dispose();
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn create_slot(
        &self,
        slot: &Rc<BufferSlot>,
        size: PixelSize,
        format: u32,
        modifiers: &[u64],
        connection: &Connection,
        queue_handle: &QueueHandle<WaylandWorkerState>,
    ) -> Result<(), OpenGlError> {
        let native = self.native;
        let gbm_device = self.wayland_display.gbm_device();

        // Allocate GBM buffer object
        let mut bo: *mut c_void = std::ptr::null_mut();
        if !modifiers.is_empty() {
            // SAFETY: the device of the display is alive; the modifiers are a slice of that
            // many values.
            bo = unsafe {
                native.gbm_bo_create_with_modifiers2(
                    gbm_device,
                    size.width as u32,
                    size.height as u32,
                    format,
                    modifiers.as_ptr(),
                    modifiers.len() as u32,
                    GbmBoFlags::GBM_BO_USE_RENDERING,
                )
            };
        }

        if bo.is_null() {
            // SAFETY: the device of the display is alive.
            bo = unsafe {
                native.gbm_bo_create(gbm_device, size.width as u32, size.height as u32, format, GbmBoFlags::GBM_BO_USE_RENDERING)
            };
        }

        if bo.is_null() {
            return Err(OpenGlError::new("Failed to create GBM buffer object"));
        }

        slot.gbm_bo.set(bo as usize);

        // Export DMA-BUF planes
        // SAFETY (this block and the ones below): `bo` is the live buffer object made above,
        // and the planes asked for are those it has.
        let plane_count = unsafe { native.gbm_bo_get_plane_count(bo) };
        slot.plane_count.set(plane_count);
        slot.modifier.set(unsafe { native.gbm_bo_get_modifier(bo) });
        *slot.dma_buf_fds.borrow_mut() = Some(Vec::new());
        *slot.strides.borrow_mut() = Some(Vec::new());
        *slot.offsets.borrow_mut() = Some(Vec::new());

        for p in 0..plane_count {
            let handle = unsafe { native.gbm_bo_get_handle_for_plane(bo, p) };
            let mut prime_fd: RawFd = -1;
            // SAFETY: the descriptor of the render node the buffer object was allocated on; a
            // handle is read as the 32 bits the call takes, as the reference reads it.
            if unsafe { native.drm_prime_handle_to_fd(self.wayland_display.drm_fd(), handle.u32, 0, &mut prime_fd) } != 0 {
                return Err(OpenGlError::new(format!("drmPrimeHandleToFD failed for plane {p}")));
            }
            // SAFETY: the call succeeded, so this is an open descriptor nobody else owns.
            let prime_fd = unsafe { OwnedFd::from_raw_fd(prime_fd) };
            if let Some(fds) = slot.dma_buf_fds.borrow_mut().as_mut() {
                fds.push(prime_fd);
            }
            if let Some(strides) = slot.strides.borrow_mut().as_mut() {
                strides.push(unsafe { native.gbm_bo_get_stride_for_plane(bo, p) });
            }
            if let Some(offsets) = slot.offsets.borrow_mut().as_mut() {
                offsets.push(unsafe { native.gbm_bo_get_offset(bo, p) });
            }
        }

        let planes = slot.planes();

        // Create EGLImage from DMA-BUF
        let mut attribs = vec![
            EGL_WIDTH,
            unsafe { native.gbm_bo_get_width(bo) } as i32,
            EGL_HEIGHT,
            unsafe { native.gbm_bo_get_height(bo) } as i32,
            EGL_LINUX_DRM_FOURCC_EXT,
            unsafe { native.gbm_bo_get_format(bo) } as i32,
        ];

        for (p, (fd, offset, stride)) in planes.iter().enumerate() {
            add_plane_attribs(&mut attribs, p, *fd, *offset, *stride, slot.modifier.get())?;
        }

        attribs.push(EGL_NONE);

        let egl_display = self.wayland_display.display();
        let image_handle = egl_display.egl_interface().create_image_khr(
            egl_display.handle(),
            EGL_NO_CONTEXT as isize,
            EGL_LINUX_DMA_BUF_EXT,
            0,
            &attribs,
        );

        if image_handle == 0 {
            return Err(OpenGlError::new("eglCreateImageKHR failed"));
        }

        *slot.egl_image.borrow_mut() = Some(EglImage::new(egl_display, image_handle));

        // Create wl_buffer via zwp_linux_buffer_params_v1, answered on a round trip of its
        // own (the temporary queue of the reference): the listener of the parameters takes
        // the buffer, which goes on the queue of the worker with its release listener.
        let release_listener = BufferReleaseListener { target: ThreadBound::new((self.this.clone(), Rc::downgrade(slot))) };
        let result = Arc::new(DmaBufCreateParamsListener {
            buffer_release_listener: queue_handle.make_data::<WlBuffer, _>(release_listener),
            buffer: Mutex::new(None),
        });
        let args: ZwpLinuxBufferParamsV1 = self
            .wayland_display
            .linux_dmabuf()
            .send_constructor(zwp_linux_dmabuf_v1::Request::CreateParams {}, result.clone())
            .map_err(|_| OpenGlError::new("The Wayland connection of the swapchain is gone"))?;
        if let Some(fds) = slot.dma_buf_fds.borrow().as_ref() {
            for (p, (fd, (offset, stride))) in fds.iter().zip(planes.iter().map(|(_, offset, stride)| (*offset, *stride))).enumerate() {
                let modifier = slot.modifier.get();
                args.add(fd.as_fd(), p as u32, offset, stride, (modifier >> 32) as u32, (modifier & 0xFFFF_FFFF) as u32);
            }
        }
        args.create(size.width, size.height, format, zwp_linux_buffer_params_v1::Flags::empty());
        let roundtrip = connection.roundtrip();
        args.destroy();
        roundtrip.map_err(|error| OpenGlError::new(error.to_string()))?;
        let buffer = result.buffer.lock().unwrap_or_else(PoisonError::into_inner).take();
        let Some(buffer) = buffer.and_then(|id| WlBuffer::from_id(connection, id).ok()) else {
            return Err(OpenGlError::new("Unable to create DMA-BUF on Wayland server side"));
        };
        *slot.wl_buffer.borrow_mut() = Some(buffer);
        Ok(())
    }

    fn destroy_swapchain(&self) {
        let Some(slots) = self.slots.borrow_mut().take() else {
            return;
        };
        for slot in slots {
            if slot.busy.get() {
                // Defer destruction of busy buffers until compositor releases them
                self.pending_destroy.borrow_mut().get_or_insert_with(Vec::new).push(slot);
            } else {
                slot.dispose();
            }
        }
        self.current_size.set(PixelSize::default());
    }

    fn on_buffer_released(&self, slot: &Rc<BufferSlot>) {
        slot.busy.set(false);
        let removed = {
            let mut pending_destroy = self.pending_destroy.borrow_mut();
            match pending_destroy.as_mut().and_then(|pending| pending.iter().position(|other| Rc::ptr_eq(other, slot))) {
                Some(index) => pending_destroy.as_mut().map(|pending| pending.remove(index)),
                None => None,
            }
        };
        if let Some(slot) = removed {
            slot.dispose();
        }
    }

    /// Releases the swapchain and the buffers that waited for the compositor.
    fn dispose_core(&self) {
        self.destroy_swapchain();
        if let Some(pending_destroy) = self.pending_destroy.borrow_mut().take() {
            for slot in pending_destroy {
                slot.dispose();
            }
        }
    }

    fn try_begin_draw_core_impl(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        if self.disposed.get() || !target_state(self.target()).is_ready {
            return Err(RenderTargetError::NotReady(RenderTargetNotReadyError::default()));
        }

        let size = scene_info.size;
        let size = PixelSize::new(size.width.max(1), size.height.max(1));

        if let Err(error) = self.ensure_swapchain(size) {
            panic!("{error}");
        }

        // Select least recently used free slot
        let free_slot = {
            let slots = self.slots.borrow();
            let mut free_slot: Option<Rc<BufferSlot>> = None;
            for slot in slots.iter().flatten() {
                if !slot.busy.get()
                    && free_slot.as_ref().is_none_or(|free| slot.last_used_frame.get() < free.last_used_frame.get())
                {
                    free_slot = Some(slot.clone());
                }
            }
            free_slot
        };

        let Some(captured_slot) = free_slot else {
            panic!("No free buffer slots available in swapchain");
        };

        self.frame_counter.set(self.frame_counter.get() + 1);
        captured_slot.last_used_frame.set(self.frame_counter.get());
        let image = captured_slot.egl_image.borrow().as_ref().map_or(0, EglImage::handle);
        let target = self.target();
        let finish_scene_info = scene_info.clone();
        let finish_slot = captured_slot.clone();
        let on_finish_draw: Rc<dyn Fn()> = Rc::new(move || {
            // Stage per-frame state (frame callback + ack_configure +
            // geometry + viewport/scale + min/max) into the next
            // commit BEFORE binding the buffer, then commit explicitly
            // after attach + damage. The base WSurface no longer
            // issues the commit itself.
            with_worker_thread(|worker| {
                worker.state.on_before_new_buffer_attached(target, &finish_scene_info);
                let Some((_, surface)) = worker.state.framebuffer_surface(target) else {
                    return;
                };
                let Some(wl_surface) = surface.wl_surface() else {
                    return;
                };
                let buffer = finish_slot.wl_buffer.borrow();
                let Some(buffer) = buffer.as_ref() else {
                    return;
                };
                wl_surface.attach(Some(buffer), 0, 0);
                wl_surface.damage_buffer(0, 0, size.width, size.height);
                finish_slot.busy.set(true);
                wl_surface.commit();
            });
        });
        // SAFETY: the image belongs to the display of the context and lives in the slot until
        // the slot is disposed, which a busy slot (the one being drawn) is not.
        match unsafe { self.base.begin_draw(image, size, scene_info.scaling, on_finish_draw) } {
            Ok(session) => Ok(session),
            Err(error) => panic!("{error}"),
        }
    }
}

impl IWaylandSurfaceRenderTarget for RenderTarget {
    fn dispose_from_surface(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.dispose_core();
    }
}

impl EglPlatformImageSurfaceRenderTarget for RenderTarget {
    fn base(&self) -> &EglPlatformImageSurfaceRenderTargetBase {
        &self.base
    }

    /// # Panics
    /// Panics when the surface is not ready for a frame or the swapchain cannot be made (the
    /// exceptions of the reference).
    fn begin_draw_core(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        match self.try_begin_draw_core_impl(scene_info) {
            Ok(session) => session,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_begin_draw_core(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        self.try_begin_draw_core_impl(scene_info)
    }

    fn state(&self) -> PlatformRenderTargetState {
        if self.is_corrupted() {
            PlatformRenderTargetState::CORRUPTED
        } else if self.disposed.get() {
            PlatformRenderTargetState::DISPOSED
        } else {
            target_state(self.target())
        }
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        if let Some(this) = self.this.upgrade() {
            let this: Rc<dyn IWaylandSurfaceRenderTarget> = this;
            with_worker_thread(|worker| {
                if let Some((_, surface)) = worker.state.framebuffer_surface(self.target()) {
                    surface.unregister_render_target(&this);
                }
            });
        }
        self.dispose_core();
        self.base.dispose();
    }
}

impl IPlatformRenderSurfaceRenderTarget for RenderTarget {
    fn state(&self) -> PlatformRenderTargetState {
        EglPlatformImageSurfaceRenderTarget::state(self)
    }
}

impl IGlPlatformSurfaceRenderTarget for RenderTarget {
    fn begin_draw(&self, scene_info: &RenderTargetSceneInfo) -> Rc<dyn IGlPlatformSurfaceRenderingSession> {
        EglPlatformImageSurfaceRenderTarget::begin_draw(self, scene_info)
    }

    fn try_begin_draw(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IGlPlatformSurfaceRenderingSession>, RenderTargetError> {
        EglPlatformImageSurfaceRenderTarget::try_begin_draw(self, scene_info)
    }

    fn dispose(&self) {
        EglPlatformImageSurfaceRenderTarget::dispose(self)
    }
}

/// The listener of the parameters of a buffer: takes the `wl_buffer` of `created`, which is
/// given the object data of the worker's queue with its release listener.
///
/// The reference gives a listener at creation; the bindings of the port take the object
/// data of a protocol object instead, which is what this is: the backend calls it when it
/// reads the event, so the round trip of the connection alone delivers it.
struct DmaBufCreateParamsListener {
    buffer_release_listener: Arc<dyn ObjectData>,
    buffer: Mutex<Option<ObjectId>>,
}

impl ObjectData for DmaBufCreateParamsListener {
    fn event(self: Arc<Self>, _backend: &Backend, msg: Message<ObjectId, OwnedFd>) -> Option<Arc<dyn ObjectData>> {
        if msg.opcode == zwp_linux_buffer_params_v1::EVT_CREATED_OPCODE {
            if let Some(Argument::NewId(buffer)) = msg.args.into_iter().next() {
                *self.buffer.lock().unwrap_or_else(PoisonError::into_inner) = Some(buffer);
                return Some(self.buffer_release_listener.clone());
            }
        }
        None
    }

    fn destroyed(&self, _object_id: ObjectId) {}
}

/// The attributes of a plane of a dma-buf for `eglCreateImageKHR`.
fn add_plane_attribs(
    attribs: &mut Vec<i32>,
    plane: usize,
    fd: RawFd,
    offset: u32,
    stride: u32,
    modifier: u64,
) -> Result<(), OpenGlError> {
    let (base_fd, base_offset, base_pitch, base_mod_lo, base_mod_hi) = match plane {
        0 => (
            EGL_DMA_BUF_PLANE0_FD_EXT,
            EGL_DMA_BUF_PLANE0_OFFSET_EXT,
            EGL_DMA_BUF_PLANE0_PITCH_EXT,
            EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
            EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
        ),
        1 => (
            EGL_DMA_BUF_PLANE1_FD_EXT,
            EGL_DMA_BUF_PLANE1_OFFSET_EXT,
            EGL_DMA_BUF_PLANE1_PITCH_EXT,
            EGL_DMA_BUF_PLANE1_MODIFIER_LO_EXT,
            EGL_DMA_BUF_PLANE1_MODIFIER_HI_EXT,
        ),
        2 => (
            EGL_DMA_BUF_PLANE2_FD_EXT,
            EGL_DMA_BUF_PLANE2_OFFSET_EXT,
            EGL_DMA_BUF_PLANE2_PITCH_EXT,
            EGL_DMA_BUF_PLANE2_MODIFIER_LO_EXT,
            EGL_DMA_BUF_PLANE2_MODIFIER_HI_EXT,
        ),
        3 => (
            EGL_DMA_BUF_PLANE3_FD_EXT,
            EGL_DMA_BUF_PLANE3_OFFSET_EXT,
            EGL_DMA_BUF_PLANE3_PITCH_EXT,
            EGL_DMA_BUF_PLANE3_MODIFIER_LO_EXT,
            EGL_DMA_BUF_PLANE3_MODIFIER_HI_EXT,
        ),
        _ => return Err(OpenGlError::new("Specified argument was out of the range of valid values. (Parameter 'plane')")),
    };

    attribs.push(base_fd);
    attribs.push(fd);
    attribs.push(base_offset);
    attribs.push(offset as i32);
    attribs.push(base_pitch);
    attribs.push(stride as i32);

    if modifier != DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID {
        attribs.push(base_mod_lo);
        attribs.push((modifier & 0xFFFF_FFFF) as i32);
        attribs.push(base_mod_hi);
        attribs.push((modifier >> 32) as i32);
    }
    Ok(())
}

/// A buffer of the swapchain: the GBM buffer object, the descriptors of its planes, the
/// `EGLImage` over them and the `wl_buffer` of the compositor.
struct BufferSlot {
    native: &'static DrmGbmUnsafeNativeMethods,
    /// The buffer object, as a number (zero: none).
    gbm_bo: Cell<usize>,
    dma_buf_fds: RefCell<Option<Vec<OwnedFd>>>,
    strides: RefCell<Option<Vec<u32>>>,
    offsets: RefCell<Option<Vec<u32>>>,
    // Kept as the reference keeps them; the planes are counted by their descriptors.
    #[allow(dead_code)]
    plane_count: Cell<i32>,
    modifier: Cell<u64>,
    #[allow(dead_code)]
    format: Cell<u32>,
    egl_image: RefCell<Option<EglImage>>,
    wl_buffer: RefCell<Option<WlBuffer>>,
    busy: Cell<bool>,
    last_used_frame: Cell<u64>,
}

impl BufferSlot {
    fn new(native: &'static DrmGbmUnsafeNativeMethods, format: u32) -> Self {
        Self {
            native,
            gbm_bo: Cell::new(0),
            dma_buf_fds: RefCell::new(None),
            strides: RefCell::new(None),
            offsets: RefCell::new(None),
            plane_count: Cell::new(0),
            modifier: Cell::new(0),
            format: Cell::new(format),
            egl_image: RefCell::new(None),
            wl_buffer: RefCell::new(None),
            busy: Cell::new(false),
            last_used_frame: Cell::new(0),
        }
    }

    /// The descriptor, the offset and the stride of every plane.
    fn planes(&self) -> Vec<(RawFd, u32, u32)> {
        let fds = self.dma_buf_fds.borrow();
        let strides = self.strides.borrow();
        let offsets = self.offsets.borrow();
        match (fds.as_ref(), strides.as_ref(), offsets.as_ref()) {
            (Some(fds), Some(strides), Some(offsets)) => fds
                .iter()
                .zip(offsets.iter())
                .zip(strides.iter())
                .map(|((fd, offset), stride)| (fd.as_raw_fd(), *offset, *stride))
                .collect(),
            _ => Vec::new(),
        }
    }

    fn dispose(&self) {
        if let Some(egl_image) = self.egl_image.borrow_mut().take() {
            egl_image.dispose();
        }

        if let Some(wl_buffer) = self.wl_buffer.borrow_mut().take() {
            wl_buffer.destroy();
        }

        // The descriptors are closed when they are dropped.
        self.dma_buf_fds.borrow_mut().take();

        let bo = self.gbm_bo.replace(0);
        if bo != 0 {
            // SAFETY: the buffer object of this slot, destroyed once.
            unsafe { self.native.gbm_bo_destroy(bo as *mut c_void) };
        }
    }
}

/// The release listener of the `wl_buffer` of a slot: its user data. The render target and
/// the slot are objects of the worker thread, which is where the events are dispatched.
struct BufferReleaseListener {
    target: ThreadBound<(Weak<RenderTarget>, Weak<BufferSlot>)>,
}

impl Dispatch<WlBuffer, BufferReleaseListener> for WaylandWorkerState {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        listener: &BufferReleaseListener,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            if !listener.target.is_on_thread() {
                return;
            }
            let (render_target, slot) = listener.target.get();
            if let (Some(render_target), Some(slot)) = (render_target.upgrade(), slot.upgrade()) {
                render_target.on_buffer_released(&slot);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn a_plane_has_its_descriptor_offset_and_pitch_and_an_explicit_modifier() {
        let mut attribs = Vec::new();
        add_plane_attribs(&mut attribs, 1, 9, 64, 4096, 0x0100_0000_0000_0002).unwrap();
        assert_eq!(
            attribs,
            vec![
                EGL_DMA_BUF_PLANE1_FD_EXT,
                9,
                EGL_DMA_BUF_PLANE1_OFFSET_EXT,
                64,
                EGL_DMA_BUF_PLANE1_PITCH_EXT,
                4096,
                EGL_DMA_BUF_PLANE1_MODIFIER_LO_EXT,
                2,
                EGL_DMA_BUF_PLANE1_MODIFIER_HI_EXT,
                0x0100_0000,
            ]
        );
    }

    #[test]
    fn the_implicit_modifier_is_not_named_and_a_fifth_plane_is_refused() {
        let mut attribs = Vec::new();
        add_plane_attribs(&mut attribs, 3, 4, 0, 256, DrmGbmUnsafeNativeMethods::DRM_FORMAT_MOD_INVALID).unwrap();
        assert_eq!(attribs, vec![EGL_DMA_BUF_PLANE3_FD_EXT, 4, EGL_DMA_BUF_PLANE3_OFFSET_EXT, 0, EGL_DMA_BUF_PLANE3_PITCH_EXT, 256]);
        assert!(add_plane_attribs(&mut attribs, 4, 4, 0, 256, 0).is_err());
    }
}
