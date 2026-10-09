//! Smaller copies of an image for drawing it reduced.
//!
//! An image that is drawn much smaller than it is has to be averaged, not
//! sampled: Skia does that with the levels of a mipmap, each half as large
//! as the one before, and blends what it samples from the two levels
//! nearest to the size the image is drawn at (`SkMipmap`,
//! `SkMipmapAccessor`). The renderers of the Vello project sample an image
//! as it is. The interpolation modes of the contract that ask Skia for
//! mipmaps (medium quality, and high quality when an image is reduced) get
//! them here: the levels are built the way Skia builds them, the two levels
//! Skia would sample are chosen the way Skia chooses them, and the renderer
//! is given both ([`fill_with_levels`]) or, where one image has to do, the
//! smaller blended into the larger ([`MipLevels::blended`]).

use crate::scene::{IVelloSceneSink, VelloSceneBrush, VelloSceneImage, VelloScenePaint};
use kurbo::{Affine, BezPath};
use peniko::{BlendMode, Compose, Extend, Fill, ImageQuality, Mix};
use peniko::{Blob, ImageAlphaType, ImageData, ImageFormat};
use std::sync::Arc;

/// Premultiplied RGBA pixels of a level.
struct Level {
    width: usize,
    height: usize,
    rgba: Vec<u8>,
}

/// An image to sample in place of another that is drawn reduced.
pub struct Prefiltered {
    /// The image: premultiplied RGBA.
    pub image: ImageData,
    /// The pixels of this image per pixel of the original, along x and y.
    pub scale_x: f64,
    pub scale_y: f64,
}

/// The two levels of the mipmap of an image that Skia samples when the
/// image is drawn reduced, and how much of each it takes.
pub struct MipLevels {
    /// The level nearest above the size the image is drawn at: the image
    /// itself or a smaller copy.
    pub upper: Prefiltered,
    /// The level below it, when it has a share in the result.
    pub lower: Option<Prefiltered>,
    /// The share of the lower level.
    pub lower_weight: f64,
}

/// The level of the mipmap of an image that is drawn with a transform, as
/// `SkMipmap::ComputeLevel` computes it: the number of halvings to the
/// smaller of the scales of the two axes, less a half ("to emulate the
/// sharpen mipmap option of a GPU"). `None` when the image is not reduced
/// enough to use a level.
fn compute_level(transform: Affine) -> Option<f64> {
    let [a, b, c, d, _, _] = transform.inverse().as_coeffs();
    let (inverse_x, inverse_y) = (a.hypot(b), c.hypot(d));
    if !(inverse_x.is_finite() && inverse_y.is_finite() && inverse_x > 0.0 && inverse_y > 0.0) {
        return None;
    }

    let scale = (1.0 / inverse_x).min(1.0 / inverse_y);
    if !(scale < 1.0 && scale > 0.0) {
        return None;
    }

    let level = (-scale.log2() - 0.5).max(0.0);
    (level.is_finite() && level > 0.0).then_some(level)
}

/// The next level: half as wide and as high, at least a pixel. Two pixels
/// are averaged along an axis of even length and three, the middle one
/// twice, along an axis of odd length: the filters of `SkMipmap`.
fn downsample(source: &Level) -> Level {
    let (width, height) = ((source.width / 2).max(1), (source.height / 2).max(1));

    // The taps along an axis: the offsets from twice the index, with their
    // weights, which sum to four.
    let taps = |length: usize| -> &'static [(usize, u32)] {
        if length == 1 {
            &[(0, 4)]
        } else if length % 2 == 0 {
            &[(0, 2), (1, 2)]
        } else {
            &[(0, 1), (1, 2), (2, 1)]
        }
    };
    let (taps_x, taps_y) = (taps(source.width), taps(source.height));

    let mut rgba = vec![0u8; width * height * 4];
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0u32; 4];
            for (offset_y, weight_y) in taps_y {
                let row = (2 * y + offset_y).min(source.height - 1) * source.width;
                for (offset_x, weight_x) in taps_x {
                    let index = (row + (2 * x + offset_x).min(source.width - 1)) * 4;
                    let weight = weight_x * weight_y;
                    for (channel, value) in sum.iter_mut().zip(&source.rgba[index..index + 4]) {
                        *channel += *value as u32 * weight;
                    }
                }
            }

            let index = (y * width + x) * 4;
            for (target, channel) in rgba[index..index + 4].iter_mut().zip(sum) {
                // The weights of a pixel sum to sixteen. The quotient is cut
                // off, not rounded, as Skia shifts the sum: rounded levels
                // are lighter than Skia's by half a step each.
                *target = (channel / 16) as u8;
            }
        }
    }

    Level { width, height, rgba }
}

/// The pixel of a level at a position between its pixels, the nearest four
/// weighted by their distance; beyond the edge the pixels of the edge.
fn sample_bilinear(level: &Level, x: f64, y: f64) -> [f64; 4] {
    let (x, y) = (x.clamp(0.0, (level.width - 1) as f64), y.clamp(0.0, (level.height - 1) as f64));
    let (x0, y0) = (x.floor() as usize, y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(level.width - 1), (y0 + 1).min(level.height - 1));
    let (fraction_x, fraction_y) = (x - x0 as f64, y - y0 as f64);

    let pixel = |x: usize, y: usize| &level.rgba[(y * level.width + x) * 4..][..4];
    let (top_left, top_right, bottom_left, bottom_right) = (pixel(x0, y0), pixel(x1, y0), pixel(x0, y1), pixel(x1, y1));

    let mut result = [0.0; 4];
    for (channel, value) in result.iter_mut().enumerate() {
        let top = top_left[channel] as f64 * (1.0 - fraction_x) + top_right[channel] as f64 * fraction_x;
        let bottom = bottom_left[channel] as f64 * (1.0 - fraction_x) + bottom_right[channel] as f64 * fraction_x;
        *value = top * (1.0 - fraction_y) + bottom * fraction_y;
    }
    result
}

/// A level with the next smaller one blended into it by `lower_weight`.
fn blend(upper: &Level, lower: &Level, lower_weight: f64) -> Level {
    let (scale_x, scale_y) =
        (lower.width as f64 / upper.width as f64, lower.height as f64 / upper.height as f64);

    let mut rgba = vec![0u8; upper.rgba.len()];
    for y in 0..upper.height {
        for x in 0..upper.width {
            // The middle of the pixel in the pixels of the smaller level.
            let below =
                sample_bilinear(lower, (x as f64 + 0.5) * scale_x - 0.5, (y as f64 + 0.5) * scale_y - 0.5);
            let index = (y * upper.width + x) * 4;
            for channel in 0..4 {
                let above = upper.rgba[index + channel] as f64;
                rgba[index + channel] = (above + (below[channel] - above) * lower_weight).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    Level { width: upper.width, height: upper.height, rgba }
}

fn to_level(image: &ImageData) -> Option<Level> {
    if image.format != ImageFormat::Rgba8
        || image.alpha_type != ImageAlphaType::AlphaPremultiplied
        || image.width == 0
        || image.height == 0
    {
        return None;
    }
    let (width, height) = (image.width as usize, image.height as usize);
    let data = image.data.data();
    if data.len() < width * height * 4 {
        return None;
    }

    Some(Level { width, height, rgba: data[..width * height * 4].to_vec() })
}

fn to_prefiltered(level: Level, original: &ImageData) -> Prefiltered {
    Prefiltered {
        scale_x: level.width as f64 / original.width as f64,
        scale_y: level.height as f64 / original.height as f64,
        image: ImageData {
            data: Blob::new(Arc::new(level.rgba)),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::AlphaPremultiplied,
            width: level.width as u32,
            height: level.height as u32,
        },
    }
}

/// The levels to sample, with bilinear interpolation, when `image` is
/// drawn with mipmaps under `transform`, which maps the pixels of the image
/// to the pixels of the target. `None` when the image is not reduced enough
/// for a level, or is not one of premultiplied RGBA pixels: it is sampled
/// as it is.
///
/// The levels are made for the draw and dropped with it; keeping them with
/// the bitmap is left open (design document).
pub fn levels(image: &ImageData, transform: Affine) -> Option<MipLevels> {
    let base = to_level(image)?;

    let level = compute_level(transform)?;
    let level_number = level.floor() as usize;
    let lower_weight = level - level_number as f64;

    // The levels below the image, down to the one the lower weight needs:
    // there are as many as it takes to bring the longer side to a pixel.
    let level_count = (usize::BITS - base.width.max(base.height).leading_zeros()) as usize - 1;
    let wanted = (level_number + usize::from(lower_weight > 0.0)).min(level_count);
    if wanted == 0 {
        return None;
    }

    let mut smaller = Vec::with_capacity(wanted);
    let mut current = downsample(&base);
    for _ in 1..wanted {
        let next = downsample(&current);
        smaller.push(current);
        current = next;
    }
    smaller.push(current);

    // The lower level is the last one that was built, unless the image is
    // drawn smaller than its smallest level: that one is then the upper
    // level and has no lower one.
    let lower = if lower_weight > 0.0 && smaller.len() > level_number { smaller.pop() } else { None };
    let upper = match smaller.pop() {
        Some(upper) => to_prefiltered(upper, image),
        // The image itself, with its own pixels: nothing is copied.
        None => Prefiltered { image: image.clone(), scale_x: 1.0, scale_y: 1.0 },
    };

    Some(MipLevels { upper, lower: lower.map(|lower| to_prefiltered(lower, image)), lower_weight })
}

impl MipLevels {
    /// One image in place of the two levels: the lower level, enlarged,
    /// blended into the upper one. Sampling it is not quite sampling both
    /// (the lower level is interpolated twice), which shows where the
    /// lower level has detail of a pixel.
    pub fn blended(&self) -> Option<Prefiltered> {
        let Some(lower) = &self.lower else {
            return Some(Prefiltered {
                image: self.upper.image.clone(),
                scale_x: self.upper.scale_x,
                scale_y: self.upper.scale_y,
            });
        };

        let blended = blend(&to_level(&self.upper.image)?, &to_level(&lower.image)?, self.lower_weight);
        Some(Prefiltered {
            scale_x: self.upper.scale_x,
            scale_y: self.upper.scale_y,
            image: ImageData {
                data: Blob::new(Arc::new(blended.rgba)),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::AlphaPremultiplied,
                width: blended.width as u32,
                height: blended.height as u32,
            },
        })
    }
}

/// The one image to sample, with bilinear interpolation, when `image` is
/// drawn with mipmaps under `transform` ([`levels`], [`MipLevels::blended`]).
pub fn prefilter(image: &ImageData, transform: Affine) -> Option<Prefiltered> {
    levels(image, transform)?.blended()
}

/// Fills a path with an image that is sampled from the two levels of its
/// mipmap, as Skia samples it: each level with bilinear interpolation, the
/// two results weighted. `image_transform` maps the pixels of the image to
/// the space of the path, `transform` that space to the pixels of the
/// target, and `alpha` is a factor of the alpha of every pixel.
///
/// The two levels are added up in a layer of their own, each with its
/// share as its alpha, and the layer is composed source-over.
pub fn fill_with_levels(
    sink: &mut dyn IVelloSceneSink,
    path: &BezPath,
    transform: Affine,
    image_transform: Affine,
    levels: &MipLevels,
    alpha: f32,
    anti_alias: bool,
) {
    let paint = |level: &Prefiltered, share: f64| VelloScenePaint {
        brush: VelloSceneBrush::Image(VelloSceneImage {
            image: level.image.clone(),
            x_extend: Extend::Pad,
            y_extend: Extend::Pad,
            quality: ImageQuality::Medium,
            alpha: alpha * share as f32,
        }),
        transform: image_transform * Affine::scale_non_uniform(1.0 / level.scale_x, 1.0 / level.scale_y),
    };

    let Some(lower) = &levels.lower else {
        sink.fill(path, Fill::NonZero, transform, &paint(&levels.upper, 1.0), BlendMode::default(), anti_alias);
        return;
    };

    sink.push_layer(BlendMode::default(), 1.0);
    sink.fill(
        path,
        Fill::NonZero,
        transform,
        &paint(&levels.upper, 1.0 - levels.lower_weight),
        BlendMode::default(),
        anti_alias,
    );
    sink.fill(
        path,
        Fill::NonZero,
        transform,
        &paint(lower, levels.lower_weight),
        BlendMode::new(Mix::Normal, Compose::Plus),
        anti_alias,
    );
    sink.pop_layer();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(width: usize, height: usize, value: impl Fn(usize, usize) -> u8) -> Level {
        let mut rgba = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let v = value(x, y);
                rgba.extend_from_slice(&[v, v, v, 255]);
            }
        }
        Level { width, height, rgba }
    }

    #[test]
    fn a_level_is_half_the_one_before() {
        // An even axis: pairs are averaged.
        let half = downsample(&level(4, 2, |x, _| [0, 100, 200, 40][x]));
        assert_eq!((2, 1), (half.width, half.height));
        assert_eq!([50, 120], [half.rgba[0], half.rgba[4]]);

        // An odd axis: three pixels, the middle one twice.
        let half = downsample(&level(5, 1, |x, _| [0, 100, 200, 40, 80][x]));
        assert_eq!((2, 1), (half.width, half.height));
        assert_eq!([100, 90], [half.rgba[0], half.rgba[4]]);

        // A level is at least a pixel.
        let half = downsample(&level(1, 1, |_, _| 77));
        assert_eq!((1, 1, 77), (half.width, half.height, half.rgba[0]));
        let half = downsample(&level(3, 1, |x, _| [10, 20, 30][x]));
        assert_eq!((1, 1, 20), (half.width, half.height, half.rgba[0]));
    }

    #[test]
    fn the_level_follows_the_scale_as_in_skia() {
        // Not reduced, and reduced by less than the bias of a half level.
        assert_eq!(None, compute_level(Affine::scale(1.0)));
        assert_eq!(None, compute_level(Affine::scale(2.0)));
        assert_eq!(None, compute_level(Affine::scale(0.75)));
        // Half the size: half a level. A quarter: one and a half.
        assert!((compute_level(Affine::scale(0.5)).unwrap() - 0.5).abs() < 1e-9);
        assert!((compute_level(Affine::scale(0.25)).unwrap() - 1.5).abs() < 1e-9);
        // The smaller of the two scales counts, under a rotation too.
        assert!((compute_level(Affine::scale_non_uniform(1.0, 0.25)).unwrap() - 1.5).abs() < 1e-9);
        assert!((compute_level(Affine::rotate(0.7) * Affine::scale(0.5)).unwrap() - 0.5).abs() < 1e-9);
    }

    fn image(width: usize, height: usize, value: impl Fn(usize, usize) -> u8) -> ImageData {
        let level = level(width, height, value);
        ImageData {
            data: Blob::new(Arc::new(level.rgba)),
            format: ImageFormat::Rgba8,
            alpha_type: ImageAlphaType::AlphaPremultiplied,
            width: width as u32,
            height: height as u32,
        }
    }

    #[test]
    fn a_reduced_image_is_the_blend_of_two_levels() {
        // A checkerboard of single pixels: every level below it is grey.
        let checker = image(16, 16, |x, y| if (x + y) % 2 == 0 { 255 } else { 0 });

        // Not reduced: sampled as it is.
        assert!(prefilter(&checker, Affine::IDENTITY).is_none());

        // At half the size the level is a half: the image itself and its
        // first level, half of each.
        let half = prefilter(&checker, Affine::scale(0.5)).expect("a prefiltered image");
        assert_eq!((16, 16, 1.0, 1.0), (half.image.width, half.image.height, half.scale_x, half.scale_y));
        let pixels = half.image.data.data();
        assert!(pixels.chunks_exact(4).all(|p| p[0] == 191 || p[0] == 64 || p[0] == 192 || p[0] == 63), "{:?}", &pixels[..8]);

        // At a quarter: the first level and the second, both grey.
        let quarter = prefilter(&checker, Affine::scale(0.25)).expect("a prefiltered image");
        assert_eq!((8, 8, 0.5, 0.5), (quarter.image.width, quarter.image.height, quarter.scale_x, quarter.scale_y));
        assert!(quarter.image.data.data().chunks_exact(4).all(|p| (p[0] as i32 - 127).abs() <= 1 && p[3] == 255));

        // Far below the smallest level: the last one, a pixel.
        let tiny = prefilter(&checker, Affine::scale(0.001)).expect("a prefiltered image");
        assert_eq!((1, 1), (tiny.image.width, tiny.image.height));
    }
}
