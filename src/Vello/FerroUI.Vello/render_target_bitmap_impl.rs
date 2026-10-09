use crate::framebuffer_render_target::FramebufferRenderTarget;
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::vello_options::VelloRenderingMode;
use crate::writeable_bitmap_impl::WriteableBitmapImpl;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::surfaces::{
    FuncFramebufferRenderTarget, IFramebufferPlatformSurface, IFramebufferRenderTarget, IPlatformRenderSurface,
};
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, ILockedFramebuffer, IReadableBitmapImpl, IRenderTarget,
    IRenderTargetBitmapImpl, IWriteableBitmapImpl, PixelFormat, RenderTargetSceneInfo,
};
use ferroui_base::rendering::composition::CompositionTransparencyLevel;
use ferroui_base::utilities::ThreadBound;
use ferroui_base::{PixelSize, Vector};
use peniko::ImageData;
use std::any::Any;
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::Arc;

/// A render target bitmap: a writeable bitmap that is drawn into through a
/// framebuffer render target over its own pixels.
pub struct RenderTargetBitmapImpl {
    bitmap: Arc<WriteableBitmapImpl>,
    /// The render target the bitmap is drawn into with, on the thread that
    /// created the bitmap.
    render_target: ThreadBound<FramebufferRenderTarget>,
}

impl RenderTargetBitmapImpl {
    /// Creates a render target bitmap in the pixel format the renderers
    /// draw. `rendering_modes` are the modes scenes are drawn in, in the
    /// order they are tried.
    pub fn new(size: PixelSize, dpi: Vector, rendering_modes: Vec<VelloRenderingMode>) -> Self {
        let bitmap = WriteableBitmapImpl::new(size, dpi, PixelFormat::RGBA8888, AlphaFormat::Premul);
        let render_target = FramebufferRenderTarget::from_render_target(
            Self::framebuffer_render_target(&bitmap),
            true,
            rendering_modes,
        );

        Self { bitmap, render_target: ThreadBound::new(render_target) }
    }

    fn framebuffer_render_target(bitmap: &Arc<WriteableBitmapImpl>) -> Rc<dyn IFramebufferRenderTarget> {
        let bitmap = bitmap.clone();
        Rc::new(FuncFramebufferRenderTarget::new(move || bitmap.lock()))
    }

    /// Whether the contents of the bitmap have been lost. Never the case for
    /// a bitmap in memory.
    pub fn is_corrupted(&self) -> bool {
        false
    }
}

impl IBitmapImpl for RenderTargetBitmapImpl {
    fn dpi(&self) -> Vector {
        self.bitmap.dpi()
    }

    fn pixel_size(&self) -> PixelSize {
        self.bitmap.pixel_size()
    }

    fn version(&self) -> i32 {
        self.bitmap.version()
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        IBitmapImpl::save(&*self.bitmap, stream, options)
    }

    fn dispose(&self) {
        if self.render_target.is_on_thread() {
            self.render_target.get().dispose();
        }
        self.bitmap.dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_readable_bitmap(&self) -> Option<&dyn IReadableBitmapImpl> {
        Some(self)
    }
}

impl IReadableBitmapImpl for RenderTargetBitmapImpl {
    fn format(&self) -> Option<PixelFormat> {
        self.bitmap.format()
    }

    fn alpha_format(&self) -> Option<AlphaFormat> {
        self.bitmap.alpha_format()
    }

    fn lock(&self) -> Rc<dyn ILockedFramebuffer> {
        self.bitmap.lock()
    }
}

impl IWriteableBitmapImpl for RenderTargetBitmapImpl {}

impl IRenderTargetBitmapImpl for RenderTargetBitmapImpl {
    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        let scene_info = RenderTargetSceneInfo::new(
            self.pixel_size(),
            self.dpi().x / 96.0,
            CompositionTransparencyLevel::None,
        );

        self.render_target.get().create_drawing_context(&scene_info).0
    }
}

impl IDrawableBitmapImpl for RenderTargetBitmapImpl {
    fn image(&self) -> Option<ImageData> {
        self.bitmap.image()
    }
}

impl IPlatformRenderSurface for RenderTargetBitmapImpl {
    fn as_framebuffer_surface(&self) -> Option<&dyn IFramebufferPlatformSurface> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IFramebufferPlatformSurface for RenderTargetBitmapImpl {
    fn create_framebuffer_render_target(&self) -> Rc<dyn IFramebufferRenderTarget> {
        Self::framebuffer_render_target(&self.bitmap)
    }
}
