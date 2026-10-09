use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl, VelloBackdrop};
use crate::gpu::{log_render_failure, VelloDeviceTexture, VelloGpuTexture, VelloTextureAlpha, VelloWgpuDevice};
use crate::helpers::image_saving_helper;
use crate::helpers::pixel_format_helper::{scene_size, to_image};
use crate::i_drawable_bitmap_impl::IDrawableBitmapImpl;
use crate::immutable_bitmap::ImmutableBitmap;
use crate::scene::{VelloSceneBrush, VelloSceneImage, VelloSceneTexture};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{
    IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, IDrawingContextLayerWithRenderContextAffinityImpl,
};
use ferroui_base::{PixelSize, Vector};
use peniko::{BlendMode, Extend, ImageData, ImageQuality};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::{self, Write};
use std::rc::Rc;
use std::sync::Arc;

/// What a surface on a device holds.
struct DeviceSurface {
    /// `None` once the surface is disposed.
    texture: Option<Arc<VelloDeviceTexture>>,
    /// The texture the next scene is rendered into, in a mode whose
    /// renderer replaces what its target holds: the scene paints with the
    /// texture that holds the content, and the two change places.
    spare: Option<Arc<VelloDeviceTexture>>,
    /// Whether anything was drawn: a surface nothing was drawn to holds
    /// nothing a scene would have to keep.
    has_content: bool,
    /// The pixels read back, until the texture is drawn into again.
    image: Option<ImageData>,
}

/// Vello render target that is a texture of a graphics device: a layer of
/// the hybrid and of the GPU mode.
///
/// What the surface in memory ([`SurfaceRenderTarget`]) is to the CPU mode:
/// a scene is rendered into the texture, on the device, and the layer is
/// drawn into a scene of the same device as a paint with the texture
/// ([`VelloSceneBrush::Texture`](crate::scene::VelloSceneBrush::Texture)),
/// so that the frame of a window never leaves the device. This is what the
/// surface of the Skia backend is on a GPU context. The pixels are read
/// back only where the contract asks for them in memory: a snapshot, a
/// file, a scene of another device or of the CPU mode.
///
/// How what the layer held is kept under a new scene differs by mode:
///
/// * **Hybrid**: the renderer composes the scene over the texture
///   ([`IVelloSceneSink::retain_target`](crate::scene::IVelloSceneSink::retain_target)):
///   one texture of premultiplied colors, and a frame that redraws a dirty
///   rectangle costs what the rectangle costs.
/// * **GPU**: the compute renderer replaces what its target holds. The
///   layer has two textures (of colors that are not premultiplied, as that
///   renderer writes and samples them): a scene starts with the one that
///   holds the content as its first paint and is rendered into the other.
///   Every frame of the layer draws all of it, on the device.
///
/// [`SurfaceRenderTarget`]: crate::SurfaceRenderTarget
pub struct DeviceSurfaceRenderTarget {
    device: Arc<VelloWgpuDevice>,
    /// The mode the scenes of the surface are drawn in.
    mode: VelloRenderingMode,
    surface: Rc<RefCell<DeviceSurface>>,
    rendering_modes: Vec<VelloRenderingMode>,
    dpi: Vector,
    pixel_size: PixelSize,
    version: Rc<Cell<i32>>,
    use_scaled_drawing: bool,
}

impl DeviceSurfaceRenderTarget {
    /// Whether a layer of scenes that are drawn in `mode` on `device` can
    /// be a texture of the device: in the hybrid and in the GPU mode, when
    /// the crate is built with the mode and the device runs it.
    pub fn is_available(mode: VelloRenderingMode, device: &VelloWgpuDevice) -> bool {
        match mode {
            VelloRenderingMode::Cpu => false,
            VelloRenderingMode::Hybrid => cfg!(feature = "hybrid"),
            VelloRenderingMode::Gpu => cfg!(feature = "gpu") && device.supports_compute(),
        }
    }

    fn alpha(mode: VelloRenderingMode) -> VelloTextureAlpha {
        match mode {
            VelloRenderingMode::Gpu => VelloTextureAlpha::Straight,
            _ => VelloTextureAlpha::Premultiplied,
        }
    }

    /// Creates a surface of `pixel_size` (at least a pixel in each
    /// direction) on `device`, whose scenes are drawn in `mode`
    /// ([`is_available`](Self::is_available)). `rendering_modes` are the
    /// modes of the intermediate surfaces of what is drawn into it.
    ///
    /// # Panics
    /// Panics when the surface is larger than a scene can be.
    pub fn new(
        device: Arc<VelloWgpuDevice>,
        mode: VelloRenderingMode,
        pixel_size: PixelSize,
        dpi: Vector,
        rendering_modes: Vec<VelloRenderingMode>,
        use_scaled_drawing: bool,
    ) -> Self {
        let pixel_size = PixelSize::new(pixel_size.width.max(1), pixel_size.height.max(1));
        let (width, height) = scene_size(pixel_size);
        let texture = VelloDeviceTexture::new(&device, u32::from(width), u32::from(height), Self::alpha(mode));

        Self {
            device,
            mode,
            surface: Rc::new(RefCell::new(DeviceSurface {
                texture: Some(texture),
                spare: None,
                has_content: false,
                image: None,
            })),
            rendering_modes,
            dpi,
            pixel_size,
            version: Rc::new(Cell::new(1)),
            use_scaled_drawing,
        }
    }

    /// The texture of the surface, or `None` once it is disposed.
    pub fn texture(&self) -> Option<Arc<VelloDeviceTexture>> {
        self.surface.borrow().texture.clone()
    }

    /// The device of the surface.
    pub fn device(&self) -> &Arc<VelloWgpuDevice> {
        &self.device
    }

    fn read_back(surface: &RefCell<DeviceSurface>, pixel_size: PixelSize) -> Option<ImageData> {
        let mut surface = surface.borrow_mut();
        if surface.image.is_none() {
            let texture = surface.texture.clone()?;
            let pixels = if surface.has_content {
                texture.read_pixels()?
            } else {
                vec![0u8; texture.width() as usize * texture.height() as usize * 4]
            };
            surface.image = Some(to_image(pixels, pixel_size));
        }
        surface.image.clone()
    }

    /// The surface as a brush for a scene: its texture for a scene of the
    /// device of the surface, its pixels read back for any other.
    pub(crate) fn brush(
        &self,
        context: &mut DrawingContextImpl,
        (x_extend, y_extend): (Extend, Extend),
        quality: ImageQuality,
        alpha: f32,
    ) -> Option<VelloSceneBrush> {
        if context.draws_on_device(&self.device) {
            let texture = self.texture()?;
            return Some(VelloSceneBrush::Texture(VelloSceneTexture { texture, x_extend, y_extend, quality, alpha }));
        }

        let image = self.image()?;
        Some(VelloSceneBrush::Image(VelloSceneImage { image, x_extend, y_extend, quality, alpha }))
    }
}

impl IBitmapImpl for DeviceSurfaceRenderTarget {
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
        let image = self.image().ok_or_else(|| io::Error::other("DeviceSurfaceRenderTarget has been disposed"))?;
        image_saving_helper::save_image(&image, stream, options)
    }

    fn dispose(&self) {
        let mut surface = self.surface.borrow_mut();
        surface.texture = None;
        surface.spare = None;
        surface.image = None;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDrawingContextLayerImpl for DeviceSurfaceRenderTarget {
    fn blit(&self, context: &mut dyn IDrawingContextImpl) {
        let context = context
            .as_any_mut()
            .downcast_mut::<DrawingContextImpl>()
            .unwrap_or_else(|| panic!("The layer can only be blitted onto a drawing context of the Vello backend"));

        if let Some(brush) = self.brush(context, (Extend::Pad, Extend::Pad), ImageQuality::Low, 1.0) {
            let (width, height) = (self.pixel_size.width as f64, self.pixel_size.height as f64);
            context.draw_brush_in_device_space(brush, width, height, BlendMode::default());
        }
    }

    fn can_blit(&self) -> bool {
        true
    }

    fn is_corrupted(&self) -> bool {
        // What a lost device held is gone.
        self.device.is_lost()
    }

    fn create_shared_snapshot(&self) -> Arc<ferroui_base::platform::SharedBitmapImpl> {
        let image = self.image().unwrap_or_else(|| panic!("DeviceSurfaceRenderTarget has been disposed or its device lost"));
        Arc::new(ImmutableBitmap::from_image(image))
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        Box::new(self.create_drawing_context_with(Vec::new()))
    }

    fn as_layer_with_render_context_affinity(&self) -> Option<&dyn IDrawingContextLayerWithRenderContextAffinityImpl> {
        Some(self)
    }
}

impl DeviceSurfaceRenderTarget {
    /// Whether a scene was rendered into the surface.
    pub fn has_content(&self) -> bool {
        self.surface.borrow().has_content
    }

    /// Creates a drawing context that draws into the surface. `disposables`
    /// run in order when the context is disposed, after the scene was
    /// rendered into the texture.
    pub fn create_drawing_context_with(&self, disposables: Vec<Box<dyn FnOnce()>>) -> DrawingContextImpl {
        let texture = self.texture().unwrap_or_else(|| panic!("DeviceSurfaceRenderTarget has been disposed"));
        let (width, height) = scene_size(self.pixel_size);
        let sink =
            crate::gpu::create_window_scene_sink(&self.device, &[self.mode], width, height, texture.texture().format());
        let composes_over_target = sink.capabilities().retained_targets;
        let has_content = self.surface.borrow().has_content;

        // The texture the scene is rendered into: the one that holds the
        // content when the renderer composes over it, the other one when
        // the renderer replaces what its target holds.
        let target = if composes_over_target {
            texture.clone()
        } else {
            let mut surface = self.surface.borrow_mut();
            surface
                .spare
                .get_or_insert_with(|| {
                    VelloDeviceTexture::new(&self.device, u32::from(width), u32::from(height), Self::alpha(self.mode))
                })
                .clone()
        };

        let backdrop = has_content.then(|| {
            if composes_over_target {
                // What the texture holds stays under the scene: the
                // renderer composes over it. Only a scene that cannot be
                // composed over its target (see `VelloBackdrop`) gets the
                // pixels, read back.
                let (surface, pixel_size) = (self.surface.clone(), self.pixel_size);
                VelloBackdrop {
                    composes_over_target: true,
                    brush: Box::new(move || {
                        Self::read_back(&surface, pixel_size).map(|image| {
                            VelloSceneBrush::Image(VelloSceneImage {
                                image,
                                x_extend: Extend::Pad,
                                y_extend: Extend::Pad,
                                quality: ImageQuality::Low,
                                alpha: 1.0,
                            })
                        })
                    }),
                }
            } else {
                // The scene starts with the texture that holds the content
                // and is rendered into the other one.
                let content = texture.clone();
                VelloBackdrop {
                    composes_over_target: false,
                    brush: Box::new(move || {
                        Some(VelloSceneBrush::Texture(VelloSceneTexture {
                            texture: content,
                            x_extend: Extend::Pad,
                            y_extend: Extend::Pad,
                            quality: ImageQuality::Low,
                            alpha: 1.0,
                        }))
                    }),
                }
            }
        });

        let surface = self.surface.clone();
        let mode = self.mode;
        let create_info = CreateInfo {
            sink,
            backdrop,
            on_finished: Box::new(move |sink| {
                let rendered = sink.render_to_texture(&VelloGpuTexture { texture: target.texture() });
                if let Err(error) = &rendered {
                    log_render_failure(if mode == VelloRenderingMode::Gpu { "GPU" } else { "hybrid" }, error);
                }

                let mut surface = surface.borrow_mut();
                surface.image = None;
                // A scene that could not be rendered leaves the layer as it
                // was.
                if rendered.is_ok() && surface.texture.is_some() {
                    surface.has_content = true;
                    if !composes_over_target {
                        surface.spare = surface.texture.replace(target);
                    }
                }
            }),
            scale_drawing_to_dpi: self.use_scaled_drawing,
            dpi: self.dpi,
            rendering_modes: self.rendering_modes.clone(),
        };

        let version = self.version.clone();
        let mut all: Vec<Box<dyn FnOnce()>> = vec![Box::new(move || version.set(version.get() + 1))];
        all.extend(disposables);
        DrawingContextImpl::new(create_info, all)
    }
}

impl IDrawingContextLayerWithRenderContextAffinityImpl for DeviceSurfaceRenderTarget {
    fn has_render_context_affinity(&self) -> bool {
        // The texture belongs to its device.
        true
    }

    fn create_non_affined_snapshot(&self) -> Arc<ferroui_base::platform::SharedBitmapImpl> {
        self.create_shared_snapshot()
    }
}

impl IDrawableBitmapImpl for DeviceSurfaceRenderTarget {
    fn image(&self) -> Option<ImageData> {
        Self::read_back(&self.surface, self.pixel_size)
    }
}
