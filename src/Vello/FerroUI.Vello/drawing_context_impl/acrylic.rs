//! Rectangles filled with an acrylic-like material.
//!
//! The paint of the Skia backend composes three shaders: the tint over the
//! color of the material, and a noise texture, repeated and nearly
//! transparent, over both. A paint of a scene is one brush, so the two
//! layers are drawn one after the other: the tint over the material as one
//! color, then the noise. Composing source-over is associative, so the
//! pixels are those of the composed paint; on an anti-aliased edge each of
//! the two fills is weighted by the coverage on its own.

use super::DrawingContextImpl;
use crate::helpers::image_saving_helper::decode_image;
use crate::helpers::pixel_format_helper::to_image;
use crate::scene::{VelloSceneBrush, VelloSceneImage, VelloScenePaint};
use crate::vello_extensions::rounded_rect_path;
use ferroui_base::media::{AcrylicBackgroundSource, Color, IExperimentalAcrylicMaterial};
use ferroui_base::platform::IDrawingContextWithAcrylicLikeSupport;
use ferroui_base::{PixelSize, RoundedRect};
use kurbo::Affine;
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Compose, Extend, Fill, ImageData, ImageQuality, Mix};

/// The opacity of the noise texture.
const NOISE_OPACITY: f64 = 0.0225;

/// The noise texture of acrylic materials (256 by 256 pixels, the asset of
/// the Skia backend) with its alpha scaled to the opacity of the noise, as
/// the color filter of the Skia backend scales it: truncated to a byte.
fn acrylic_noise() -> Option<ImageData> {
    thread_local! {
        static ACRYLIC_NOISE: std::cell::OnceCell<Option<ImageData>> = const { std::cell::OnceCell::new() };
    }

    static NOISE_ASSET: &[u8] = include_bytes!("../assets/noise_asset_256x256_png.png");

    ACRYLIC_NOISE.with(|noise| {
        noise
            .get_or_init(|| {
                let (mut rgba, width, height) = decode_image(&mut &NOISE_ASSET[..]).ok()?;

                for pixel in rgba.chunks_exact_mut(4) {
                    let alpha = pixel[3] as u32;
                    if alpha == 0 {
                        continue;
                    }
                    let faded = (alpha as f64 * NOISE_OPACITY) as u32;
                    // The colors are premultiplied: they follow the alpha.
                    for channel in &mut pixel[..3] {
                        *channel = ((*channel as u32 * faded + alpha / 2) / alpha) as u8;
                    }
                    pixel[3] = faded as u8;
                }

                Some(to_image(rgba, PixelSize::new(width as i32, height as i32)))
            })
            .clone()
    })
}

/// One color composed source-over another.
fn over(source: Color, destination: Color) -> AlphaColor<Srgb> {
    let (source_alpha, destination_alpha) = (source.a as f32 / 255.0, destination.a as f32 / 255.0);
    let alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    if alpha <= 0.0 {
        return AlphaColor::TRANSPARENT;
    }

    let channel = |source: u8, destination: u8| {
        (source as f32 / 255.0 * source_alpha + destination as f32 / 255.0 * destination_alpha * (1.0 - source_alpha))
            / alpha
    };

    AlphaColor::new([
        channel(source.r, destination.r),
        channel(source.g, destination.g),
        channel(source.b, destination.b),
        alpha,
    ])
}

impl IDrawingContextWithAcrylicLikeSupport for DrawingContextImpl {
    fn draw_rectangle_with_material(&mut self, material: &dyn IExperimentalAcrylicMaterial, rect: RoundedRect) {
        if rect.rect.height <= 0.0 || rect.rect.width <= 0.0 {
            return;
        }

        let path = rounded_rect_path(rect);
        let transform = self.device_transform();

        // The tint over the color of the material. A material that digs
        // through to what is behind the window replaces what is under it.
        let tint = over(material.tint_color(), material.material_color());
        let blend_mode = if material.background_source() == AcrylicBackgroundSource::Digger {
            BlendMode::new(Mix::Normal, Compose::Copy)
        } else {
            BlendMode::default()
        };
        self.sink().fill(&path, Fill::NonZero, transform, &VelloScenePaint::solid(tint), blend_mode, true);

        if let Some(image) = acrylic_noise() {
            let noise = VelloScenePaint {
                brush: VelloSceneBrush::Image(VelloSceneImage {
                    image,
                    x_extend: Extend::Repeat,
                    y_extend: Extend::Repeat,
                    quality: ImageQuality::Low,
                    alpha: 1.0,
                }),
                transform: Affine::IDENTITY,
            };
            self.sink().fill(&path, Fill::NonZero, transform, &noise, BlendMode::default(), true);
        }
    }
}
