//! Skia's Graphite on the device of a Vulkan context (#1365, part 2; the feature `graphite-vulkan`,
//! off by default): the context, its recorder, Graphite surfaces on images of `ferroui-vulkan`, and
//! the flush of a frame. The window target and the pipelined layer on Vulkan build on it (the
//! next parts); Ganesh stays the GPU of the default build and the fallback, never the CPU.

use super::vulkan_skia_gpu::with_backend_context;
use ferroui_vulkan::{IVulkanPlatformGraphicsContext, VulkanImageInfo};
use skia_safe::gpu::graphite::{self, InsertRecordingInfo};
use skia_safe::{ColorSpace, ColorType, ImageInfo, Surface, SurfaceProps};
use std::rc::Rc;

/// A Graphite context of a Vulkan device and the recorder its drawing goes through. The recorder
/// is declared first, so that it goes before the context that made it.
pub struct GraphiteVulkanContext {
    recorder: graphite::Recorder,
    context: graphite::Context,
    vulkan: Rc<dyn IVulkanPlatformGraphicsContext>,
}

impl GraphiteVulkanContext {
    /// A Graphite context of the device of `vulkan`, with the recordings inserted in order (as the
    /// window context on Metal, desktop-performance.md section 9.17). `None` when Skia cannot make
    /// one for the device: the caller takes Ganesh.
    pub fn new(vulkan: Rc<dyn IVulkanPlatformGraphicsContext>) -> Option<Self> {
        let current = vulkan.ensure_current();
        let options = ordered_recordings_options();
        let context = with_backend_context(&*vulkan, |backend| {
            graphite::vk::context_factory::make_vulkan(backend, &options)
        });
        let made = context.and_then(|mut context| context.make_recorder(None).map(|recorder| (recorder, context)));
        current.dispose();
        let (recorder, context) = made?;
        Some(Self { recorder, context, vulkan })
    }

    /// The recorder, for drawing.
    pub fn recorder(&mut self) -> &mut graphite::Recorder {
        &mut self.recorder
    }

    /// A Graphite surface on an image of `ferroui-vulkan` (its handle, size, format, tiling, usage
    /// and current layout), owned by the queue family of the device's main queue. The image needs
    /// the usage flags Graphite asks for (`IVulkanRenderTarget::request_image_usage` with
    /// `INPUT_ATTACHMENT`).
    pub fn wrap_image(
        &mut self,
        image: &VulkanImageInfo,
        color_type: ColorType,
        props: &SurfaceProps,
    ) -> Option<Surface> {
        let queue_family = self.vulkan.device().graphics_queue_family_index();
        let texture = backend_texture(image, queue_family);
        let color_space = ColorSpace::new_srgb();
        graphite::surfaces::wrap_backend_texture(&mut self.recorder, &texture, color_type, color_space, Some(props))
    }

    /// An offscreen surface of the recorder.
    pub fn make_surface(&mut self, info: &ImageInfo, props: &SurfaceProps) -> Option<Surface> {
        graphite::surfaces::render_target(&mut self.recorder, info, skia_safe::gpu::Mipmapped::No, Some(props), None)
    }

    /// Snaps the recorder, inserts the recording and submits it, without waiting. Returns whether
    /// the recording went in and was submitted.
    pub fn flush(&mut self) -> bool {
        let Some(mut recording) = self.recorder.snap() else {
            return self.context.submit(None);
        };
        let inserted = self.context.insert_recording(&InsertRecordingInfo::new(&mut recording))
            == graphite::InsertStatus::Success;
        inserted && self.context.submit(None)
    }

    /// Reads the pixels of a surface of the recorder back to the CPU, waiting for the GPU: a
    /// read-back (a screenshot, a test), never a step of a frame.
    pub fn read_pixels(
        &mut self,
        surface: &mut Surface,
        info: &ImageInfo,
        pixels: &mut [u8],
        row_bytes: usize,
    ) -> bool {
        self.flush();
        self.context.read_pixels(surface, info, pixels, row_bytes, (0, 0))
    }

    /// Whether the device was lost.
    pub fn is_lost(&self) -> bool {
        self.context.is_device_lost()
    }
}

/// Graphite's context options with `fRequireOrderedRecordings` (skia-safe has no setter; the
/// native field through `skia-bindings`, as on Metal).
fn ordered_recordings_options() -> graphite::ContextOptions {
    let mut options = graphite::ContextOptions::new();
    // SAFETY: `Handle<N>` is `repr(transparent)` over `UnsafeCell<N>`, itself transparent over the
    // native `skgpu::graphite::ContextOptions`, whose `fRequireOrderedRecordings` is a plain bool.
    unsafe {
        let native = &mut *(&mut options as *mut graphite::ContextOptions)
            .cast::<skia_bindings::skgpu_graphite_ContextOptions>();
        native.fRequireOrderedRecordings = true;
    }
    options
}

/// The Graphite backend texture of an image of `ferroui-vulkan`, through the patched bindings
/// (`C_BackendTextures_MakeVulkan`, FERROUI-PATCH.md of `external/rust-skia-bindings`).
fn backend_texture(image: &VulkanImageInfo, queue_family: u32) -> graphite::BackendTexture {
    let mut texture = graphite::BackendTexture::new();
    // SAFETY: the backend texture is a valid, default-constructed native value, which the call
    // destroys in place and constructs again (the binding's constructors do the same); the image
    // is only described, and outlives the surfaces made from it (the caller's contract).
    unsafe {
        let native =
            (&mut texture as *mut graphite::BackendTexture).cast::<skia_bindings::skgpu_graphite_BackendTexture>();
        skia_bindings::C_BackendTexture_destruct(native);
        skia_bindings::C_BackendTextures_MakeVulkan(
            native,
            image.pixel_size.width,
            image.pixel_size.height,
            image.format,
            image.tiling,
            image.usage_flags,
            image.layout,
            queue_family,
            image.handle,
        );
    }
    texture
}
