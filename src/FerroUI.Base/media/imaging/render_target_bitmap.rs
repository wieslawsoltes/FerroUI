use crate::media::imaging::{Bitmap, BitmapEncoderOptions, IBitmap};
use crate::media::{Colors, DrawingContext, IImage, IImageBrushSource, PlatformDrawingContext};
use crate::platform::{self, IBitmapImpl, IDrawingContextImpl, IRenderTargetBitmapImpl};
use crate::rendering::ImmediateRenderer;
use crate::utilities::{RefCountable, RefCounted};
use crate::{PixelSize, Rect, Ref, Size, Vector, Visual};
use std::any::Any;
use std::io::Write;
use std::ops::Deref;
use std::rc::Rc;

/// A bitmap that holds the rendering of a visual.
pub struct RenderTargetBitmap {
    base: Bitmap,
    platform_impl: RefCounted<dyn IRenderTargetBitmapImpl>,
}

impl Deref for RenderTargetBitmap {
    type Target = Bitmap;

    #[inline]
    fn deref(&self) -> &Bitmap {
        &self.base
    }
}

impl RenderTargetBitmap {
    /// Creates a bitmap of the given size, in pixels, at 96 DPI.
    pub fn new(pixel_size: PixelSize) -> RenderTargetBitmap {
        Self::with_dpi(pixel_size, Vector::new(96.0, 96.0))
    }

    /// Creates a bitmap of the given size, in pixels, and DPI.
    pub fn with_dpi(pixel_size: PixelSize, dpi: Vector) -> RenderTargetBitmap {
        let item = Self::create_impl(pixel_size, dpi);
        let release = item.clone();
        Self::from_ref(RefCountable::create(item, move || release.dispose()))
    }

    fn from_ref(platform_impl: RefCounted<dyn IRenderTargetBitmapImpl>) -> RenderTargetBitmap {
        let base_impl: RefCounted<dyn IBitmapImpl> = platform_impl.clone_as(|item| item as Rc<dyn IBitmapImpl>);
        let base = Bitmap::from_ref(&base_impl);
        base_impl.dispose();
        RenderTargetBitmap { base, platform_impl }
    }

    /// The platform-specific bitmap implementation.
    pub fn platform_impl(&self) -> &RefCounted<dyn IRenderTargetBitmapImpl> {
        &self.platform_impl
    }

    /// Renders a visual to the bitmap.
    pub fn render(&self, visual: &Ref<Visual>) {
        let mut context = self.create_drawing_context();
        ImmediateRenderer::render(&mut context, visual);
        context.dispose();
    }

    fn create_impl(size: PixelSize, dpi: Vector) -> Rc<dyn IRenderTargetBitmapImpl> {
        platform::render_interface().create_render_target_bitmap(size, dpi)
    }

    /// Creates a drawing context that draws into the bitmap, after clearing
    /// the bitmap. Disposing (or dropping) the context finishes the drawing.
    pub fn create_drawing_context(&self) -> DrawingContext<'static> {
        self.create_drawing_context_with_clear(true)
    }

    /// Creates a drawing context that draws into the bitmap, clearing the
    /// bitmap first when `clear` is set.
    pub fn create_drawing_context_with_clear(&self, clear: bool) -> DrawingContext<'static> {
        let platform = self.create_platform_drawing_context(clear);
        DrawingContext::owned(Box::new(PlatformDrawingContext::new(platform)))
    }

    /// Creates the platform drawing context that draws into the bitmap,
    /// clearing the bitmap first when `clear` is set.
    pub fn create_platform_drawing_context(&self, clear: bool) -> Box<dyn IDrawingContextImpl> {
        let mut platform = self.platform_impl.item().create_drawing_context();
        if clear {
            platform.clear(Colors::TRANSPARENT);
        }
        platform
    }

    /// Releases the bitmap's references to the platform bitmap.
    pub fn dispose(&self) {
        self.platform_impl.dispose();
        self.base.dispose();
    }
}

impl IImage for RenderTargetBitmap {
    fn size(&self) -> Size {
        self.base.size()
    }

    fn draw(&self, context: &mut DrawingContext<'_>, source_rect: Rect, dest_rect: Rect) {
        IImage::draw(&self.base, context, source_rect, dest_rect)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_bitmap(&self) -> Option<&dyn IBitmap> {
        Some(self)
    }
}

impl IBitmap for RenderTargetBitmap {
    fn dpi(&self) -> Vector {
        self.base.dpi()
    }

    fn pixel_size(&self) -> PixelSize {
        self.base.pixel_size()
    }

    fn platform_impl(&self) -> &RefCounted<dyn IBitmapImpl> {
        self.base.platform_impl()
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> std::io::Result<()> {
        self.base.save(stream, options)
    }

    fn dispose(&self) {
        RenderTargetBitmap::dispose(self)
    }
}

impl IImageBrushSource for RenderTargetBitmap {
    fn bitmap(&self) -> Option<&RefCounted<dyn IBitmapImpl>> {
        IImageBrushSource::bitmap(&self.base)
    }
}
