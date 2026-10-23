//! The graphics device of the hybrid and the GPU mode.
//!
//! Both modes draw with `wgpu`: [`VelloWgpuDevice`] is the device and the
//! queue with what the renderers keep on them. A device is made without a
//! surface (tests, scenes drawn into memory) or over the graphics device
//! of the platform ([`IVelloGpu`]; [`metal`]: the Metal device and the
//! drawables of a window of the macOS platform).
//!
//! Built with the features `hybrid` and `gpu` of the crate.

mod device_surface_render_target;
mod i_vello_gpu;
#[cfg(target_os = "macos")]
pub mod metal;
#[cfg(target_os = "macos")]
pub mod metal_offscreen;
mod vello_wgpu_device;
mod window_frame;
#[cfg(any(target_os = "linux", windows))]
pub mod gl_context_gpu;
#[cfg(any(target_os = "linux", windows))]
pub mod window_surface;


pub use device_surface_render_target::DeviceSurfaceRenderTarget;
pub use i_vello_gpu::{create_window_scene_sink, window_rendering_mode, IVelloGpu};
pub use vello_wgpu_device::{VelloWgpuDevice, VelloWgpuDeviceError};

use ferroui_base::platform::IPlatformGraphicsContext;
use std::rc::Rc;

/// The GPU of the backend for a graphics context of a platform, or `None`
/// when the backend draws the windows of the platform in software: the
/// backend context then draws into their framebuffers.
///
/// On macOS the device is the Metal device of the platform ([`metal`]); on
/// Linux and Windows it is a device of the backend's own, whatever the
/// graphics context is, and the windows are drawn through surfaces of
/// `wgpu` ([`window_surface`]): `None` when the machine has no adapter for
/// it (the reason is logged).
///
/// # Panics
/// Panics on macOS when `wgpu` does not take the Metal device of the
/// platform.
pub fn create_gpu(graphics_context: &Rc<dyn IPlatformGraphicsContext>) -> Option<Rc<dyn IVelloGpu>> {
    #[cfg(target_os = "macos")]
    if let Some(metal) = (&**graphics_context as &dyn ferroui_base::platform::IOptionalFeatureProvider)
        .try_get::<dyn ferroui_metal::IMetalDevice>()
    {
        return match metal::VelloMetalGpu::new(metal) {
            Ok(gpu) => Some(gpu),
            Err(error) => panic!("{error}"),
        };
    }

    // A device that is hardware on Vulkan or Direct3D 12; otherwise the
    // context of OpenGL of the platform; otherwise (Linux) OpenGL through
    // EGL on a device of the backend's own.
    #[cfg(any(target_os = "linux", windows))]
    {
        let mut reasons = Vec::new();
        match window_surface::VelloSurfaceGpu::new(graphics_context.clone(), window_surface::PRIMARY_BACKENDS) {
            Ok(gpu) => return Some(gpu),
            Err(error) => reasons.push(error.reason),
        }
        let features: &dyn ferroui_base::platform::IOptionalFeatureProvider = &**graphics_context;
        if let Some(gl_context) = features.try_get::<dyn ferroui_opengl::IGlContext>() {
            match gl_context_gpu::VelloGlGpu::new(gl_context) {
                Ok(gpu) => return Some(gpu),
                Err(error) => reasons.push(error.reason),
            }
        } else {
            #[cfg(target_os = "linux")]
            match window_surface::VelloSurfaceGpu::new(graphics_context.clone(), wgpu::Backends::GL) {
                Ok(gpu) => return Some(gpu),
                Err(error) => reasons.push(error.reason),
            }
        }
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::VISUAL) {
            let reasons = reasons.join("; ");
            logger.log_with_values(None, "The Vello backend draws the windows in software: {Reason}", &[&reasons.as_str()]);
        }
        return None;
    }

    #[allow(unreachable_code)]
    None
}

#[cfg(all(test, feature = "hybrid"))]
mod layer_tests;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod tests;

use ferroui_base::logging::{LogArea, LogEventLevel, Logger};

/// A texture of the device of a sink that a scene is rendered into: the
/// texture of the drawable of a window.
pub struct VelloGpuTexture<'a> {
    /// The texture. It can be rendered to (`RENDER_ATTACHMENT`); its format
    /// has four 8 bit channels.
    pub texture: &'a wgpu::Texture,
}

/// The pixels of an image as premultiplied RGBA, the form the renderers
/// sample.
#[cfg(feature = "hybrid")]
pub(crate) fn premultiplied_rgba(image: &peniko::ImageData) -> std::borrow::Cow<'_, [u8]> {
    use std::borrow::Cow;

    let data = image.data.data();
    let swap = matches!(image.format, peniko::ImageFormat::Bgra8);
    let premultiply = matches!(image.alpha_type, peniko::ImageAlphaType::Alpha);

    if !swap && !premultiply {
        return Cow::Borrowed(data);
    }

    let mut rgba = data.to_vec();
    for pixel in rgba.chunks_exact_mut(4) {
        if swap {
            pixel.swap(0, 2);
        }
        if premultiply {
            premultiply_pixel(pixel);
        }
    }
    Cow::Owned(rgba)
}

/// Multiplies the color channels of a pixel by its alpha.
pub(crate) fn premultiply_pixel(pixel: &mut [u8]) {
    let alpha = u32::from(pixel[3]);
    if alpha == 255 {
        return;
    }
    for channel in &mut pixel[..3] {
        *channel = ((u32::from(*channel) * alpha + 127) / 255) as u8;
    }
}

/// Logs a scene that a renderer could not draw.
pub(crate) fn log_render_failure(mode: &str, error: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
        logger.log_with_values(None, "The {Mode} mode of the Vello backend could not render a scene: {Error}", &[&mode, &error]);
    }
}

/// How the colors of a texture relate to its alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VelloTextureAlpha {
    /// The colors are multiplied by the alpha: what the hybrid renderer
    /// draws and samples.
    Premultiplied,
    /// The colors are not multiplied by the alpha: what the compute
    /// renderer of the GPU mode writes (design document, section 1.3) and
    /// takes for a texture it samples.
    Straight,
}

/// A texture of a device that scenes are rendered into and painted with:
/// what a layer of a GPU mode holds, so that what was drawn on the device
/// is drawn from without leaving it. `Rgba8Unorm`, in the alpha form of the
/// renderer that draws into it.
pub struct VelloDeviceTexture {
    device: std::sync::Arc<VelloWgpuDevice>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    id: u64,
    alpha: VelloTextureAlpha,
}

impl std::fmt::Debug for VelloDeviceTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelloDeviceTexture")
            .field("id", &self.id)
            .field("width", &self.texture.width())
            .field("height", &self.texture.height())
            .finish()
    }
}

impl VelloDeviceTexture {
    /// Creates a texture of `Rgba8Unorm` that can be rendered to, sampled
    /// and read back: by the hybrid renderer (a render attachment) when
    /// its alpha is premultiplied, by the compute renderer (a storage
    /// texture) when it is straight. It holds nothing until a scene is
    /// rendered into it.
    pub fn new(
        device: &std::sync::Arc<VelloWgpuDevice>,
        width: u32,
        height: u32,
        alpha: VelloTextureAlpha,
    ) -> std::sync::Arc<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};

        // The identities of textures are apart from the identities of the
        // pixels of images (`peniko::Blob::id`, which counts from one): a
        // scene binds both by number.
        static NEXT_ID: AtomicU64 = AtomicU64::new(1 << 63);

        let usage = match alpha {
            VelloTextureAlpha::Premultiplied => wgpu::TextureUsages::RENDER_ATTACHMENT,
            VelloTextureAlpha::Straight => wgpu::TextureUsages::STORAGE_BINDING,
        };
        let texture = device.create_rgba_texture(width, height, usage | wgpu::TextureUsages::TEXTURE_BINDING);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        crate::perf::count(crate::perf::Phase::SurfaceOnDevice, u64::from(width.max(1)) * u64::from(height.max(1)) * 4);

        std::sync::Arc::new(Self {
            device: device.clone(),
            texture,
            view,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            alpha,
        })
    }

    /// The device of the texture.
    pub fn device(&self) -> &std::sync::Arc<VelloWgpuDevice> {
        &self.device
    }

    /// The texture.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// The view of the whole texture.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// How the colors of the texture relate to its alpha.
    pub fn alpha(&self) -> VelloTextureAlpha {
        self.alpha
    }

    /// The number a scene binds the texture by.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The width in pixels.
    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    /// The height in pixels.
    pub fn height(&self) -> u32 {
        self.texture.height()
    }

    /// Reads the pixels back as premultiplied RGBA, `width * 4` bytes a row
    /// without padding, or `None` when the device did not hand them over
    /// (it is lost). Waits for the device.
    pub fn read_pixels(&self) -> Option<Vec<u8>> {
        let mut pixels = vec![0u8; self.width() as usize * self.height() as usize * 4];
        if !self.device.read_texture(&self.texture, &mut pixels) {
            return None;
        }
        if self.alpha == VelloTextureAlpha::Straight {
            pixels.chunks_exact_mut(4).for_each(premultiply_pixel);
        }
        Some(pixels)
    }
}

/// Runs `f` with an autorelease pool of its own, where there are such
/// pools: the objects of Metal that `wgpu` and the drivers autorelease
/// while commands are encoded end with `f`. The thread that renders is a
/// thread of the render loop, which has no pool: without one those objects
/// would never be released.
pub(crate) fn with_autorelease_pool<R>(f: impl FnOnce() -> R) -> R {
    #[cfg(target_os = "macos")]
    {
        objc2::rc::autoreleasepool(|_| f())
    }
    #[cfg(not(target_os = "macos"))]
    {
        f()
    }
}
