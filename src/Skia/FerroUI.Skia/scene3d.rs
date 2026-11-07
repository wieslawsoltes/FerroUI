//! 3D scenes on the Skia backend (#608, `docs/porting/media3d.md`, step D3): the scene of a
//! `DrawScene3D` is rasterized by the CPU reference of `ferroui-render3d` at the resolution of the
//! device and drawn as an image over its bounds, under the transform, clip and opacity of the
//! canvas.
//!
//! On a raster surface this is the defined path (the surface has no GPU). On a GPU surface
//! (Graphite, Ganesh) it is the last resort until the GPU executors of steps D5 (Metal), D8 (GL)
//! and D9 (Vulkan) land: each such scene is counted (`ferroui_render3d::scenes_drawn_on_cpu`) and
//! the first is logged.
//!
//! The image of a scene is kept on the render thread while its snapshot, the size, the scale and
//! the edge mode are unchanged, so an unchanged viewport is neither rasterized nor uploaded again
//! (Graphite keeps the texture of an image while the image lives).

use crate::gpu::ISkiaGrContext;
use crate::sk_paint_cache::SkPaintCache;
use crate::skia_sharp_extensions::to_sk_rect;
use ferroui_base::rendering::scene3d::Scene3DFrame;
use ferroui_base::Matrix;
use ferroui_render3d::{report_scene_drawn_on_cpu, scene_pixel_size, CpuRenderOptions, Scene3DOutputCache};
use skia_safe as sk;
use std::cell::RefCell;

ferroui_base::ferro_thread_local! {
    /// The images of the scenes drawn recently on this thread.
    static OUTPUTS: RefCell<Scene3DOutputCache<sk::Image>> = RefCell::new(Scene3DOutputCache::new());
}

/// Draws the scene of `frame` on `canvas`, whose transform to the device is `device` (the
/// transform of the context with its scale to the DPI), with `opacity` and antialiased edges or
/// not. Returns `false` when the image of the scene could not be made.
pub(crate) fn draw_scene_3d(
    canvas: &sk::Canvas,
    gr_context: Option<&dyn ISkiaGrContext>,
    device: Matrix,
    opacity: f64,
    antialiased: bool,
    frame: &Scene3DFrame,
) -> bool {
    let bounds = frame.bounds;
    if bounds.width <= 0.0 || bounds.height <= 0.0 {
        return true;
    }
    let (size, scale) = scene_pixel_size(bounds, (device.m11, device.m12), (device.m21, device.m22));
    let options = CpuRenderOptions { antialiased };
    let image = OUTPUTS.with(|outputs| {
        outputs.borrow_mut().get_or_render(&frame.scene, size, scale, options, |image| {
            let info = sk::ImageInfo::new(
                (image.width as i32, image.height as i32),
                sk::ColorType::RGBA8888,
                sk::AlphaType::Premul,
                None,
            );
            // An image of pixels: uploaded to the GPU when drawn there, once while it is kept.
            crate::composition_effects::note_pixel_image();
            sk::images::raster_from_data(&info, sk::Data::new_copy(&image.rgba), image.width as usize * 4)
        })
    });
    let Some(image) = image else { return false };
    if gr_context.is_some() {
        report_scene_drawn_on_cpu("Skia (GPU)");
    }
    let image = crate::gpu::drawable_image(gr_context, image, false);
    let mut paint = SkPaintCache::get();
    paint.set_color(sk::Color::from_argb((255.0 * opacity.clamp(0.0, 1.0)) as u8, 255, 255, 255));
    paint.set_anti_alias(antialiased);
    let sampling = sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None);
    canvas.draw_image_rect_with_sampling_options(&image, None, to_sk_rect(bounds), sampling, &paint);
    SkPaintCache::return_reset(paint);
    true
}
