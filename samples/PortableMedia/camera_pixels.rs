//! Counted diagnostics of real GPU output and the normal composition render walk.
use super::*;
pub use crate::desktop::native_pixels::{composition, presented};
use ferroui_base::{Point, Visual};
use ferroui_media_opengl::NativeGlFrameProducer;
use image::RgbaImage;

pub fn color(pattern: u64, u: f64, v: f64) -> [u8; 3] {
    const COLORS: [[u8; 3]; 4] = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 255]];
    if u < 0.025 || u >= 0.975 || v < 8. / 240. || v >= 232. / 240. {
        return [255, 0, 255];
    }
    if (0.375..0.425).contains(&v) {
        return [255, 255, 0];
    }
    if (0.125..0.175).contains(&u) {
        return [0, 255, 255];
    }
    COLORS[(((v >= 0.5) as usize * 2 + (u >= 0.5) as usize) + pattern as usize) % 4]
}
/// Independent projection calculation, including the perspective divide.
pub fn projected(u: f64, v: f64) -> (f64, f64) {
    let w = 1. + 0.35 * u + 0.15 * v;
    (
        (1. + (1.4 * u - 0.25 * v - 0.55) / w) / 2.,
        (1. + (-1.35 * v + 0.7) / w) / 2.,
    )
}
fn close(actual: &[u8], expected: &[u8], tolerance: u8) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(a, e)| a.abs_diff(*e) <= tolerance)
}
pub fn check_projection(pixels: &RgbaImage, pattern: u64) -> Result<(), String> {
    for (u, v) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        let (x, y) = projected(u, v);
        let source = color(pattern, u, v);
        let expected = [
            (source[0] as f64 * 0.85).round() as u8,
            (source[1] as f64 * 0.85).round() as u8,
            (source[2] as f64 * 0.85).round() as u8,
            217,
        ];
        let actual = pixels.get_pixel(
            (x * pixels.width() as f64) as u32,
            (y * pixels.height() as f64) as u32,
        );
        if !close(&actual.0, &expected, 4) {
            return Err(format!(
                "Projection ({u},{v}): {actual:?}, expected {expected:?}"
            ));
        }
    }
    for (x, y) in [
        (0, 0),
        (pixels.width() - 1, 0),
        (0, pixels.height() - 1),
        (pixels.width() - 1, pixels.height() - 1),
    ] {
        if pixels.get_pixel(x, y).0 != [0, 0, 0, 0] {
            return Err(format!(
                "Projection coverage at ({x},{y}) is not transparent"
            ));
        }
    }
    Ok(())
}
pub async fn projection(
    producer: &NativeGlFrameProducer,
    frame: FrameLease,
    pattern: u64,
) -> Result<(), String> {
    let image = crate::desktop::native_pixels::read_texture(producer, frame).await?;
    check_projection(&image, pattern)?;
    if check_projection(&image, pattern + 1).is_ok() {
        return Err("Wrong-palette projection negative control passed".into());
    }
    println!("CAMERA_PROJECTED_PIXELS_PASS pattern={pattern} rgba=true transparent=true negative_control=rejected");
    Ok(())
}
pub fn check_view(
    pixels: &RgbaImage,
    window: &Ref<Window>,
    view: &Visual,
    pattern: u64,
) -> Result<(), String> {
    check_stretch(pixels, window, view, pattern, Stretch::Fill)
}
pub fn check_cleared(
    pixels: &RgbaImage,
    window: &Ref<Window>,
    view: &Visual,
) -> Result<(), String> {
    let size = view.bounds().size();
    for u in [0.25, 0.75] {
        for v in [0.25, 0.75] {
            let p = view
                .translate_point(Point::new(size.width * u, size.height * v), window)
                .ok_or("Detached cleared view")?;
            let actual = pixels.get_pixel(
                (p.x * window.render_scaling()).round() as u32,
                (p.y * window.render_scaling()).round() as u32,
            );
            if !close(&actual.0[..3], &[32, 48, 64], 8) {
                return Err(format!("Stopped preview retained pixels {actual:?}"));
            }
        }
    }
    Ok(())
}
pub fn check_stretch(
    pixels: &RgbaImage,
    window: &Ref<Window>,
    view: &Visual,
    pattern: u64,
    stretch: Stretch,
) -> Result<(), String> {
    let size = view.bounds().size();
    let scale = window.render_scaling();
    let (sx, sy) = (size.width / 320., size.height / 240.);
    let (sx, sy) = match stretch {
        Stretch::Uniform => (sx.min(sy), sx.min(sy)),
        Stretch::UniformToFill => (sx.max(sy), sx.max(sy)),
        _ => (sx, sy),
    };
    let (w, h) = (320. * sx, 240. * sy);
    let (left, top) = ((size.width - w) / 2., (size.height - h) / 2.);
    // Dense asymmetric probes include the border/stripes, letterbox and crop.
    for v in [0.01, 0.10, 0.20, 0.25, 0.40, 0.60, 0.75, 0.90, 0.99] {
        for u in [0.01, 0.05, 0.10, 0.15, 0.20, 0.25, 0.75, 0.90, 0.99] {
            let (source_u, source_v) = ((u * size.width - left) / w, (v * size.height - top) / h);
            // Avoid sampling the bilinear transition at a known colour boundary.
            if [0.025, 0.125, 0.175, 0.5, 0.975]
                .into_iter()
                .any(|x| (source_u - x).abs() * 320. < 2.)
                || [8. / 240., 0.375, 0.425, 0.5, 232. / 240.]
                    .into_iter()
                    .any(|y| (source_v - y).abs() * 240. < 2.)
            {
                continue;
            }
            let p = view
                .translate_point(Point::new(size.width * u, size.height * v), window)
                .ok_or("Detached diagnostic view")?;
            let (x, y) = ((p.x * scale).round() as u32, (p.y * scale).round() as u32);
            if x >= pixels.width() || y >= pixels.height() {
                return Err("Diagnostic view outside snapshot".into());
            }
            let expected = if (0. ..1.).contains(&source_u) && (0. ..1.).contains(&source_v) {
                color(pattern, source_u, source_v)
            } else {
                [32, 48, 64]
            };
            let actual = pixels.get_pixel(x, y);
            if !close(&actual.0[..3], &expected, 8) {
                return Err(format!(
                "Composed view {stretch:?} ({x},{y}) source=({source_u},{source_v}): {actual:?}, expected {expected:?}"
            ));
            }
        }
    }
    Ok(())
}
pub fn check_composed_projection(
    pixels: &RgbaImage,
    window: &Ref<Window>,
    host: &Visual,
    pattern: u64,
) -> Result<(), String> {
    let size = host.bounds().size();
    let scale = window.render_scaling();
    let background = [32., 48., 64.];
    for (u, v) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
        let (x, y) = projected(u, v);
        let source = color(pattern, u, v);
        let p = host
            .translate_point(Point::new(size.width * x, size.height * (1. - y)), window)
            .ok_or("Detached renderer")?;
        let actual = pixels.get_pixel((p.x * scale).round() as u32, (p.y * scale).round() as u32);
        let expected = std::array::from_fn::<_, 3, _>(|c| {
            (source[c] as f64 * 0.85 + background[c] * 0.15).round() as u8
        });
        if !close(&actual.0[..3], &expected, 8) {
            return Err(format!(
                "Composed projection {actual:?}, expected {expected:?}"
            ));
        }
    }
    Ok(())
}
