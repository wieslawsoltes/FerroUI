//! The graphics device of the hybrid and the GPU mode.
//!
//! Both modes draw with `wgpu`: [`VelloWgpuDevice`] is the device and the
//! queue with what the renderers keep on them. A device is made without a
//! surface (tests, scenes drawn into memory) or over the graphics device
//! of the platform ([`IVelloGpu`]; [`metal`]: the Metal device and the
//! drawables of a window of the macOS platform).
//!
//! Built with the features `hybrid` and `gpu` of the crate.

mod i_vello_gpu;
#[cfg(target_os = "macos")]
pub mod metal;
mod vello_wgpu_device;


pub use i_vello_gpu::{create_window_scene_sink, render_window_scene, IVelloGpu};
pub use vello_wgpu_device::{VelloWgpuDevice, VelloWgpuDeviceError};

use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphicsContext};
use std::rc::Rc;

/// The GPU of the backend for a graphics context of a platform.
///
/// # Panics
/// Panics when the graphics context is of a kind the backend cannot render
/// with (it draws with the Metal device of the macOS platform), and when
/// `wgpu` does not take the device.
pub fn create_gpu(graphics_context: &Rc<dyn IPlatformGraphicsContext>) -> Rc<dyn IVelloGpu> {
    let features: &dyn IOptionalFeatureProvider = &**graphics_context;

    #[cfg(target_os = "macos")]
    if let Some(metal) = features.try_get::<dyn ferroui_metal::IMetalDevice>() {
        return match metal::VelloMetalGpu::new(metal) {
            Ok(gpu) => gpu,
            Err(error) => panic!("{error}"),
        };
    }

    let _ = features;
    panic!("Graphics context of type is not supported");
}

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
