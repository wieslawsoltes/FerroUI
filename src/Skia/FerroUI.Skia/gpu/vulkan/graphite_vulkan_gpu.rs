//! The Skia GPU of a Vulkan device on Graphite (#1365, part 3; the feature `graphite-vulkan`, off by
//! default): the window target drawn on the image of a Vulkan session, and the pipelined frame
//! layers of `gpu/graphite/pipelined_layer.rs` on images of `ferroui-vulkan`, whose band threads
//! draw on no shared target. Ganesh on Vulkan ([`VulkanSkiaGpu`](super::VulkanSkiaGpu)) stays the
//! GPU when Skia makes no Graphite context for the device, and when `FERROUI_SKIA_VULKAN=ganesh`
//! asks for it; never the CPU.

use super::vulkan_skia_gpu::is_ready_to_render;
use super::sk_image_layout;
use super::vulkan_skia_render_target::WrappedSurfaceKey;
use crate::gpu::graphite::{FrameTarget, GraphiteGrContext, LayerTexture, PipelinedLayerSurface};
use crate::gpu::{
    ISkiaGpu, ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, ISkiaSurface, ScopedGrContext,
    SkiaSurfaceOrigin,
};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::platform::{
    IOptionalFeatureProvider, IPlatformGraphicsContext, PlatformRenderTargetState, RenderTargetSceneInfo,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::PixelSize;
use ferroui_vulkan::ash::vk;
use ferroui_vulkan::{IVulkanPlatformGraphicsContext, IVulkanRenderSession, IVulkanRenderTarget};
use skia_safe::gpu::graphite;
use skia_safe::gpu::vk::mutable_texture_states;
use skia_safe::{ColorSpace, ColorType, Surface, SurfaceProps};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// Whether a Vulkan device is drawn with Graphite (in a build with the feature): unless
/// `FERROUI_SKIA_VULKAN` (`asked`) says `ganesh`, which keeps Ganesh, so that one build can be
/// measured with both.
pub(crate) fn selects_graphite(asked: Option<&str>) -> bool {
    !asked.is_some_and(|asked| asked.trim().eq_ignore_ascii_case("ganesh"))
}

/// The usage Graphite needs of the images it draws on besides the default ones: input attachment
/// (it reads the target in a render pass for some blends).
const GRAPHITE_IMAGE_USAGE: u32 = vk::ImageUsageFlags::INPUT_ATTACHMENT.as_raw();

/// A Skia GPU that renders with the device of a Vulkan graphics context through Graphite.
pub struct GraphiteVulkanSkiaGpu {
    vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
    gr_context: Rc<GraphiteGrContext>,
}

impl GraphiteVulkanSkiaGpu {
    /// A Graphite GPU of the device of `vulkan`; `None` when Skia makes no Graphite context for it
    /// (the caller takes Ganesh).
    pub fn try_new(
        vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
        max_resource_bytes: Option<i64>,
    ) -> Option<Rc<Self>> {
        let current = vulkan.ensure_current();
        let gr_context = super::graphite_vulkan::make_context(&*vulkan).and_then(GraphiteGrContext::on_vulkan);
        current.dispose();
        let gr_context = gr_context?;
        if let Some(max_resource_bytes) = max_resource_bytes {
            gr_context.set_resource_cache_limit(max_resource_bytes);
        }
        Some(Rc::new(Self { vulkan, gr_context: Rc::new(gr_context) }))
    }

    /// The Vulkan context the GPU renders with.
    pub fn vulkan(&self) -> &Rc<dyn IVulkanPlatformGraphicsContext> {
        &self.vulkan
    }

    /// The Graphite context of the GPU.
    pub fn gr_context(&self) -> Rc<GraphiteGrContext> {
        self.gr_context.clone()
    }

    /// A pipelined frame layer of the given size on an image of the device ([`LayerTexture::vulkan`]),
    /// whatever asks for it; `None` when pipelining is off, the layer is too low or the image cannot
    /// be made.
    pub(crate) fn try_create_pipelined_layer(
        &self,
        size: PixelSize,
        surface_props: &SurfaceProps,
        mode: crate::SkiaFramePipelining,
        bands: usize,
    ) -> Option<PipelinedLayerSurface> {
        let vulkan = self.vulkan.clone();
        let make_texture = |width, height, rgba| LayerTexture::vulkan(&*vulkan, width, height, rgba);
        PipelinedLayerSurface::try_with_texture(&self.gr_context, size, surface_props, mode, bands, make_texture)
    }
}

impl IOptionalFeatureProvider for GraphiteVulkanSkiaGpu {
    /// No feature: the import of images of other devices is Ganesh's
    /// (`VulkanSkiaExternalObjectsFeature`), and has no Graphite counterpart yet.
    fn try_get_feature(&self, _feature_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}

impl IPlatformGraphicsContext for GraphiteVulkanSkiaGpu {
    fn is_lost(&self) -> bool {
        self.vulkan.is_lost() || self.gr_context.is_lost()
    }

    fn ensure_current(&self) -> Rc<dyn IDisposable> {
        self.vulkan.ensure_current()
    }

    fn dispose(&self) {
        // The disposal waits for the GPU and destroys API objects, both of which need the device lock.
        let current = self.vulkan.ensure_current();
        self.gr_context.dispose();
        current.dispose();
        self.vulkan.dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl ISkiaGpu for GraphiteVulkanSkiaGpu {
    fn platform_graphics_context(&self) -> Option<Rc<dyn IPlatformGraphicsContext>> {
        Some(self.vulkan.clone())
    }

    fn try_create_render_target(
        &self,
        surfaces: &[Arc<dyn IPlatformRenderSurface>],
    ) -> Option<Rc<dyn ISkiaGpuRenderTarget>> {
        let target = self.vulkan.create_render_target(surfaces);
        // The first image is made with the usage, so no image is made twice.
        target.request_image_usage(GRAPHITE_IMAGE_USAGE);
        Some(Rc::new(GraphiteVulkanRenderTarget {
            gr_context: self.gr_context.clone(),
            vulkan: self.vulkan.clone(),
            target,
            wrapped: RefCell::new(None),
        }))
    }

    fn is_ready_to_create_render_target(&self, surfaces: &[Arc<dyn IPlatformRenderSurface>]) -> bool {
        is_ready_to_render(&*self.vulkan, surfaces)
    }

    /// The frame layer of the compositor (the layer a render target context creates carries a
    /// session) can pipeline its frames, unless
    /// [`SkiaOptions::frame_pipelining`](crate::SkiaOptions::frame_pipelining) is off; every other
    /// surface stays a surface of the context, as on Metal.
    fn try_create_surface(
        &self,
        size: PixelSize,
        session: Option<&Rc<dyn ISkiaGpuRenderSession>>,
        surface_props: &SurfaceProps,
    ) -> Option<Rc<dyn ISkiaSurface>> {
        session?;
        if self.gr_context.is_disposed() {
            return None;
        }
        let options = crate::SkiaOptions::current();
        let layer = self.try_create_pipelined_layer(
            size,
            surface_props,
            options.frame_pipelining,
            options.effective_frame_pipelining_bands(),
        )?;
        Some(Rc::new(layer))
    }

    fn try_get_gr_context(&self) -> Option<ScopedGrContext> {
        let context: Rc<dyn ISkiaGrContext> = self.gr_context.clone();
        Some(ScopedGrContext::new(context, Some(self.ensure_current())))
    }
}

/// The Graphite render target of a Vulkan render target: a frame is drawn into the image of a
/// session of the target.
pub struct GraphiteVulkanRenderTarget {
    gr_context: Rc<GraphiteGrContext>,
    vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
    target: Rc<dyn IVulkanRenderTarget>,
    /// The Graphite surface that wraps the image of the target, kept from frame to frame while the
    /// image is the same (a new one comes with a new swap chain, a resize). Graphite keeps its own
    /// record of the image's layout in it; every frame leaves the image in the layout of a colour
    /// attachment, where the copy to the swap chain expects it and returns it, so the record stays
    /// true.
    wrapped: RefCell<Option<(WrappedSurfaceKey, Surface)>>,
}

impl GraphiteVulkanRenderTarget {
    /// The surface of the frame: the kept one when the session's image is the one it wraps, or a new
    /// one wrapping the image in the layout the session reports.
    fn surface_for(&self, session: &dyn IVulkanRenderSession, surface_props: &SurfaceProps) -> Surface {
        let image = session.image_info();
        let key = WrappedSurfaceKey::new(image, session.is_y_flipped(), session.is_rgba(), *surface_props);
        let mut wrapped = self.wrapped.borrow_mut();
        if let Some((_, surface)) = wrapped.as_ref().filter(|(kept, _)| *kept == key) {
            return surface.clone();
        }
        // A new image (the first frame, a new swap chain) or other surface properties.
        *wrapped = None;
        if sk_image_layout(image.layout).is_none() {
            panic!("Unsupported image layout {}", image.layout);
        }
        let queue_family = self.vulkan.device().graphics_queue_family_index();
        let texture = super::graphite_backend_texture(&image, queue_family);
        let color_type = if session.is_rgba() { ColorType::RGBA8888 } else { ColorType::BGRA8888 };
        let surface = self.gr_context.with_recorder(|recorder| {
            graphite::surfaces::wrap_backend_texture(
                recorder,
                &texture,
                color_type,
                ColorSpace::new_srgb(),
                Some(surface_props),
            )
        });
        let surface = surface.unwrap_or_else(|| panic!("Unable to create a Graphite surface for the Vulkan image"));
        *wrapped = Some((key, surface.clone()));
        surface
    }
}

/// Disposes a Vulkan session unless the Skia session took it over.
struct SessionGuard {
    session: Rc<dyn IVulkanRenderSession>,
    success: Cell<bool>,
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if !self.success.get() {
            self.session.dispose();
        }
    }
}

impl ISkiaGpuRenderTarget for GraphiteVulkanRenderTarget {
    fn dispose(&self) {
        // Graphite's wrapper goes before the image it wraps; the target waits for the GPU before it
        // destroys its images, as with Ganesh.
        self.wrapped.borrow_mut().take();
        self.target.dispose();
    }

    fn begin_rendering_session(
        &self,
        _scene_info: &RenderTargetSceneInfo,
        surface_props: &SurfaceProps,
    ) -> Rc<dyn ISkiaGpuRenderSession> {
        let session = self.target.begin_draw();
        let guard = SessionGuard { session: session.clone(), success: Cell::new(false) };

        let size = session.size();
        let scaling = session.scaling();
        if size.width <= 0 || size.height <= 0 || scaling < 0.0 {
            panic!("Can't create drawing context for surface with {size} size and {scaling} scaling");
        }
        // Graphite draws with the first row at the top only.
        if !session.is_y_flipped() {
            panic!("Graphite on Vulkan draws on images whose first row is the top row only");
        }

        let surface = self.surface_for(&*session, surface_props);
        let queue_family = self.vulkan.device().graphics_queue_family_index();
        guard.success.set(true);

        Rc::new(GraphiteVulkanRenderSession {
            gr_context: self.gr_context.clone(),
            surface: RefCell::new(Some(surface)),
            vulkan_session: session,
            queue_family,
            disposed: Cell::new(false),
        })
    }

    fn state(&self) -> PlatformRenderTargetState {
        self.target.state()
    }
}

/// One frame rendered with Graphite to the image of a Vulkan session.
struct GraphiteVulkanRenderSession {
    gr_context: Rc<GraphiteGrContext>,
    surface: RefCell<Option<Surface>>,
    vulkan_session: Rc<dyn IVulkanRenderSession>,
    /// The queue family the image stays owned by when Graphite is done with it.
    queue_family: u32,
    disposed: Cell<bool>,
}

impl ISkiaGpuRenderSession for GraphiteVulkanRenderSession {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        if let Some(surface) = self.surface.borrow_mut().take() {
            if !self.gr_context.is_disposed() {
                // The one submission of the frame, without a wait: the recording leaves the image in the layout of a
                // colour attachment, where the copy to the swap chain (submitted after it, on the same queue)
                // expects it.
                let state = mutable_texture_states::new_vulkan(
                    skia_safe::gpu::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    self.queue_family,
                );
                self.gr_context.flush_to_target(FrameTarget { surface, state });
            }
        }
        self.vulkan_session.dispose();
    }

    fn gr_context(&self) -> Rc<dyn ISkiaGrContext> {
        self.gr_context.clone()
    }

    fn sk_surface(&self) -> Surface {
        self.surface.borrow().clone().expect("the Graphite Vulkan render session has been disposed")
    }

    fn scale_factor(&self) -> f64 {
        self.vulkan_session.scaling()
    }

    fn surface_origin(&self) -> SkiaSurfaceOrigin {
        SkiaSurfaceOrigin::TopLeft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graphite_is_the_vulkan_gpu_unless_ganesh_is_asked_for() {
        assert!(selects_graphite(None));
        assert!(selects_graphite(Some("graphite")));
        assert!(selects_graphite(Some("")));
        assert!(!selects_graphite(Some("ganesh")));
        assert!(!selects_graphite(Some(" Ganesh ")));
    }
}
