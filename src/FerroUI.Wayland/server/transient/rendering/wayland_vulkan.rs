//! Native Vulkan WSI on the same connection and worker that own the wl_surface.
//! Queue presentation performs the buffer attach and commit; configure acknowledgement,
//! scale, viewport and the next frame callback are staged immediately before it.
use super::i_wayland_framebuffer_surface::{
    IWaylandSurfaceRenderTarget, WaylandRenderSurfaceTarget,
};
use super::wayland_framebuffer::target_state;
use crate::server::interop::wayland_connection::WaylandConnection;
use crate::server::persistent::w_surface::{WSurfaceId, WlSurfaceData};
use crate::server::transient::wayland_globals::WaylandGlobals;
use crate::server::wayland_platform_graphics::IWaylandGraphics;
use crate::server::wayland_worker::with_worker_thread;
use crate::FerroWaylandError;
use ferroui_base::platform::surfaces::{
    IPlatformRenderSurface, IPlatformRenderSurfaceRenderTarget,
};
use ferroui_base::platform::{
    IPlatformGraphics, IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetError,
    RenderTargetSceneInfo,
};
use ferroui_base::{PixelSize, RenderTargetNotReadyError};
use ferroui_vulkan::ash::{self, vk, vk::Handle};
use ferroui_vulkan::{
    IVulkanInstance, IVulkanKhrSurfacePlatformSurface, IVulkanPlatformGraphicsContext,
    IVulkanRenderSession, IVulkanRenderTarget, IVulkanRenderTargetPlatformSurface, VulkanError,
    VulkanImageInfo, VulkanOptions, VulkanPlatformGraphics, VulkanPlatformSpecificOptions,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::ffi::CString;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};
use wayland_client::{protocol::wl_surface::WlSurface, Connection, Proxy};

pub(crate) struct WaylandVulkanGraphics(Arc<VulkanPlatformGraphics>);

struct Probe(WlSurface);
impl Drop for Probe {
    fn drop(&mut self) {
        self.0.destroy();
    }
}

fn create_surface(
    instance: &dyn IVulkanInstance,
    display: &Connection,
    surface: &WlSurface,
) -> Result<u64, VulkanError> {
    let addr = instance.get_instance_proc_address(instance.handle(), "vkCreateWaylandSurfaceKHR");
    if addr == 0 {
        return Err(VulkanError::new(
            "VK_KHR_wayland_surface entry point unavailable",
        ));
    }
    let info = vk::WaylandSurfaceCreateInfoKHR::default()
        .display(display.backend().display_ptr().cast())
        .surface(surface.id().as_ptr().cast());
    let mut handle = vk::SurfaceKHR::null();
    // SAFETY: the extension was enabled on this instance. Both Wayland objects
    // belong to the same live libwayland connection. The worker releases the
    // swapchain and VkSurfaceKHR before destruction of the underlying wl_surface.
    let result = unsafe {
        let create: vk::PFN_vkCreateWaylandSurfaceKHR = std::mem::transmute(addr);
        create(
            vk::Instance::from_raw(instance.handle() as u64),
            &info,
            std::ptr::null(),
            &mut handle,
        )
    };
    if result != vk::Result::SUCCESS {
        return Err(VulkanError::new(format!(
            "vkCreateWaylandSurfaceKHR: {result:?}"
        )));
    }
    Ok(handle.as_raw())
}

impl WaylandVulkanGraphics {
    pub(crate) fn new(
        connection: &WaylandConnection,
        globals: &WaylandGlobals,
        options: &VulkanOptions,
    ) -> Result<Rc<Self>, FerroWaylandError> {
        // SAFETY: ash loads the system Vulkan loader; the callback retains it.
        let entry = Arc::new(
            unsafe { ash::Entry::load() }
                .map_err(|e| FerroWaylandError::with_inner("Vulkan loader", e))?,
        );
        let display = connection.display().clone();
        let probe = Probe(
            globals
                .wl_compositor
                .create_surface(globals.queue_handle(), WlSurfaceData::Cursor),
        );
        let options_for_wsi = VulkanPlatformSpecificOptions {
            required_instance_extensions: vec!["VK_KHR_wayland_surface".into()],
            get_proc_address_delegate: Some(Arc::new(move |instance, name| {
                let Ok(name) = CString::new(name) else {
                    return 0;
                };
                // SAFETY: the framework supplies instances obtained from this loader.
                let address = unsafe {
                    (entry.static_fn().get_instance_proc_addr)(
                        vk::Instance::from_raw(instance as u64),
                        name.as_ptr(),
                    )
                };
                address.map_or(0, |p| p as usize as isize)
            })),
            device_check_surface_factory: Some(Arc::new(move |instance| {
                create_surface(&**instance, &display, &probe.0)
            })),
            ..Default::default()
        };
        VulkanPlatformGraphics::try_create(options, options_for_wsi)
            .map(|gpu| Rc::new(Self(gpu)))
            .ok_or_else(|| {
                FerroWaylandError::new("No Vulkan device can present to this Wayland compositor")
            })
    }
}
impl IWaylandGraphics for WaylandVulkanGraphics {
    fn create_context(&self) -> Rc<dyn IPlatformGraphicsContext> {
        if self.0.uses_shared_context() {
            self.0.get_shared_context()
        } else {
            self.0.create_context()
        }
    }
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Copy)]
pub(crate) struct WaylandVulkanSurface(pub WSurfaceId);
impl IPlatformRenderSurface for WaylandVulkanSurface {
    fn is_ready(&self) -> bool {
        target_state(WaylandRenderSurfaceTarget::Surface(self.0)).is_ready
    }
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        (kind == TypeId::of::<dyn IVulkanRenderTargetPlatformSurface>()).then(|| {
            let surface: Rc<dyn IVulkanRenderTargetPlatformSurface> = Rc::new(*self);
            Rc::new(surface) as Rc<dyn Any>
        })
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IVulkanRenderTargetPlatformSurface for WaylandVulkanSurface {
    fn create_render_target(
        &self,
        context: &Rc<dyn IVulkanPlatformGraphicsContext>,
    ) -> Rc<dyn IVulkanRenderTarget> {
        let target = Rc::new_cyclic(|this| Target {
            this: this.clone(),
            id: self.0,
            context: context.clone(),
            bound: RefCell::default(),
            disposed: Cell::new(false),
        });
        let listener: Rc<dyn IWaylandSurfaceRenderTarget> = target.clone();
        with_worker_thread(|worker| {
            if let Some((_, surface)) = worker
                .state
                .framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.0))
            {
                surface.register_render_target(Rc::downgrade(&listener));
            }
        });
        target
    }
}

#[derive(Clone)]
struct NativeSurface {
    display: Connection,
    surface: WlSurface,
    extent: Arc<Mutex<(PixelSize, f64)>>,
}
impl IPlatformRenderSurface for NativeSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn try_get_surface_kind(&self, kind: TypeId) -> Option<Rc<dyn Any>> {
        (kind == TypeId::of::<dyn IVulkanKhrSurfacePlatformSurface>()).then(|| {
            let surface: Rc<dyn IVulkanKhrSurfacePlatformSurface> = Rc::new(self.clone());
            Rc::new(surface) as Rc<dyn Any>
        })
    }
}
impl IVulkanKhrSurfacePlatformSurface for NativeSurface {
    fn scaling(&self) -> f64 {
        self.extent.lock().unwrap().1
    }
    fn size(&self) -> PixelSize {
        self.extent.lock().unwrap().0
    }
    fn create_surface(&self, context: &dyn IVulkanPlatformGraphicsContext) -> u64 {
        create_surface(&*context.instance(), &self.display, &self.surface)
            .unwrap_or_else(|e| panic!("{e}"))
    }
    fn dispose(&self) {}
}

struct Target {
    this: Weak<Target>,
    id: WSurfaceId,
    context: Rc<dyn IVulkanPlatformGraphicsContext>,
    bound: RefCell<Option<(NativeSurface, Rc<dyn IVulkanRenderTarget>)>>,
    disposed: Cell<bool>,
}
impl IWaylandSurfaceRenderTarget for Target {
    fn dispose_from_surface(&self) {
        self.disposed.set(true);
        if let Some((_, target)) = self.bound.borrow_mut().take() {
            target.dispose();
        }
    }
}
impl IPlatformRenderSurfaceRenderTarget for Target {
    fn state(&self) -> PlatformRenderTargetState {
        if self.disposed.get() {
            PlatformRenderTargetState::DISPOSED
        } else {
            target_state(WaylandRenderSurfaceTarget::Surface(self.id))
        }
    }
}
impl IVulkanRenderTarget for Target {
    fn begin_draw(&self) -> Rc<dyn IVulkanRenderSession> {
        let scene = with_worker_thread(|worker| worker.state.shell(self.id)?.negotiated_scene())
            .flatten()
            .expect("Wayland Vulkan target has no acknowledged configure on this render thread");
        self.try_begin_draw(&scene)
            .unwrap_or_else(|error| panic!("Cannot begin a Wayland Vulkan frame: {error}"))
    }
    fn try_begin_draw(
        &self,
        scene: &RenderTargetSceneInfo,
    ) -> Result<Rc<dyn IVulkanRenderSession>, RenderTargetError> {
        let not_ready = || RenderTargetError::NotReady(RenderTargetNotReadyError::default());
        if !self.state().is_ready || scene.size.width < 1 || scene.size.height < 1 {
            return Err(not_ready());
        }
        let mut bound = self.bound.borrow_mut();
        if bound.is_none() {
            let surface = with_worker_thread(|w| {
                w.state
                    .framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.id))
                    .and_then(|(_, s)| s.wl_surface().cloned())
            })
            .flatten()
            .ok_or_else(not_ready)?;
            let display =
                Connection::from_backend(surface.backend().upgrade().ok_or_else(not_ready)?);
            let native = NativeSurface {
                display,
                surface,
                extent: Arc::new(Mutex::new((scene.size, scene.scaling))),
            };
            let target = self
                .context
                .create_render_target(&[Arc::new(native.clone())]);
            *bound = Some((native, target));
        }
        let (native, target) = bound.as_ref().unwrap();
        *native.extent.lock().unwrap() = (scene.size, scene.scaling);
        Ok(Rc::new(Frame {
            inner: target.begin_draw(),
            scene: scene.clone(),
            id: self.id,
            presented: Cell::new(false),
        }))
    }
    fn dispose(&self) {
        if self.disposed.get() {
            return;
        }
        self.dispose_from_surface();
        if let Some(this) = self.this.upgrade() {
            let listener: Rc<dyn IWaylandSurfaceRenderTarget> = this;
            with_worker_thread(|worker| {
                if let Some((_, surface)) = worker
                    .state
                    .framebuffer_surface(WaylandRenderSurfaceTarget::Surface(self.id))
                {
                    surface.unregister_render_target(&listener);
                }
            });
        }
    }
}
struct Frame {
    inner: Rc<dyn IVulkanRenderSession>,
    scene: RenderTargetSceneInfo,
    id: WSurfaceId,
    presented: Cell<bool>,
}
impl IVulkanRenderSession for Frame {
    fn size(&self) -> PixelSize {
        self.inner.size()
    }
    fn scaling(&self) -> f64 {
        self.inner.scaling()
    }
    fn is_y_flipped(&self) -> bool {
        self.inner.is_y_flipped()
    }
    fn image_info(&self) -> VulkanImageInfo {
        self.inner.image_info()
    }
    fn is_rgba(&self) -> bool {
        self.inner.is_rgba()
    }
    fn dispose(&self) {
        if self.presented.replace(true) {
            return;
        }
        with_worker_thread(|w| {
            w.state.on_before_new_buffer_attached(
                WaylandRenderSurfaceTarget::Surface(self.id),
                &self.scene,
            )
        });
        self.inner.dispose();
    }
}
