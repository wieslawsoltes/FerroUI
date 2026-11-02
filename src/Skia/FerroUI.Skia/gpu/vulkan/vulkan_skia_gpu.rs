use super::{VulkanSkiaExternalObjectsFeature, VulkanSkiaRenderTarget};
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::{
    ISkiaGpu, ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, ISkiaSurface, ScopedGrContext,
};
use crate::skia_options::SkiaOptions;
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IExternalObjectsRenderInterfaceContextFeature, IOptionalFeatureProvider, IPlatformGraphicsContext,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use ferroui_vulkan::{
    try_get_vulkan_khr_surface, try_get_vulkan_render_target_surface, IVulkanContextExternalObjectsFeature,
    IVulkanKhrSurfacePlatformSurfaceFactory, IVulkanPlatformGraphicsContext, VulkanError,
};
use skia_safe::gpu::vk::{BackendContext, GetProcOf, Version};
use skia_safe::gpu::{direct_contexts, ContextOptions};
use std::any::{Any, TypeId};
use std::ffi::{c_void, CStr};
use std::rc::Rc;
use std::sync::Arc;

/// A Skia GPU that renders with the device of a Vulkan graphics context
/// through Ganesh.
pub struct VulkanSkiaGpu {
    external_objects: Option<Rc<VulkanSkiaExternalObjectsFeature>>,
    vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
    gr_context: Rc<GaneshGrContext>,
}

impl VulkanSkiaGpu {
    /// Creates a Skia GPU for a Vulkan context.
    ///
    /// # Panics
    /// Panics when Skia cannot create a context for the device.
    pub fn new(
        vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
        max_resource_bytes: Option<i64>,
        use_stencil_buffers: Option<bool>,
    ) -> Rc<Self> {
        let device = vulkan.device();
        let gr_context = {
            let current = vulkan.ensure_current();
            let instance = vulkan.instance();

            let get_proc_address_wrapper = |name: &str, instance_handle: isize, device_handle: isize| -> isize {
                if device_handle != 0 {
                    let addr = instance.get_device_proc_address(device_handle, name);
                    if addr != 0 {
                        return addr;
                    }
                }

                if instance_handle != 0 {
                    let addr = instance.get_instance_proc_address(instance_handle, name);
                    if addr != 0 {
                        return addr;
                    }
                }

                instance.get_instance_proc_address(0, name)
            };
            let get_proc = |of: GetProcOf| -> *const c_void {
                let (name, instance_handle, device_handle) = match of {
                    GetProcOf::Instance(instance, name) => (name, instance as usize as isize, 0),
                    GetProcOf::Device(device, name) => (name, 0, device as usize as isize),
                };
                // SAFETY: Skia passes the name of an entry point as a
                // zero-terminated string that lives for the call.
                let name = unsafe { CStr::from_ptr(name) };
                match name.to_str() {
                    Ok(name) => get_proc_address_wrapper(name, instance_handle, device_handle) as *const c_void,
                    Err(_) => std::ptr::null(),
                }
            };

            // SAFETY: the handles are those of the device of the context,
            // which outlives the Skia context: the GPU holds the context and
            // releases Skia's first.
            let ctx = unsafe {
                BackendContext::new_builder(
                    device.instance().handle() as usize as _,
                    device.physical_device_handle() as usize as _,
                    device.handle() as usize as _,
                    (device.main_queue_handle() as usize as _, device.graphics_queue_family_index() as usize),
                    &get_proc,
                    // The version Skia validates the entry points for. Without one Skia takes the
                    // version of the loader, and a driver hands out the entry points of the
                    // version the instance was created for only: 1.1 is what the platform
                    // graphics ask for by default and the least Skia accepts.
                    Some(Version::new(1, 1, 0)),
                )
                .build()
            };

            let avoid_stencil_buffers = SkiaOptions::should_avoid_stencil_buffers(use_stencil_buffers);
            let mut options = ContextOptions::default();
            options.avoid_stencil_buffers = avoid_stencil_buffers;
            let gr_context = direct_contexts::make_vulkan(&ctx, &options).map(GaneshGrContext::new);
            current.dispose();
            let gr_context = gr_context.unwrap_or_else(|| {
                panic!("{}", VulkanError::new("Unable to create GrContext from IVulkanDevice"))
            });
            if let Some(max_resource_bytes) = max_resource_bytes {
                gr_context.set_resource_cache_limit(max_resource_bytes);
            }
            Rc::new(gr_context)
        };

        let features: &dyn IOptionalFeatureProvider = &*vulkan;
        let external_objects = features.try_get::<dyn IVulkanContextExternalObjectsFeature>().map(|external_objects| {
            Rc::new(VulkanSkiaExternalObjectsFeature::new(gr_context.clone(), vulkan.clone(), external_objects))
        });

        Rc::new(Self { external_objects, vulkan, gr_context })
    }

    /// The Vulkan context the GPU renders with.
    pub fn vulkan(&self) -> &Rc<dyn IVulkanPlatformGraphicsContext> {
        &self.vulkan
    }

    /// The Ganesh context of the GPU.
    pub fn gr_context(&self) -> Rc<GaneshGrContext> {
        self.gr_context.clone()
    }

    fn surface_factory(&self) -> Option<Arc<dyn IVulkanKhrSurfacePlatformSurfaceFactory>> {
        let features: &dyn IOptionalFeatureProvider = &*self.vulkan;
        features.try_get_shared::<dyn IVulkanKhrSurfacePlatformSurfaceFactory>()
    }
}

impl IOptionalFeatureProvider for VulkanSkiaGpu {
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>> {
        if feature_type == TypeId::of::<dyn IExternalObjectsRenderInterfaceContextFeature>() {
            let feature: Rc<dyn IExternalObjectsRenderInterfaceContextFeature> = self.external_objects.clone()?;
            return Some(Rc::new(feature));
        }
        None
    }
}

impl IPlatformGraphicsContext for VulkanSkiaGpu {
    fn is_lost(&self) -> bool {
        self.vulkan.is_lost()
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.vulkan.ensure_current()
    }

    fn dispose(&self) {
        if self.vulkan.is_lost() {
            self.gr_context.dispose(false);
        } else {
            // Releasing resources does vkQueueWaitIdle and destroys API objects,
            // both of which require external synchronization, i. e. the device lock
            let current = self.vulkan.ensure_current();
            self.gr_context.dispose(true);
            current.dispose();
        }

        self.vulkan.dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ISkiaGpu for VulkanSkiaGpu {
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        Some(self.vulkan.clone())
    }

    fn try_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn ISkiaGpuRenderTarget>> {
        let target = self.vulkan.create_render_target(surfaces);
        Some(Rc::new(VulkanSkiaRenderTarget::new(self.gr_context.clone(), self.vulkan.clone(), target)))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        let factory = self.surface_factory();
        for surface in surfaces {
            if try_get_vulkan_khr_surface(&**surface).is_some()
                || try_get_vulkan_render_target_surface(&**surface).is_some()
                || factory.as_ref().is_some_and(|factory| factory.can_render_to_surface(&*self.vulkan, surface))
            {
                return surface.is_ready();
            }
        }

        false
    }

    fn try_create_surface(
        &self,
        _size: PixelSize,
        _session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
        _surface_props: &skia_safe::SurfaceProps,
    ) -> Option<Rc<dyn ISkiaSurface>> {
        None
    }

    fn try_get_gr_context(&self) -> Option<ScopedGrContext> {
        let context: Rc<dyn ISkiaGrContext> = self.gr_context.clone();
        Some(ScopedGrContext::new(context, Some(self.ensure_current())))
    }
}
