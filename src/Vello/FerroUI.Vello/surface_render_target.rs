use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::helpers::image_saving_helper;
use crate::helpers::pixel_format_helper::{scene_size, to_image};
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::scene::create_scene_sink;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, RenderTargetProperties};
use ferroui_base::{PixelSize, Vector};
use peniko::{BlendMode, ImageData};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::rc::Rc;

/// Create info of a [`SurfaceRenderTarget`].
#[derive(Clone, Default)]
pub struct SurfaceRenderTargetCreateInfo {
    /// Width of a render target.
    pub width: i32,

    /// Height of a render target.
    pub height: i32,

    /// Dpi used when rendering to a surface.
    pub dpi: Vector,

    /// The rendering modes in the order they are tried.
    pub rendering_modes: Vec<VelloRenderingMode>,

    /// Apply the DPI as a hidden transform when drawing.
    pub use_scaled_drawing: bool,
}

/// The pixels of a surface in memory: premultiplied RGBA without padding.
struct SurfacePixels {
    /// `None` once the surface is disposed.
    data: Option<Vec<u8>>,
    /// Whether anything was drawn: a surface nothing was drawn to has no
    /// backdrop for the next scene.
    has_content: bool,
    /// The pixels as an image, until they change.
    image: Option<ImageData>,
}

/// Vello render target that writes to a surface in memory: a layer.
///
/// A scene is rendered into the pixels of the surface in whatever rendering
/// mode draws it; a layer that stays on the GPU is part of the GPU modes
/// (design document, stages 7 and 8).
pub struct SurfaceRenderTarget {
    use_scaled_drawing: bool,
    pixels: Rc<RefCell<SurfacePixels>>,
    rendering_modes: Vec<VelloRenderingMode>,
    dpi: Vector,
    pixel_size: PixelSize,
    version: Rc<Cell<i32>>,
}

impl SurfaceRenderTarget {
    /// Creates a surface render target. A surface is at least one pixel in
    /// each direction.
    ///
    /// # Panics
    /// Panics when the surface is larger than a scene can be.
    pub fn new(create_info: SurfaceRenderTargetCreateInfo) -> Self {
        let pixel_size = PixelSize::new(create_info.width.max(1), create_info.height.max(1));
        let (width, height) = scene_size(pixel_size);

        Self {
            use_scaled_drawing: create_info.use_scaled_drawing,
            pixels: Rc::new(RefCell::new(SurfacePixels {
                data: Some(vec![0u8; width as usize * height as usize * 4]),
                has_content: false,
                image: None,
            })),
            rendering_modes: create_info.rendering_modes,
            dpi: create_info.dpi,
            pixel_size,
            version: Rc::new(Cell::new(1)),
        }
    }

    /// The properties of this render target.
    pub fn properties(&self) -> RenderTargetProperties {
        RenderTargetProperties::default()
    }

    /// Whether the render target lives on a GPU context and can only be used
    /// with it. Never the case for pixels in memory.
    pub fn has_render_context_affinity(&self) -> bool {
        false
    }

    fn image_of(pixels: &mut SurfacePixels, pixel_size: PixelSize) -> Option<ImageData> {
        if pixels.image.is_none() {
            pixels.image = Some(to_image(pixels.data.as_ref()?.clone(), pixel_size));
        }
        pixels.image.clone()
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
        let image = self.image().ok_or_else(|| io::Error::other("SurfaceRenderTarget has been disposed"))?;
        image_saving_helper::save_image(&image, stream, options)
    }

    fn dispose(&self) {
        let mut pixels = self.pixels.borrow_mut();
        pixels.data = None;
        pixels.image = None;
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
            .unwrap_or_else(|| panic!("The layer can only be blitted onto a drawing context of the Vello backend"));

        if let Some(image) = self.image() {
            context.draw_image_in_device_space(image, BlendMode::default());
        }
    }

    fn can_blit(&self) -> bool {
        true
    }

    fn is_corrupted(&self) -> bool {
        false
    }

    fn create_shared_snapshot(&self) -> std::sync::Arc<ferroui_base::platform::SharedBitmapImpl> {
        let image = self.image().unwrap_or_else(|| panic!("SurfaceRenderTarget has been disposed"));
        std::sync::Arc::new(ImmutableBitmap::from_image(image))
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        let (width, height) = scene_size(self.pixel_size);
        let sink = create_scene_sink(&self.rendering_modes, width, height);

        let backdrop = {
            let mut pixels = self.pixels.borrow_mut();
            if pixels.has_content {
                Self::image_of(&mut pixels, self.pixel_size)
            } else {
                None
            }
        };

        let pixels = self.pixels.clone();
        let create_info = CreateInfo {
            sink,
            backdrop,
            on_finished: Box::new(move |sink| {
                let mut pixels = pixels.borrow_mut();
                pixels.image = None;
                pixels.has_content = true;
                if let Some(data) = pixels.data.as_mut() {
                    sink.render_to_pixels(data);
                }
            }),
            scale_drawing_to_dpi: self.use_scaled_drawing,
            dpi: self.dpi,
            rendering_modes: self.rendering_modes.clone(),
        };

        let version = self.version.clone();
        Box::new(DrawingContextImpl::new(create_info, vec![Box::new(move || version.set(version.get() + 1))]))
    }
}

impl IDrawableBitmapImpl for SurfaceRenderTarget {
    fn image(&self) -> Option<ImageData> {
        Self::image_of(&mut self.pixels.borrow_mut(), self.pixel_size)
    }
}
