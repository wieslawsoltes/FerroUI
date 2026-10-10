//! Conversions between the framework's primitives and Skia's.

use ferroui_base::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use ferroui_base::media::{Color, GradientSpreadMethod, PenLineCap, PenLineJoin};
use ferroui_base::platform::{AlphaFormat, LtrbPixelRect, LtrbRect, PixelFormat};
use ferroui_base::{Matrix, PixelRect, Point, Rect, RoundedRect, Vector};
use skia_safe as sk;

/// Sampling options for an interpolation mode, assuming upscaling.
pub fn to_sk_sampling_options(interpolation_mode: BitmapInterpolationMode) -> sk::SamplingOptions {
    to_sk_sampling_options_scaled(interpolation_mode, true)
}

/// Sampling options for an interpolation mode. High quality uses a cubic
/// resampler when upscaling and mipmaps when downscaling.
pub fn to_sk_sampling_options_scaled(
    interpolation_mode: BitmapInterpolationMode,
    is_upscaling: bool,
) -> sk::SamplingOptions {
    match interpolation_mode {
        BitmapInterpolationMode::None => sk::SamplingOptions::new(sk::FilterMode::Nearest, sk::MipmapMode::None),
        BitmapInterpolationMode::Unspecified | BitmapInterpolationMode::LowQuality => {
            sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::None)
        }
        BitmapInterpolationMode::MediumQuality => {
            sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::Linear)
        }
        BitmapInterpolationMode::HighQuality => {
            if is_upscaling {
                sk::SamplingOptions::from(sk::CubicResampler::mitchell())
            } else {
                sk::SamplingOptions::new(sk::FilterMode::Linear, sk::MipmapMode::Linear)
            }
        }
    }
}

pub fn to_sk_blend_mode(blending_mode: BitmapBlendingMode) -> sk::BlendMode {
    match blending_mode {
        BitmapBlendingMode::Unspecified => sk::BlendMode::SrcOver,
        BitmapBlendingMode::SourceOver => sk::BlendMode::SrcOver,
        BitmapBlendingMode::Source => sk::BlendMode::Src,
        BitmapBlendingMode::SourceIn => sk::BlendMode::SrcIn,
        BitmapBlendingMode::SourceOut => sk::BlendMode::SrcOut,
        BitmapBlendingMode::SourceAtop => sk::BlendMode::SrcATop,
        BitmapBlendingMode::Destination => sk::BlendMode::Dst,
        BitmapBlendingMode::DestinationIn => sk::BlendMode::DstIn,
        BitmapBlendingMode::DestinationOut => sk::BlendMode::DstOut,
        BitmapBlendingMode::DestinationOver => sk::BlendMode::DstOver,
        BitmapBlendingMode::DestinationAtop => sk::BlendMode::DstATop,
        BitmapBlendingMode::Xor => sk::BlendMode::Xor,
        BitmapBlendingMode::Plus => sk::BlendMode::Plus,
        BitmapBlendingMode::Screen => sk::BlendMode::Screen,
        BitmapBlendingMode::Overlay => sk::BlendMode::Overlay,
        BitmapBlendingMode::Darken => sk::BlendMode::Darken,
        BitmapBlendingMode::Lighten => sk::BlendMode::Lighten,
        BitmapBlendingMode::ColorDodge => sk::BlendMode::ColorDodge,
        BitmapBlendingMode::ColorBurn => sk::BlendMode::ColorBurn,
        BitmapBlendingMode::HardLight => sk::BlendMode::HardLight,
        BitmapBlendingMode::SoftLight => sk::BlendMode::SoftLight,
        BitmapBlendingMode::Difference => sk::BlendMode::Difference,
        BitmapBlendingMode::Exclusion => sk::BlendMode::Exclusion,
        BitmapBlendingMode::Multiply => sk::BlendMode::Multiply,
        BitmapBlendingMode::Hue => sk::BlendMode::Hue,
        BitmapBlendingMode::Saturation => sk::BlendMode::Saturation,
        BitmapBlendingMode::Color => sk::BlendMode::Color,
        BitmapBlendingMode::Luminosity => sk::BlendMode::Luminosity,
    }
}

#[inline]
pub fn to_sk_point(p: Point) -> sk::Point {
    sk::Point::new(p.x as f32, p.y as f32)
}

#[inline]
pub fn vector_to_sk_point(p: Vector) -> sk::Point {
    sk::Point::new(p.x as f32, p.y as f32)
}

#[inline]
pub fn to_sk_rect(r: Rect) -> sk::Rect {
    sk::Rect::new(r.x as f32, r.y as f32, r.right() as f32, r.bottom() as f32)
}

#[inline]
pub fn ltrb_to_sk_rect(r: LtrbRect) -> sk::Rect {
    sk::Rect::new(r.left as f32, r.top as f32, r.right as f32, r.bottom as f32)
}

#[inline]
pub fn to_sk_rect_i(r: PixelRect) -> sk::IRect {
    sk::IRect::new(r.x, r.y, r.right(), r.bottom())
}

#[inline]
pub fn ltrb_to_sk_rect_i(r: LtrbPixelRect) -> sk::IRect {
    sk::IRect::new(r.left, r.top, r.right, r.bottom)
}

pub fn to_sk_round_rect(r: RoundedRect) -> sk::RRect {
    let rc = to_sk_rect(r.rect);
    let mut result = sk::RRect::new();
    result.set_rect_radii(
        rc,
        &[
            vector_to_sk_point(r.radii_top_left),
            vector_to_sk_point(r.radii_top_right),
            vector_to_sk_point(r.radii_bottom_right),
            vector_to_sk_point(r.radii_bottom_left),
        ],
    );
    result
}

#[inline]
pub fn to_rect(r: sk::Rect) -> Rect {
    Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64)
}

#[inline]
pub fn to_pixel_rect(r: sk::IRect) -> PixelRect {
    PixelRect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}

#[inline]
pub fn to_ltrb_pixel_rect(r: sk::IRect) -> LtrbPixelRect {
    LtrbPixelRect { left: r.left, top: r.top, right: r.right, bottom: r.bottom }
}

pub fn to_sk_matrix(m: Matrix) -> sk::Matrix {
    sk::Matrix::new_all(
        m.m11 as f32,
        m.m21 as f32,
        m.m31 as f32,
        m.m12 as f32,
        m.m22 as f32,
        m.m32 as f32,
        m.m13 as f32,
        m.m23 as f32,
        m.m33 as f32,
    )
}

/// Converts to a 4x4 matrix directly, avoiding the 3x3 intermediate the
/// canvas would otherwise convert from.
pub fn to_sk_matrix44(m: Matrix) -> sk::M44 {
    sk::M44::col_major(&[
        m.m11 as f32,
        m.m12 as f32,
        0.0,
        m.m13 as f32,
        m.m21 as f32,
        m.m22 as f32,
        0.0,
        m.m23 as f32,
        0.0,
        0.0,
        1.0,
        0.0,
        m.m31 as f32,
        m.m32 as f32,
        0.0,
        m.m33 as f32,
    ])
}

pub fn to_matrix(m: &sk::Matrix) -> Matrix {
    Matrix::new_3x3(
        m.scale_x() as f64,
        m.skew_y() as f64,
        m.persp_x() as f64,
        m.skew_x() as f64,
        m.scale_y() as f64,
        m.persp_y() as f64,
        m.translate_x() as f64,
        m.translate_y() as f64,
        m[sk::matrix::Member::Persp2] as f64,
    )
}

pub fn matrix44_to_matrix(m: &sk::M44) -> Matrix {
    Matrix::new_3x3(
        m.rc(0, 0) as f64,
        m.rc(1, 0) as f64,
        m.rc(3, 0) as f64,
        m.rc(0, 1) as f64,
        m.rc(1, 1) as f64,
        m.rc(3, 1) as f64,
        m.rc(0, 3) as f64,
        m.rc(1, 3) as f64,
        m.rc(3, 3) as f64,
    )
}

#[inline]
pub fn to_sk_color(c: Color) -> sk::Color {
    sk::Color::from_argb(c.a, c.r, c.g, c.b)
}

/// The Skia color type of a pixel format, or `None` when Skia has no
/// equivalent.
pub fn to_sk_color_type(fmt: PixelFormat) -> Option<sk::ColorType> {
    if fmt == PixelFormat::RGB565 {
        return Some(sk::ColorType::RGB565);
    }
    if fmt == PixelFormat::BGRA8888 {
        return Some(sk::ColorType::BGRA8888);
    }
    if fmt == PixelFormat::RGBA8888 {
        return Some(sk::ColorType::RGBA8888);
    }
    if fmt == PixelFormat::RGB32 {
        return Some(sk::ColorType::RGB888x);
    }
    None
}

/// The Skia color type of a pixel format.
///
/// # Panics
/// Panics when the pixel format is unknown to Skia.
pub fn to_sk_color_type_or_panic(fmt: PixelFormat) -> sk::ColorType {
    to_sk_color_type(fmt).unwrap_or_else(|| panic!("Unknown pixel format: {fmt}"))
}

/// The pixel format of a Skia color type, or `None` when there is no
/// equivalent.
pub fn to_pixel_format_opt(color_type: sk::ColorType) -> Option<PixelFormat> {
    match color_type {
        sk::ColorType::RGB565 => Some(PixelFormat::RGB565),
        sk::ColorType::BGRA8888 => Some(PixelFormat::BGRA8888),
        sk::ColorType::RGBA8888 => Some(PixelFormat::RGBA8888),
        sk::ColorType::RGB888x => Some(PixelFormat::RGB32),
        _ => None,
    }
}

/// The pixel format of a Skia color type.
///
/// # Panics
/// Panics for color types other than RGB565, BGRA8888 and RGBA8888.
pub fn to_pixel_format(fmt: sk::ColorType) -> PixelFormat {
    match fmt {
        sk::ColorType::RGB565 => PixelFormat::RGB565,
        sk::ColorType::BGRA8888 => PixelFormat::BGRA8888,
        sk::ColorType::RGBA8888 => PixelFormat::RGBA8888,
        _ => panic!("Unknown pixel format: {fmt:?}"),
    }
}

pub fn to_sk_alpha_type(fmt: AlphaFormat) -> sk::AlphaType {
    match fmt {
        AlphaFormat::Premul => sk::AlphaType::Premul,
        AlphaFormat::Unpremul => sk::AlphaType::Unpremul,
        AlphaFormat::Opaque => sk::AlphaType::Opaque,
    }
}

/// The alpha format of a Skia alpha type.
///
/// # Panics
/// Panics for the unknown alpha type.
pub fn to_alpha_format(fmt: sk::AlphaType) -> AlphaFormat {
    match fmt {
        sk::AlphaType::Premul => AlphaFormat::Premul,
        sk::AlphaType::Unpremul => AlphaFormat::Unpremul,
        sk::AlphaType::Opaque => AlphaFormat::Opaque,
        _ => panic!("Unknown alpha format: {fmt:?}"),
    }
}

pub fn to_sk_shader_tile_mode(m: GradientSpreadMethod) -> sk::TileMode {
    match m {
        GradientSpreadMethod::Pad => sk::TileMode::Clamp,
        GradientSpreadMethod::Reflect => sk::TileMode::Mirror,
        GradientSpreadMethod::Repeat => sk::TileMode::Repeat,
    }
}

pub fn to_sk_stroke_cap(cap: PenLineCap) -> sk::PaintCap {
    match cap {
        PenLineCap::Round => sk::PaintCap::Round,
        PenLineCap::Square => sk::PaintCap::Square,
        _ => sk::PaintCap::Butt,
    }
}

pub fn to_sk_stroke_join(join: PenLineJoin) -> sk::PaintJoin {
    match join {
        PenLineJoin::Bevel => sk::PaintJoin::Bevel,
        PenLineJoin::Round => sk::PaintJoin::Round,
        _ => sk::PaintJoin::Miter,
    }
}

pub fn to_sk_text_align(a: ferroui_base::media::TextAlignment) -> sk::utils::text_utils::Align {
    use ferroui_base::media::TextAlignment;
    match a {
        TextAlignment::Center => sk::utils::text_utils::Align::Center,
        TextAlignment::Right => sk::utils::text_utils::Align::Right,
        _ => sk::utils::text_utils::Align::Left,
    }
}

pub fn to_text_alignment(a: sk::utils::text_utils::Align) -> ferroui_base::media::TextAlignment {
    use ferroui_base::media::TextAlignment;
    match a {
        sk::utils::text_utils::Align::Center => TextAlignment::Center,
        sk::utils::text_utils::Align::Right => TextAlignment::Right,
        _ => TextAlignment::Left,
    }
}

pub fn slant_to_font_style(slant: sk::font_style::Slant) -> ferroui_base::media::FontStyle {
    use ferroui_base::media::FontStyle;
    match slant {
        sk::font_style::Slant::Upright => FontStyle::Normal,
        sk::font_style::Slant::Italic => FontStyle::Italic,
        sk::font_style::Slant::Oblique => FontStyle::Oblique,
    }
}

pub fn font_style_to_slant(style: ferroui_base::media::FontStyle) -> sk::font_style::Slant {
    use ferroui_base::media::FontStyle;
    match style {
        FontStyle::Normal => sk::font_style::Slant::Upright,
        FontStyle::Italic => sk::font_style::Slant::Italic,
        FontStyle::Oblique => sk::font_style::Slant::Oblique,
    }
}

/// Upstream's `SKPath.TightBounds`.
///
/// A path that has only lines has the bounds of all its points as its tight
/// bounds (`SkPath::computeTightBounds`: "if we're only lines, then our
/// (quick) bounds is also tight"), and in the Skia upstream links those are
/// the bounds of every point of the path, the points of its moves included,
/// also of a move no segment follows. The Skia this backend links leaves
/// the trailing moves of a path out of its bounds, so the bounds of such a
/// path are computed here; upstream's render tests have paths that end in
/// moves to give a shape its extent (`M 10,190 L 190,10 M0,0M200,200`).
pub fn tight_bounds(path: &sk::Path) -> sk::Rect {
    if path.count_verbs() != 0 && path.segment_masks() == sk::PathSegmentMask::LINE {
        let points = path.points();
        if let Some(first) = points.first() {
            let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
            let mut finite = true;
            for point in points {
                finite &= point.x.is_finite() && point.y.is_finite();
                left = left.min(point.x);
                top = top.min(point.y);
                right = right.max(point.x);
                bottom = bottom.max(point.y);
            }
            // The bounds of a path with a point that is not finite are empty.
            return if finite { sk::Rect::new(left, top, right, bottom) } else { sk::Rect::new_empty() };
        }
    }

    path.compute_tight_bounds()
}
