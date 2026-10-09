use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::helpers::pixel_format_helper::{from_premul_rgba, scene_size, to_image, to_premul_rgba};
use crate::scene::create_scene_sink;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::platform::surfaces::{IFramebufferPlatformSurface, IFramebufferRenderTarget};
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Vello render target that renders to a framebuffer surface. No GPU
/// acceleration available: a scene is rendered into memory and written to
/// the framebuffer in its format.
///
/// The Skia backend draws into the framebuffer itself when Skia has its
/// format and through a conversion shim otherwise; the renderers of the
/// Vello project draw one format (premultiplied RGBA without padding), so
/// every frame goes the way of the shim. Drawing a framebuffer of that very
/// format in place is an optimization left open (design document, risks).
pub struct FramebufferRenderTarget {
    use_scaled_drawing: bool,
    rendering_modes: Vec<VelloRenderingMode>,
    render_target: RefCell<Option<Rc<dyn IFramebufferRenderTarget>>>,
}

impl FramebufferRenderTarget {
    /// Creates a framebuffer render target for a framebuffer surface.
    pub fn new(
        platform_surface: &dyn IFramebufferPlatformSurface,
        use_scaled_drawing: bool,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        Self::from_render_target(
            platform_surface.create_framebuffer_render_target(),
            use_scaled_drawing,
            rendering_modes,
        )
    }

    /// Creates a framebuffer render target over the render target of a
    /// framebuffer surface.
    pub fn from_render_target(
        render_target: Rc<dyn IFramebufferRenderTarget>,
        use_scaled_drawing: bool,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        Self { use_scaled_drawing, rendering_modes, render_target: RefCell::new(Some(render_target)) }
    }
}

impl IRenderTarget for FramebufferRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties {
            retains_previous_frame_contents: self
                .render_target
                .borrow()
                .as_ref()
                .is_some_and(|target| target.retains_frame_contents()),
            is_suitable_for_direct_rendering: true,
        }
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let render_target =
            self.render_target.borrow().clone().unwrap_or_else(|| panic!("FramebufferRenderTarget has been disposed"));

        let (framebuffer, lock_properties) = render_target.lock(scene_info);
        let size = framebuffer.size();
        if size.width <= 0 || size.height <= 0 {
            panic!("Unable to create a surface with size {}x{}", size.width, size.height);
        }

        let (format, alpha_format) = (framebuffer.format(), framebuffer.alpha_format());
        let row_bytes = framebuffer.row_bytes().max(0) as usize;
        let (width, height) = scene_size(size);

        // What the framebuffer holds is what the scene is drawn over,
        // whether it is the frame before or not: the Skia backend draws
        // into the framebuffer itself, and a render target bitmap is drawn
        // into again and again. A framebuffer that holds nothing (every
        // byte is zero) needs no backdrop.
        let mut backdrop = None;
        framebuffer.with_data(&mut |data| {
            if data.iter().any(|byte| *byte != 0) {
                backdrop = Some(to_image(to_premul_rgba(data, row_bytes, size, format, alpha_format), size));
            }
        });

        let properties =
            RenderTargetDrawingContextProperties { previous_frame_is_retained: lock_properties.previous_frame_is_retained };

        let presented = framebuffer.clone();
        let create_info = CreateInfo {
            sink: create_scene_sink(&self.rendering_modes, width, height),
            backdrop,
            on_finished: Box::new(move |sink| {
                let mut rgba = vec![0u8; width as usize * height as usize * 4];
                sink.render_to_pixels(&mut rgba);
                presented.with_data(&mut |data| from_premul_rgba(&rgba, data, row_bytes, size, format, alpha_format));
            }),
            scale_drawing_to_dpi: self.use_scaled_drawing,
            dpi: framebuffer.dpi(),
            rendering_modes: self.rendering_modes.clone(),
        };

        let disposables: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || framebuffer.dispose())];

        (Box::new(DrawingContextImpl::new(create_info, disposables)), properties)
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        match &*self.render_target.borrow() {
            Some(render_target) => render_target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        if let Some(render_target) = self.render_target.borrow_mut().take() {
            render_target.dispose();
        }
    }
}
