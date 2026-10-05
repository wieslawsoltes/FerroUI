use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::{ISkiaGpu, ISkiaGpuRenderSession, ISkiaGrContext, ISkiaSurface};
use crate::helpers::image_saving_helper;
use crate::helpers::pixel_format_helper;
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::immutable_bitmap::ImmutableBitmap;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, PixelFormat, RenderTargetProperties,
};
use ferroui_base::{PixelSize, Vector};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    surfaces, AlphaType, Canvas, Image, ImageInfo, Paint, PixelGeometry, Rect, SamplingOptions, Surface,
    SurfaceProps, SurfacePropsFlags,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::rc::Rc;

/// A plain Skia surface as a GPU surface that cannot be blitted.
struct SkiaSurfaceWrapper {
    surface: RefCell<Option<Surface>>,
}

impl ISkiaSurface for SkiaSurfaceWrapper {
    fn surface(&self) -> Surface {
        self.surface.borrow().clone().expect("SkiaSurfaceWrapper has been disposed")
    }

    fn can_blit(&self) -> bool {
        false
    }

    fn blit(&self, _canvas: &Canvas) {
        panic!("Blitting is not supported by this surface");
    }

    fn dispose(&self) {
        self.surface.borrow_mut().take();
    }
}

/// Create info of a [`SurfaceRenderTarget`].
#[derive(Clone, Default)]
pub struct SurfaceRenderTargetCreateInfo {
    /// Width of a render target.
    pub width: i32,

    /// Height of a render target.
    pub height: i32,

    /// Dpi used when rendering to a surface.
    pub dpi: Vector,

    /// Pixel format of a render target.
    pub format: Option<PixelFormat>,

    /// Render text without LCD (subpixel) rendering.
    pub disable_text_lcd_rendering: bool,

    /// GPU-accelerated context (optional).
    pub gr_context: Option<Rc<dyn ISkiaGrContext>>,

    /// The GPU that owns the context (optional).
    pub gpu: Option<Rc<dyn ISkiaGpu>>,

    /// The render session the target is created during (optional).
    pub session: Option<Rc<dyn ISkiaGpuRenderSession>>,

    /// Never ask the GPU for a surface of its own making.
    pub disable_manual_fbo: bool,

    /// Apply the DPI as a hidden transform when drawing.
    pub use_scaled_drawing: bool,
}

/// Skia render target that writes to a surface.
pub struct SurfaceRenderTarget {
    use_scaled_drawing: bool,
    surface: Rc<dyn ISkiaSurface>,
    disable_lcd_rendering: bool,
    gr_context: Option<Rc<dyn ISkiaGrContext>>,
    gpu: Option<Rc<dyn ISkiaGpu>>,
    dpi: Vector,
    pixel_size: PixelSize,
    version: Rc<Cell<i32>>,
}

impl SurfaceRenderTarget {
    /// Creates a surface render target.
    ///
    /// # Panics
    /// Panics when the surface cannot be created.
    pub fn new(create_info: SurfaceRenderTargetCreateInfo) -> Self {
        let pixel_size = PixelSize::new(create_info.width, create_info.height);

        let mut surface: Option<Rc<dyn ISkiaSurface>> = None;

        if !create_info.disable_manual_fbo {
            if let Some(gpu) = &create_info.gpu {
                surface = gpu.try_create_surface(pixel_size, create_info.session.as_ref());
            }
        }

        if surface.is_none() {
            if let Some(sk_surface) = Self::create_surface(
                create_info.gr_context.as_deref(),
                pixel_size.width,
                pixel_size.height,
                create_info.format,
            ) {
                surface = Some(Rc::new(SkiaSurfaceWrapper { surface: RefCell::new(Some(sk_surface)) }));
            }
        }

        let surface = surface.unwrap_or_else(|| panic!("Failed to create Skia render target surface"));

        Self {
            use_scaled_drawing: create_info.use_scaled_drawing,
            surface,
            disable_lcd_rendering: create_info.disable_text_lcd_rendering,
            gr_context: create_info.gr_context,
            gpu: create_info.gpu,
            dpi: create_info.dpi,
            pixel_size,
            version: Rc::new(Cell::new(1)),
        }
    }

    /// The properties of this render target.
    pub fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    /// Creates the backing surface: on the GPU when a GPU context is given,
    /// in memory otherwise.
    fn create_surface(
        gpu: Option<&dyn ISkiaGrContext>,
        width: i32,
        height: i32,
        format: Option<PixelFormat>,
    ) -> Option<Surface> {
        let image_info = Self::make_image_info(width, height, format);
        let props = SurfaceProps::new(SurfacePropsFlags::default(), PixelGeometry::RGBH);

        match gpu {
            Some(gpu) => gpu.create_surface(&image_info, &props),
            None => surfaces::raster(&image_info, None, Some(&props)),
        }
    }

    /// Makes the image info of the backing surface. A surface is at least one
    /// pixel in each direction.
    fn make_image_info(width: i32, height: i32, format: Option<PixelFormat>) -> ImageInfo {
        let color_type = pixel_format_helper::resolve_color_type(format);

        ImageInfo::new((width.max(1), height.max(1)), color_type, AlphaType::Premul, None)
    }

    /// Creates a Skia image from a snapshot of the surface.
    pub fn snapshot_image(&self) -> Image {
        self.surface.surface().image_snapshot()
    }

    /// Whether the render target lives on a GPU context and can only be used
    /// with it.
    pub fn has_render_context_affinity(&self) -> bool {
        self.gr_context.is_some()
    }

    fn create_raster_snapshot(&self) -> Option<Image> {
        let gr_context = self.gr_context.as_ref()?;
        gr_context.snapshot_to_raster(&mut self.surface.surface())
    }

    /// Creates a bitmap with a copy of the contents that can be used without
    /// the GPU context.
    ///
    /// # Panics
    /// Panics when the render target has no render context affinity or its
    /// contents cannot be read back.
    pub fn create_non_affined_snapshot(&self) -> Rc<dyn IBitmapImpl> {
        if !self.has_render_context_affinity() {
            panic!("The render target has no render context affinity");
        }

        let image = self.create_raster_snapshot().unwrap_or_else(|| panic!("Unable to read back the render target"));
        Rc::new(ImmutableBitmap::from_image(image, None))
    }
}

impl IBitmapImpl for SurfaceRenderTarget {
    fn dpi(&self) -> Vector {
        self.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    fn version(&self) -> i32 {
        self.version.get()
    }

    fn save(&self, stream: &mut dyn Write, options: &BitmapEncoderOptions) -> io::Result<()> {
        let image = match self.create_raster_snapshot() {
            Some(image) => image,
            None => self.snapshot_image(),
        };
        image_saving_helper::save_image(&image, stream, options)
    }

    fn dispose(&self) {
        self.surface.dispose();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDrawingContextLayerImpl for SurfaceRenderTarget {
    fn blit(&self, context: &mut dyn IDrawingContextImpl) {
        let context = context
            .as_any_mut()
            .downcast_mut::<DrawingContextImpl>()
            .unwrap_or_else(|| panic!("The layer can only be blitted onto a drawing context of the Skia backend"));
        let canvas = context.canvas();

        if self.surface.can_blit() {
            self.surface.blit(canvas);
        } else {
            let old_matrix = canvas.local_to_device();
            canvas.reset_matrix();
            self.surface.surface().draw(canvas, (0.0, 0.0), SamplingOptions::default(), None);
            canvas.set_matrix(&old_matrix);
        }
    }

    fn can_blit(&self) -> bool {
        true
    }

    fn is_corrupted(&self) -> bool {
        self.gpu.as_ref().is_some_and(|gpu| gpu.is_lost())
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        let mut surface = self.surface.surface();
        {
            let canvas = surface.canvas();
            canvas.restore_to_count(1);
            canvas.reset_matrix();
        }

        let create_info = CreateInfo {
            surface: Some(surface),
            dpi: self.dpi,
            scale_drawing_to_dpi: self.use_scaled_drawing,
            disable_subpixel_text_rendering: self.disable_lcd_rendering,
            gr_context: self.gr_context.clone(),
            gpu: self.gpu.clone(),
            ..CreateInfo::default()
        };

        let version = self.version.clone();
        Box::new(DrawingContextImpl::new(create_info, vec![Box::new(move || version.set(version.get() + 1))]))
    }
}

impl IDrawableBitmapImpl for SurfaceRenderTarget {
    fn draw(
        &self,
        canvas: &Canvas,
        source_rect: &Rect,
        dest_rect: &Rect,
        sampling_options: SamplingOptions,
        paint: &Paint,
    ) {
        let image = self.snapshot_image();
        canvas.draw_image_rect_with_sampling_options(
            image,
            Some((source_rect, SrcRectConstraint::Fast)),
            dest_rect,
            sampling_options,
            paint,
        );
    }
}
