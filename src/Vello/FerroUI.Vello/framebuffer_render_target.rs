use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl, VelloBackdrop};
use crate::helpers::pixel_format_helper::{from_premul_rgba, from_premul_rgba_rect, scene_size, to_image, to_premul_rgba};
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::surface_render_target::{SurfaceChange, SurfaceRenderTarget, SurfaceRenderTargetCreateInfo};
use crate::scene::{create_scene_sink, VelloSceneBrush, VelloSceneImage};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::platform::surfaces::{IFramebufferPlatformSurface, IFramebufferRenderTarget};
use ferroui_base::platform::{
    IDrawingContextImpl, IRenderTarget, PlatformRenderTargetState, RenderTargetDrawingContextProperties,
    RenderTargetProperties, RenderTargetSceneInfo,
};
use ferroui_base::platform::IBitmapImpl;
use ferroui_base::PixelSize;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Vello render target that renders to a framebuffer surface. No GPU
/// acceleration available: a scene is rendered into memory and written to
/// the framebuffer in its format.
///
/// The Skia backend draws into the framebuffer itself when Skia has its
/// format and through a conversion shim otherwise; the renderers of the
/// Vello project draw one format (premultiplied RGBA without padding), so
/// every frame goes the way of the shim.
///
/// The framebuffer of a window keeps a frame of its own in that format,
/// from frame to frame ([`SurfaceRenderTarget`]), and says that it retains
/// its frame and can be rendered to directly: the compositor draws what
/// changed straight into it, without a layer of its own, and the scene is
/// rendered only where it changed. The frame is converted into the
/// framebuffer when it is presented: where it changed when the platform
/// keeps the framebuffer from frame to frame, all of it otherwise.
pub struct FramebufferRenderTarget {
    use_scaled_drawing: bool,
    rendering_modes: Vec<VelloRenderingMode>,
    render_target: RefCell<Option<Rc<dyn IFramebufferRenderTarget>>>,
    /// Whether the framebuffer is the surface of a platform: its frames are
    /// the frames [`crate::perf`] times. A bitmap that is drawn into is not.
    presents_frames: bool,
    /// The frame of the window, once a frame was drawn: of the size and
    /// the DPI of the last framebuffer.
    frame: RefCell<Option<Rc<SurfaceRenderTarget>>>,
    /// The size of the framebuffer the frame was last converted into.
    presented_size: Cell<Option<PixelSize>>,
}

impl FramebufferRenderTarget {
    /// Creates a framebuffer render target for a framebuffer surface.
    pub fn new(
        platform_surface: &dyn IFramebufferPlatformSurface,
        use_scaled_drawing: bool,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        Self {
            presents_frames: true,
            ..Self::from_render_target(
                platform_surface.create_framebuffer_render_target(),
                use_scaled_drawing,
                rendering_modes,
            )
        }
    }

    /// Creates a framebuffer render target over the render target of a
    /// framebuffer surface.
    pub fn from_render_target(
        render_target: Rc<dyn IFramebufferRenderTarget>,
        use_scaled_drawing: bool,
        rendering_modes: Vec<VelloRenderingMode>,
    ) -> Self {
        Self {
            use_scaled_drawing,
            rendering_modes,
            render_target: RefCell::new(Some(render_target)),
            presents_frames: false,
            frame: RefCell::new(None),
            presented_size: Cell::new(None),
        }
    }

    /// The frame of the window for a framebuffer of `size` at `dpi`: the
    /// one of the frame before when it still fits, a new one otherwise.
    fn frame_for(&self, size: PixelSize, dpi: ferroui_base::Vector) -> Rc<SurfaceRenderTarget> {
        let mut frame = self.frame.borrow_mut();
        let fits = frame.as_ref().is_some_and(|frame| frame.pixel_size() == size && frame.dpi() == dpi);
        if !fits {
            if let Some(old) = frame.take() {
                IBitmapImpl::dispose(&*old);
            }
            *frame = Some(Rc::new(SurfaceRenderTarget::new(SurfaceRenderTargetCreateInfo {
                width: size.width,
                height: size.height,
                dpi,
                rendering_modes: self.rendering_modes.clone(),
                use_scaled_drawing: self.use_scaled_drawing,
            })));
        }
        frame.clone().unwrap_or_else(|| panic!("The frame of the window was just created"))
    }

    /// The drawing context of a frame of a window: drawn into the frame the
    /// target keeps, which is converted into the framebuffer when the
    /// context is disposed, before the framebuffer is presented.
    fn create_window_drawing_context(
        &self,
        render_target: Rc<dyn IFramebufferRenderTarget>,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let frame_start = crate::perf::frame_start();

        let (framebuffer, lock_properties) = render_target.lock(scene_info);
        let size = framebuffer.size();
        if size.width <= 0 || size.height <= 0 {
            panic!("Unable to create a surface with size {}x{}", size.width, size.height);
        }
        let (format, alpha_format) = (framebuffer.format(), framebuffer.alpha_format());
        let row_bytes = framebuffer.row_bytes().max(0) as usize;
        let frame = self.frame_for(size, framebuffer.dpi());
        let (width, height) = (frame.pixel_size().width.max(0) as usize, frame.pixel_size().height.max(0) as usize);

        // The framebuffer holds the frame before only when the platform
        // keeps it and the frame was converted into it at this size.
        let whole = !(lock_properties.previous_frame_is_retained && self.presented_size.get() == Some(size));
        self.presented_size.set(Some(size));

        // A new frame of a framebuffer the platform keeps starts with what
        // the framebuffer holds.
        if !frame.has_content() && lock_properties.previous_frame_is_retained {
            framebuffer.with_data(&mut |data| {
                if data.iter().any(|byte| *byte != 0) {
                    let rgba = to_premul_rgba(data, row_bytes, size, format, alpha_format);
                    frame.write_pixels(&mut |pixels| pixels.copy_from_slice(&rgba[..pixels.len()]));
                }
            });
        }

        // The frame holds the frame before, or a new frame what a framebuffer
        // the platform keeps holds (nothing but transparent pixels, when it
        // was not written to).
        let properties = RenderTargetDrawingContextProperties {
            previous_frame_is_retained: frame.has_content() || lock_properties.previous_frame_is_retained,
        };

        let drawn = frame.clone();
        let disposables: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || {
            let changed = drawn.take_changed();
            let rect = match (whole, changed) {
                (true, _) | (false, Some(SurfaceChange::All)) => Some((0, 0, width, height)),
                (false, Some(SurfaceChange::Rect(rect))) => {
                    Some((usize::from(rect.x0), usize::from(rect.y0), usize::from(rect.x1), usize::from(rect.y1)))
                }
                (false, None) => None,
            };
            if let (Some(rect), Some(image)) = (rect, IDrawableBitmapImpl::image(&*drawn)) {
                let _perf = crate::perf::scope(crate::perf::Phase::PixelCopy, ((rect.2 - rect.0) * (rect.3 - rect.1) * 4) as u64);
                framebuffer.with_data(&mut |data| {
                    from_premul_rgba_rect(image.data.data(), data, row_bytes, size, format, alpha_format, rect)
                });
            }
            {
                let _perf = crate::perf::scope(crate::perf::Phase::Present, 0);
                framebuffer.dispose();
            }
            crate::perf::frame_end(frame_start, "framebuffer", width as u32, height as u32);
        })];

        (Box::new(frame.create_drawing_context_with(disposables)), properties)
    }
}

impl IRenderTarget for FramebufferRenderTarget {
    fn properties(&self) -> RenderTargetProperties {
        // The framebuffer of a window: the frame is the target's own.
        if self.presents_frames {
            return RenderTargetProperties {
                retains_previous_frame_contents: true,
                is_suitable_for_direct_rendering: true,
                ..Default::default()
            };
        }
        RenderTargetProperties {
            retains_previous_frame_contents: self
                .render_target
                .borrow()
                .as_ref()
                .is_some_and(|target| target.retains_frame_contents()),
            is_suitable_for_direct_rendering: true,
            ..Default::default()
        }
    }

    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties) {
        let render_target =
            self.render_target.borrow().clone().unwrap_or_else(|| panic!("FramebufferRenderTarget has been disposed"));

        if self.presents_frames {
            return self.create_window_drawing_context(render_target, scene_info);
        }

        let frame_start = if self.presents_frames { crate::perf::frame_start() } else { None };

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
        //
        // A framebuffer of the very format the renderers draw (premultiplied
        // RGBA without padding) is drawn into where it is by the CPU sink,
        // which composes the scene over it: nothing is converted or copied.
        // Any other is given to the scene as an image, converted.
        let sink = create_scene_sink(&self.rendering_modes, width, height);
        let in_place = sink.rendering_mode() == VelloRenderingMode::Cpu
            && format == ferroui_base::platform::PixelFormats::RGBA8888
            && alpha_format == ferroui_base::platform::AlphaFormat::Premul
            && row_bytes == width as usize * 4;

        let mut has_content = false;
        framebuffer.with_data(&mut |data| has_content = data.iter().any(|byte| *byte != 0));
        let backdrop = has_content.then(|| {
            let framebuffer = framebuffer.clone();
            VelloBackdrop {
                composes_over_target: in_place,
                brush: Box::new(move || {
                    let mut image = None;
                    framebuffer.with_data(&mut |data| {
                        let _perf = crate::perf::scope(crate::perf::Phase::PixelCopy, data.len() as u64);
                        image = Some(to_image(to_premul_rgba(data, row_bytes, size, format, alpha_format), size));
                    });
                    image.map(|image| {
                        VelloSceneBrush::Image(VelloSceneImage {
                            image,
                            x_extend: peniko::Extend::Pad,
                            y_extend: peniko::Extend::Pad,
                            quality: peniko::ImageQuality::Low,
                            alpha: 1.0,
                        })
                    })
                }),
            }
        });

        let properties =
            RenderTargetDrawingContextProperties { previous_frame_is_retained: lock_properties.previous_frame_is_retained };

        let presented = framebuffer.clone();
        let create_info = CreateInfo {
            sink,
            backdrop,
            on_finished: Box::new(move |sink| {
                if in_place {
                    let length = width as usize * height as usize * 4;
                    presented.with_data(&mut |data| sink.render_to_pixels(&mut data[..length]));
                    return;
                }
                let mut rgba = vec![0u8; width as usize * height as usize * 4];
                sink.render_to_pixels(&mut rgba);
                let _perf = crate::perf::scope(crate::perf::Phase::PixelCopy, rgba.len() as u64);
                presented.with_data(&mut |data| from_premul_rgba(&rgba, data, row_bytes, size, format, alpha_format));
            }),
            scale_drawing_to_dpi: self.use_scaled_drawing,
            dpi: framebuffer.dpi(),
            rendering_modes: self.rendering_modes.clone(),
        };

        let disposables: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || {
            {
                let _perf = crate::perf::scope(crate::perf::Phase::Present, 0);
                framebuffer.dispose();
            }
            crate::perf::frame_end(frame_start, "framebuffer", u32::from(width), u32::from(height));
        })];

        (Box::new(DrawingContextImpl::new(create_info, disposables)), properties)
    }

    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        match &*self.render_target.borrow() {
            Some(render_target) => render_target.state(),
            None => PlatformRenderTargetState::DISPOSED,
        }
    }

    fn dispose(&self) {
        if let Some(frame) = self.frame.borrow_mut().take() {
            IBitmapImpl::dispose(&*frame);
        }
        if let Some(render_target) = self.render_target.borrow_mut().take() {
            render_target.dispose();
        }
    }
}
