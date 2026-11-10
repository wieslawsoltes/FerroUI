//! The GPU objects the parts of a backend context share: the device, the engine of the offscreen
//! targets, the textures and the rendering of a recorded scene.

use crate::device::{ExternalImage, ProGpuDevice, ProGpuEngine, TargetFormat};
use crate::notes;
use crate::scene::{SceneBuildError, SceneBuilder};
use progpu_native_sys::webgpu::*;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;
use std::time::Instant;

/// A texture of `RGBA8Unorm` of the device (premultiplied content) with its view: the content of
/// a layer or of an offscreen frame. Scenes of the same device sample it as an external image.
pub(crate) struct GpuTexture {
    device: Rc<ProGpuDevice>,
    texture: WGPUTexture,
    view: WGPUTextureView,
    width: u32,
    height: u32,
    version: Cell<u64>,
}

impl GpuTexture {
    pub(crate) fn new(device: &Rc<ProGpuDevice>, width: u32, height: u32) -> Rc<Self> {
        let (width, height) = (width.max(1), height.max(1));
        let (texture, view) = device.create_target(width, height);
        Rc::new(Self { device: device.clone(), texture, view, width, height, version: Cell::new(1) })
    }

    pub(crate) fn device(&self) -> &Rc<ProGpuDevice> {
        &self.device
    }

    pub(crate) fn view(&self) -> WGPUTextureView {
        self.view
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn height(&self) -> u32 {
        self.height
    }

    /// The version of the content: it advances with every frame rendered into the texture.
    pub(crate) fn version(&self) -> u64 {
        self.version.get()
    }

    pub(crate) fn changed(&self) {
        self.version.set(self.version.get() + 1);
    }

    /// The content, read back: premultiplied `RGBA`, row by row without padding.
    pub(crate) fn read(&self) -> Vec<u8> {
        self.device.read_rgba8(self.texture, self.width, self.height)
    }
}

impl Drop for GpuTexture {
    fn drop(&mut self) {
        self.device.release_texture(self.texture, self.view);
    }
}

/// A texture a recorded scene samples: the identity of its resource in the scene and the texture,
/// kept alive until the frame is rendered.
pub(crate) struct SampledTexture {
    pub(crate) texture: Rc<GpuTexture>,
    pub(crate) resource_id: u64,
    pub(crate) generation: u64,
}

/// What a drawing context recorded: the scene, the first error of the recording and the textures
/// the scene samples.
pub(crate) struct Recording {
    pub(crate) scene: SceneBuilder,
    pub(crate) error: Option<SceneBuildError>,
    pub(crate) images: Vec<SampledTexture>,
}

impl Recording {
    /// Sends the recorded scene to `engine` and renders it into `view` (a view of the engine's
    /// format of `width` by `height` pixels), over the content of the target with `preserve`.
    pub(crate) fn render(&mut self, engine: &ProGpuEngine, view: WGPUTextureView, width: u32, height: u32, preserve: bool) -> Result<(), String> {
        if let Some(error) = self.error {
            return Err(error.to_string());
        }
        let (scene_id, generation) = (self.scene.scene_id(), self.scene.generation());
        let stream = self.scene.build().map_err(|error| error.to_string())?;
        let stream_bytes = stream.len();
        let started = crate::perf::enabled().then(Instant::now);
        let scene_metrics = engine.update_scene(stream)?;
        let updated = started.map(|_| Instant::now());
        // The table of the engine is ordered by the identifiers of the resources.
        let mut images: Vec<ExternalImage> = self
            .images
            .iter()
            .map(|image| ExternalImage {
                resource_id: image.resource_id,
                generation: image.generation,
                view: image.texture.view(),
                width: image.texture.width(),
                height: image.texture.height(),
            })
            .collect();
        images.sort_by_key(|image| image.resource_id);
        engine.bind_external_images(&images)?;
        let rendering = started.map(|_| Instant::now());
        let frame_metrics = engine.render_scene(scene_id, generation, view, width, height, preserve)?;
        if let (Some(started), Some(updated), Some(rendering)) = (started, updated, rendering) {
            crate::perf::scene(stream_bytes, updated - started, &scene_metrics, rendering.elapsed(), &frame_metrics);
        }
        Ok(())
    }
}

/// The device of a backend context and the engine of its offscreen targets (layers, offscreen
/// frames): one engine renders them all, a scene at a time.
pub(crate) struct Gpu {
    device: Rc<ProGpuDevice>,
    offscreen: OnceCell<Option<Rc<ProGpuEngine>>>,
}

impl Gpu {
    pub(crate) fn new(device: ProGpuDevice) -> Rc<Self> {
        Rc::new(Self { device: Rc::new(device), offscreen: OnceCell::new() })
    }

    pub(crate) fn device(&self) -> &Rc<ProGpuDevice> {
        &self.device
    }

    /// An engine of the device for targets of `format`, or `None` (with a logged failure) when
    /// the engine refuses the device.
    pub(crate) fn engine(&self, format: TargetFormat) -> Option<Rc<ProGpuEngine>> {
        match ProGpuEngine::new(&self.device, format) {
            Ok(engine) => Some(Rc::new(engine)),
            Err(error) => {
                notes::failure(&format!("no engine for the targets of {format:?}: {error}"));
                None
            }
        }
    }

    /// The engine of the offscreen targets (`RGBA8Unorm`), created at the first use.
    pub(crate) fn offscreen(&self) -> Option<&Rc<ProGpuEngine>> {
        self.offscreen.get_or_init(|| self.engine(TargetFormat::Rgba8)).as_ref()
    }
}

thread_local! {
    static DEFAULT: RefCell<Option<Option<Rc<Gpu>>>> = const { RefCell::new(None) };
}

/// The device on the default adapter for the calling thread: what renders the bitmaps that are
/// drawn into outside a backend context (render-target bitmaps keep their pixels in memory, so
/// their device is of no consequence to who reads them).
pub(crate) fn default_gpu() -> Option<Rc<Gpu>> {
    DEFAULT.with(|slot| {
        slot.borrow_mut()
            .get_or_insert_with(|| match ProGpuDevice::new() {
                Ok(device) => Some(Gpu::new(device)),
                Err(error) => {
                    notes::note_once("no-device", &format!("no WebGPU device, nothing is drawn: {error}"));
                    None
                }
            })
            .clone()
    })
}
