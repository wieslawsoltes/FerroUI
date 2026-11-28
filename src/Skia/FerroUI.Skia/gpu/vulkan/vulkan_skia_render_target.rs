use super::{sk_image_info, sk_image_layout};
use crate::gpu::ganesh::GaneshGrContext;
use crate::gpu::{ISkiaGpuRenderSession, ISkiaGpuRenderTarget, ISkiaGrContext, SkiaSurfaceOrigin};
use ferroui_base::platform::{PlatformRenderTargetState, RenderTargetSceneInfo};
use ferroui_vulkan::{IVulkanPlatformGraphicsContext, IVulkanRenderSession, IVulkanRenderTarget, VulkanImageInfo};
use skia_safe::gpu::vk::mutable_texture_states;
use skia_safe::gpu::{backend_render_targets, surfaces, BackendRenderTarget, FlushInfo, SurfaceOrigin};
use skia_safe::{ColorSpace, ColorType, Surface, SurfaceProps};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The Skia render target of a Vulkan render target: a frame is drawn into
/// the image of a session of the target.
pub struct VulkanSkiaRenderTarget {
    gr_context: Rc<GaneshGrContext>,
    vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
    target: Rc<dyn IVulkanRenderTarget>,
    /// The Skia surface that wraps the image of the target, kept from frame
    /// to frame while the image is the same: the Vulkan target makes a new
    /// image only with a new swap chain (a resize).
    wrapped: RefCell<Option<WrappedSurface>>,
}

/// A Skia surface wrapping the image of a Vulkan session, with the backend
/// render target it was made from (which shares Skia's record of the
/// image's layout) and what it was made for.
struct WrappedSurface {
    key: WrappedSurfaceKey,
    render_target: BackendRenderTarget,
    surface: Surface,
}

/// What a wrapped surface depends on: the image (all of its description but
/// its current layout, which is told to Skia every frame), the orientation,
/// the order of the channels and the surface properties.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WrappedSurfaceKey {
    image: VulkanImageInfo,
    y_flipped: bool,
    rgba: bool,
    props: SurfaceProps,
}

impl WrappedSurfaceKey {
    pub(crate) fn new(image: VulkanImageInfo, y_flipped: bool, rgba: bool, props: SurfaceProps) -> Self {
        Self { image: VulkanImageInfo { layout: 0, ..image }, y_flipped, rgba, props }
    }
}

impl VulkanSkiaRenderTarget {
    /// The render target of `target` for the GPU that renders with `vulkan`
    /// through `gr_context`.
    // The original holds its `VulkanSkiaGpu`; the two things it reads of the
    // GPU are held here instead.
    pub fn new(
        gr_context: Rc<GaneshGrContext>,
        vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
        target: Rc<dyn IVulkanRenderTarget>,
    ) -> Self {
        Self { gr_context, vulkan, target, wrapped: RefCell::new(None) }
    }

    /// The surface of the frame: the kept one when the session's image is
    /// the one it wraps, told the layout FerroUI left the image in, or a new
    /// one wrapping the image.
    fn surface_for(&self, session: &dyn IVulkanRenderSession, surface_props: &SurfaceProps) -> Surface {
        let session_image_info = session.image_info();
        let key = WrappedSurfaceKey::new(session_image_info, session.is_y_flipped(), session.is_rgba(), *surface_props);
        let mut wrapped = self.wrapped.borrow_mut();
        if let Some(kept) = wrapped.as_mut().filter(|kept| kept.key == key) {
            // FerroUI's commands (the copy to the swap chain) change the layout outside Skia; the backend render target
            // shares Skia's record of it, so this tells Skia without resetting the context.
            let layout = sk_image_layout(session_image_info.layout)
                .unwrap_or_else(|| panic!("Unsupported image layout {}", session_image_info.layout));
            backend_render_targets::set_vk_image_layout(&mut kept.render_target, layout);
            return kept.surface.clone();
        }
        // A new image (the first frame, a new swap chain) or other surface properties.
        *wrapped = None;
        let size = session.size();
        let image_info = sk_image_info(&session_image_info, self.vulkan.device().graphics_queue_family_index());
        let render_target = backend_render_targets::make_vk((size.width, size.height), &image_info);
        let surface = self.gr_context.with_context(|context| {
            surfaces::wrap_backend_render_target(
                context,
                &render_target,
                if session.is_y_flipped() { SurfaceOrigin::TopLeft } else { SurfaceOrigin::BottomLeft },
                if session.is_rgba() { ColorType::RGBA8888 } else { ColorType::BGRA8888 },
                ColorSpace::new_srgb(),
                Some(surface_props),
            )
        });
        let surface = surface.unwrap_or_else(|| panic!("Surface can't be created with the provided render target"));
        *wrapped = Some(WrappedSurface { key, render_target, surface: surface.clone() });
        surface
    }
}

/// Disposes a Vulkan session unless the Skia session took it over; what the
/// `finally` block of a managed implementation does.
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

impl ISkiaGpuRenderTarget for VulkanSkiaRenderTarget {
    fn dispose(&self) {
        // Skia's wrapper goes before the image it wraps.
        self.wrapped.borrow_mut().take();
        self.target.dispose();
    }

    fn begin_rendering_session(
        &self,
        _scene_info: &RenderTargetSceneInfo,
        surface_props: &skia_safe::SurfaceProps,
    ) -> Rc<dyn ISkiaGpuRenderSession> {
        // TODO: use expectedPixelSize
        let session = self.target.begin_draw();
        let guard = SessionGuard { session: session.clone(), success: Cell::new(false) };

        let size = session.size();
        let scaling = session.scaling();
        if size.width <= 0 || size.height <= 0 || scaling < 0.0 {
            panic!("Can't create drawing context for surface with {size} size and {scaling} scaling");
        }

        // No reset of the context: Ganesh on Vulkan keeps no state of the API that FerroUI's commands change, and the
        // one thing they change, the layout of the image, is told to Skia by `surface_for`.
        let surface = self.surface_for(&*session, surface_props);
        let queue_family = self.vulkan.device().graphics_queue_family_index();

        guard.success.set(true);

        Rc::new(VulkanSkiaRenderSession::new(self.gr_context.clone(), surface, session, queue_family))
    }

    fn state(&self) -> PlatformRenderTargetState {
        self.target.state()
    }
}

/// One frame rendered with Skia to the image of a Vulkan session.
pub(crate) struct VulkanSkiaRenderSession {
    gr_context: Rc<GaneshGrContext>,
    sk_surface: RefCell<Option<Surface>>,
    vulkan_session: Rc<dyn IVulkanRenderSession>,
    surface_origin: SkiaSurfaceOrigin,
    /// The queue family the image stays owned by when Skia is done with it.
    queue_family: u32,
    disposed: Cell<bool>,
}

impl VulkanSkiaRenderSession {
    pub fn new(
        gr_context: Rc<GaneshGrContext>,
        surface: Surface,
        vulkan_session: Rc<dyn IVulkanRenderSession>,
        queue_family: u32,
    ) -> Self {
        let surface_origin =
            if vulkan_session.is_y_flipped() { SkiaSurfaceOrigin::TopLeft } else { SkiaSurfaceOrigin::BottomLeft };
        Self {
            gr_context,
            sk_surface: RefCell::new(Some(surface)),
            vulkan_session,
            surface_origin,
            queue_family,
            disposed: Cell::new(false),
        }
    }
}

impl ISkiaGpuRenderSession for VulkanSkiaRenderSession {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        if let Some(mut surface) = self.sk_surface.borrow_mut().take() {
            if !self.gr_context.is_disposed() {
                // Skia leaves the image in the layout of a colour attachment, which is where the copy to the swap
                // chain expects it and returns it: Skia's record of the layout stays true for the next frame. The
                // flush records; the one submission of the frame is the context's flush below.
                let state = mutable_texture_states::new_vulkan(
                    skia_safe::gpu::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    self.queue_family,
                );
                self.gr_context.with_context(|context| {
                    context.flush_surface_with_texture_state(&mut surface, &FlushInfo::default(), Some(&state));
                });
            }
        }
        self.gr_context.flush();
        self.vulkan_session.dispose();
    }

    fn gr_context(&self) -> Rc<dyn ISkiaGrContext> {
        self.gr_context.clone()
    }

    fn sk_surface(&self) -> Surface {
        self.sk_surface.borrow().clone().expect("the Vulkan render session has been disposed")
    }

    fn scale_factor(&self) -> f64 {
        self.vulkan_session.scaling()
    }

    fn surface_origin(&self) -> SkiaSurfaceOrigin {
        self.surface_origin
    }
}
