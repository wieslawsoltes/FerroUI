//! 3D scenes on the Vello backend (#608, `docs/porting/media3d.md`, steps D3 and D4).
//!
//! - **Hybrid and GPU modes** (a scene of the device that paints with its textures): the scene of
//!   a `DrawScene3D` is drawn on the device by the wgpu executor of `ferroui-render3d` into a
//!   texture of the pool (`gpu/scene3d_passes.rs`), and that texture is painted over the bounds,
//!   under the transform, clips and layers of the scene of the context, as the other textures of
//!   the device are. An unchanged viewport is painted from its kept texture without a 3D pass.
//! - **The CPU mode**, and a scene of the CPU renderer inside a GPU mode: the CPU reference of
//!   `ferroui-render3d` rasterizes the scene at the resolution of the device, and the image is
//!   painted over the bounds. In the CPU mode this is the defined path (the mode is the
//!   reference).
//! - **The last resort.** Where a GPU mode cannot draw on the device (no executor for it, a
//!   failed pipeline or render: reported once by `gpu/scene3d_passes.rs`), the CPU reference draws
//!   the scene, and each such scene is counted (`ferroui_render3d::scenes_drawn_on_cpu`); the first
//!   is logged.
//!
//! The image of the CPU reference is kept on the render thread while its snapshot, the size, the
//! scale and the edge mode are unchanged: an unchanged viewport is not rasterized again, and the
//! same image (the same blob) is painted, which the renderers that upload images upload once.

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

        #[cfg(any(feature = "hybrid", feature = "gpu"))]
        if self.draw_scene_3d_on_device(frame, transform, size, scale, options.antialiased) {
            return true;
        }

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

#[cfg(any(feature = "hybrid", feature = "gpu"))]
impl DrawingContextImpl {
    /// Draws the scene of `frame` on the device of the scene of the context, when the scene is
    /// one of the device that paints with its textures (`gpu/scene3d_passes.rs`): the texture of
    /// the scene, `size` device pixels drawn at its top left, painted over the bounds of the
    /// frame. `false` when the scene is not drawn on a device (the CPU mode, a scene of the CPU
    /// renderer inside a layer) or when the device cannot draw 3D scenes: the CPU reference
    /// draws it then.
    fn draw_scene_3d_on_device(
        &mut self,
        frame: &Scene3DFrame,
        transform: Affine,
        size: PixelSize,
        scale: f64,
        antialiased: bool,
    ) -> bool {
        use crate::scene::VelloSceneTexture;
        let device = {
            let scene = self.scene();
            if scene.rendering_mode() == VelloRenderingMode::Cpu || !scene.capabilities().device_textures {
                return false;
            }
            match scene.device() {
                Some(device) => device.clone(),
                None => return false,
            }
        };
        let bounds = frame.bounds;
        // The device pixel of the top left of the viewport names the viewport whose snapshot
        // changed (its kept output and uniform buffers are drawn into again).
        let corner = transform * kurbo::Point::new(bounds.x, bounds.y);
        let origin = (corner.x.round() as i32, corner.y.round() as i32);
        let Some(drawn) = crate::gpu::scene3d_passes::scene_texture(&device, &frame.scene, size, scale, antialiased, origin)
        else {
            return false;
        };

        // The pixels of the image (the top left of the texture) stretched over the bounds,
        // sampled bilinearly, as the image of the CPU reference is.
        let (width, height) = drawn.size;
        let image_transform = Affine::translate((bounds.x, bounds.y))
            * Affine::scale_non_uniform(bounds.width / f64::from(width), bounds.height / f64::from(height));
        let alpha = ((255.0 * self.current_opacity.clamp(0.0, 1.0)) as u8) as f32 / 255.0;
        let paint = VelloScenePaint {
            brush: VelloSceneBrush::Texture(VelloSceneTexture {
                texture: drawn.texture,
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
