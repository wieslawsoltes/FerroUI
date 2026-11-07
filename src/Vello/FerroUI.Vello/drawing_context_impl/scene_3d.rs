//! 3D scenes on the Vello backend (#608, `docs/porting/media3d.md`, step D3): the scene of a
//! `DrawScene3D` is rasterized by the CPU reference of `ferroui-render3d` at the resolution of the
//! device and painted as an image over its bounds, under the transform, clips and layers of the
//! scene of the context.
//!
//! In the CPU mode this is the defined path (the mode is the reference). In the hybrid and GPU
//! modes it is the last resort until the wgpu executor of step D4 lands: each such scene is
//! counted (`ferroui_render3d::scenes_drawn_on_cpu`) and the first is logged.
//!
//! The image of a scene is kept on the render thread while its snapshot, the size, the scale and
//! the edge mode are unchanged: an unchanged viewport is not rasterized again, and the same image
//! (the same blob) is painted, which the renderers that upload images upload once.

use super::DrawingContextImpl;
use crate::helpers::pixel_format_helper::to_image;
use crate::scene::{VelloSceneBrush, VelloSceneImage, VelloScenePaint};
use crate::vello_extensions::rect_path;
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::EdgeMode;
use ferroui_base::platform::IDrawingContextImplWithScene3D;
use ferroui_base::rendering::scene3d::Scene3DFrame;
use ferroui_base::PixelSize;
use ferroui_render3d::{report_scene_drawn_on_cpu, scene_pixel_size, CpuRenderOptions, Scene3DOutputCache};
use kurbo::Affine;
use peniko::{BlendMode, Extend, Fill, ImageData, ImageQuality};
use std::cell::RefCell;

ferroui_base::ferro_thread_local! {
    /// The images of the scenes drawn recently on this thread.
    static OUTPUTS: RefCell<Scene3DOutputCache<ImageData>> = RefCell::new(Scene3DOutputCache::new());
}

impl IDrawingContextImplWithScene3D for DrawingContextImpl {
    fn draw_scene_3d(&mut self, frame: &Scene3DFrame) -> bool {
        let bounds = frame.bounds;
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return true;
        }
        let transform = self.device_transform();
        let [a, b, c, d, _, _] = transform.as_coeffs();
        let (size, scale) = scene_pixel_size(bounds, (a, b), (c, d));
        let options = CpuRenderOptions { antialiased: self.render_options.edge_mode != EdgeMode::Aliased };
        let image = OUTPUTS.with(|outputs| {
            outputs.borrow_mut().get_or_render(&frame.scene, size, scale, options, |image| {
                let size = PixelSize::new(image.width as i32, image.height as i32);
                Some(to_image(image.rgba, size))
            })
        });
        let Some(image) = image else { return false };
        if self.rendering_mode() != VelloRenderingMode::Cpu {
            report_scene_drawn_on_cpu("Vello (GPU)");
        }

        // The pixels of the image stretched over the bounds, sampled bilinearly.
        let image_transform = Affine::translate((bounds.x, bounds.y))
            * Affine::scale_non_uniform(bounds.width / f64::from(image.width), bounds.height / f64::from(image.height));
        let alpha = ((255.0 * self.current_opacity.clamp(0.0, 1.0)) as u8) as f32 / 255.0;
        let paint = VelloScenePaint {
            brush: VelloSceneBrush::Image(VelloSceneImage {
                image,
                x_extend: Extend::Pad,
                y_extend: Extend::Pad,
                quality: ImageQuality::Medium,
                alpha,
            }),
            transform: image_transform,
        };
        let anti_alias = self.anti_alias();
        self.sink().fill(&rect_path(bounds), Fill::NonZero, transform, &paint, BlendMode::default(), anti_alias);
        true
    }
}
