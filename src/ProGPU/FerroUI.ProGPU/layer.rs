//! The layers of the backend: offscreen targets on the device a drawing context draws into and
//! another scene of the same device samples (the layer of the compositor, the intermediate
//! surfaces of visual brushes and effects).

use crate::bitmap::{unsupported_codec, ProGpuBitmap};
use crate::drawing_context::ProGpuDrawingContext;
use crate::gpu::{Gpu, GpuTexture};
use crate::notes;
use crate::scene::SceneBuilder;
use ferroui_base::media::imaging::BitmapEncoderOptions;
use ferroui_base::platform::{
    AlphaFormat, IBitmapImpl, IDrawingContextImpl, IDrawingContextLayerImpl, PixelFormat, SharedBitmapImpl,
};
use ferroui_base::{PixelSize, Rect, Vector};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io;
use std::rc::Rc;
use std::sync::Arc;

struct Inner {
    gpu: Option<Rc<Gpu>>,
    texture: Option<Rc<GpuTexture>>,
    /// The builder of the layer's scenes, reused across its frames.
    scene: RefCell<Option<SceneBuilder>>,
    size: PixelSize,
    dpi: Vector,
    failed: Cell<bool>,
}

/// A layer: a texture of the device with premultiplied content. A frame drawn into it goes over
/// its content (a clear replaces it), so the layer of the compositor keeps what a frame does not
/// redraw.
///
/// The frames of all layers of a context render with one engine, which keeps one scene: a layer
/// sends its scene with every frame. A scene that samples the layer samples its content at the
/// time that scene is rendered.
pub struct ProGpuLayer {
    inner: Rc<Inner>,
}

impl ProGpuLayer {
    /// A layer of `size` pixels on the device of `gpu`. Without a device the layer records and
    /// draws nothing.
    pub(crate) fn new(gpu: Option<Rc<Gpu>>, size: PixelSize, dpi: Vector) -> Self {
        let texture = gpu
            .as_ref()
            .filter(|gpu| gpu.offscreen().is_some())
            .map(|gpu| GpuTexture::new(gpu.device(), size.width.max(1) as u32, size.height.max(1) as u32));
        ProGpuLayer { inner: Rc::new(Inner { gpu, texture, scene: RefCell::new(None), size, dpi, failed: Cell::new(false) }) }
    }

    /// The texture of the layer, if it has a device.
    pub(crate) fn texture(&self) -> Option<&Rc<GpuTexture>> {
        self.inner.texture.as_ref()
    }
}

impl IBitmapImpl for ProGpuLayer {
    fn dpi(&self) -> Vector {
        self.inner.dpi
    }

    fn pixel_size(&self) -> PixelSize {
        self.inner.size
    }

    fn version(&self) -> i32 {
        self.inner.texture.as_ref().map_or(0, |texture| texture.version() as i32)
    }

    fn save(&self, _stream: &mut dyn io::Write, _options: &BitmapEncoderOptions) -> io::Result<()> {
        Err(unsupported_codec("encoding a layer"))
    }

    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl IDrawingContextLayerImpl for ProGpuLayer {
    fn blit(&self, context: &mut dyn IDrawingContextImpl) {
        let all = Rect::new(0.0, 0.0, self.inner.size.width as f64, self.inner.size.height as f64);
        context.draw_bitmap(self, 1.0, all, all);
    }

    fn can_blit(&self) -> bool {
        true
    }

    fn is_corrupted(&self) -> bool {
        self.inner.failed.get()
    }

    fn create_drawing_context(&self) -> Box<dyn IDrawingContextImpl> {
        let scene = self.inner.scene.borrow_mut().take().unwrap_or_default();
        let inner = self.inner.clone();
        Box::new(ProGpuDrawingContext::with_gpu(
            scene,
            self.inner.gpu.clone(),
            Some(Box::new(move |mut recording| {
                if let (Some(gpu), Some(texture)) = (&inner.gpu, &inner.texture) {
                    if let Some(engine) = gpu.offscreen() {
                        match recording.render(engine, texture.view(), texture.width(), texture.height(), true) {
                            Ok(()) => texture.changed(),
                            Err(error) => {
                                inner.failed.set(true);
                                notes::failure(&format!("a layer was not drawn: {error}"));
                            }
                        }
                    }
                }
                *inner.scene.borrow_mut() = Some(recording.scene);
            })),
        ))
    }

    fn create_shared_snapshot(&self) -> Arc<SharedBitmapImpl> {
        let size = self.inner.size;
        let bitmap = ProGpuBitmap::new(size, self.inner.dpi, PixelFormat::RGBA8888, AlphaFormat::Premul);
        if let Some(texture) = &self.inner.texture {
            if texture.width() == size.width as u32 && texture.height() == size.height as u32 {
                bitmap.set_pixels(&texture.read());
            }
        }
        Arc::new(bitmap)
    }
}
