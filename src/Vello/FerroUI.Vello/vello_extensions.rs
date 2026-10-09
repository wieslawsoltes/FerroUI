//! Conversions between the types of the base library and those of kurbo and
//! peniko: the counterpart of the Skia backend's extensions.

use ferroui_base::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use ferroui_base::media::{Color, FillRule, GradientSpreadMethod, PenLineCap, PenLineJoin};
use ferroui_base::{Matrix, Point, Rect, RoundedRect, Vector};
use kurbo::{Affine, BezPath, PathEl, Shape};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Compose, Extend, Fill, ImageQuality, Mix};

/// The tolerance the paths of shapes and strokes are built with, in the
/// units of the shape.
pub const PATH_TOLERANCE: f64 = 1e-3;

/// Converts a point.
pub fn to_kurbo_point(p: Point) -> kurbo::Point {
    kurbo::Point::new(p.x, p.y)
}

/// Converts a point back.
pub fn to_point(p: kurbo::Point) -> Point {
    Point::new(p.x, p.y)
}

/// Converts a rectangle.
pub fn to_kurbo_rect(r: Rect) -> kurbo::Rect {
    kurbo::Rect::new(r.x, r.y, r.x + r.width, r.y + r.height)
}

/// Converts a rectangle back.
pub fn to_rect(r: kurbo::Rect) -> Rect {
    Rect::new(r.x0, r.y0, r.x1 - r.x0, r.y1 - r.y0)
}

/// Converts the affine part of a matrix. The base library multiplies row
/// vectors (`x' = x * m11 + y * m21 + m31`), kurbo column vectors.
///
/// The perspective column of the matrix is dropped: kurbo and the renderers
/// of the Vello project draw affine transforms only (design document, gaps).
pub fn to_affine(m: Matrix) -> Affine {
    Affine::new([m.m11, m.m12, m.m21, m.m22, m.m31, m.m32])
}

/// Converts a color (not premultiplied, sRGB).
pub fn to_color(c: Color) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(c.r, c.g, c.b, c.a)
}

/// Converts a color with its alpha multiplied by `opacity`. The alpha is
/// truncated to a byte, as the Skia backend does.
pub fn to_color_with_opacity(c: Color, opacity: f64) -> AlphaColor<Srgb> {
    AlphaColor::from_rgba8(c.r, c.g, c.b, (c.a as f64 * opacity) as u8)
}

/// Converts a fill rule.
pub fn to_fill(rule: FillRule) -> Fill {
    if rule == FillRule::EvenOdd {
        Fill::EvenOdd
    } else {
        Fill::NonZero
    }
}

/// Converts a gradient spread method.
pub fn to_extend(m: GradientSpreadMethod) -> Extend {
    match m {
        GradientSpreadMethod::Reflect => Extend::Reflect,
        GradientSpreadMethod::Repeat => Extend::Repeat,
        _ => Extend::Pad,
    }
}

/// Converts a line cap.
pub fn to_cap(cap: PenLineCap) -> kurbo::Cap {
    match cap {
        PenLineCap::Round => kurbo::Cap::Round,
        PenLineCap::Square => kurbo::Cap::Square,
        _ => kurbo::Cap::Butt,
    }
}

/// Converts a line join.
pub fn to_join(join: PenLineJoin) -> kurbo::Join {
    match join {
        PenLineJoin::Round => kurbo::Join::Round,
        PenLineJoin::Bevel => kurbo::Join::Bevel,
        _ => kurbo::Join::Miter,
    }
}

/// Converts an interpolation mode to the sampling quality of an image:
/// nearest neighbour, bilinear or bicubic.
pub fn to_image_quality(interpolation_mode: BitmapInterpolationMode) -> ImageQuality {
    match interpolation_mode {
        BitmapInterpolationMode::None => ImageQuality::Low,
        BitmapInterpolationMode::HighQuality => ImageQuality::High,
        _ => ImageQuality::Medium,
    }
}

/// How an image is sampled in an interpolation mode: the filter, and
/// whether the image is averaged first when it is drawn reduced
/// ([`mipmap_helper`](crate::helpers::mipmap_helper)). The modes are those
/// of the Skia backend: the nearest pixel, bilinear, bilinear with mipmaps
/// for medium quality, and for high quality the bicubic filter of Mitchell
/// when the image is enlarged and bilinear with mipmaps when it is not.
pub fn to_sampling(interpolation_mode: BitmapInterpolationMode, is_upscaling: bool) -> (ImageQuality, bool) {
    match interpolation_mode {
        BitmapInterpolationMode::None => (ImageQuality::Low, false),
        BitmapInterpolationMode::Unspecified | BitmapInterpolationMode::LowQuality => (ImageQuality::Medium, false),
        BitmapInterpolationMode::MediumQuality => (ImageQuality::Medium, true),
        BitmapInterpolationMode::HighQuality => {
            if is_upscaling {
                (ImageQuality::High, false)
            } else {
                (ImageQuality::Medium, true)
            }
        }
    }
}

/// Converts a bitmap blending mode. Every mode of the contract has a
/// counterpart: the composition modes are Porter-Duff operators, the others
/// mix functions composed source-over.
pub fn to_blend_mode(blending_mode: BitmapBlendingMode) -> BlendMode {
    let compose = |compose| BlendMode::new(Mix::Normal, compose);
    let mix = |mix| BlendMode::new(mix, Compose::SrcOver);

    match blending_mode {
        BitmapBlendingMode::Unspecified | BitmapBlendingMode::SourceOver => compose(Compose::SrcOver),
        BitmapBlendingMode::Source => compose(Compose::Copy),
        BitmapBlendingMode::Destination => compose(Compose::Dest),
        BitmapBlendingMode::DestinationOver => compose(Compose::DestOver),
        BitmapBlendingMode::SourceIn => compose(Compose::SrcIn),
        BitmapBlendingMode::DestinationIn => compose(Compose::DestIn),
        BitmapBlendingMode::SourceOut => compose(Compose::SrcOut),
        BitmapBlendingMode::DestinationOut => compose(Compose::DestOut),
        BitmapBlendingMode::SourceAtop => compose(Compose::SrcAtop),
        BitmapBlendingMode::DestinationAtop => compose(Compose::DestAtop),
        BitmapBlendingMode::Xor => compose(Compose::Xor),
        BitmapBlendingMode::Plus => compose(Compose::Plus),
        BitmapBlendingMode::Screen => mix(Mix::Screen),
        BitmapBlendingMode::Overlay => mix(Mix::Overlay),
        BitmapBlendingMode::Darken => mix(Mix::Darken),
        BitmapBlendingMode::Lighten => mix(Mix::Lighten),
        BitmapBlendingMode::ColorDodge => mix(Mix::ColorDodge),
        BitmapBlendingMode::ColorBurn => mix(Mix::ColorBurn),
        BitmapBlendingMode::HardLight => mix(Mix::HardLight),
        BitmapBlendingMode::SoftLight => mix(Mix::SoftLight),
        BitmapBlendingMode::Difference => mix(Mix::Difference),
        BitmapBlendingMode::Exclusion => mix(Mix::Exclusion),
        BitmapBlendingMode::Multiply => mix(Mix::Multiply),
        BitmapBlendingMode::Hue => mix(Mix::Hue),
        BitmapBlendingMode::Saturation => mix(Mix::Saturation),
        BitmapBlendingMode::Color => mix(Mix::Color),
        BitmapBlendingMode::Luminosity => mix(Mix::Luminosity),
    }
}

/// The path of a rectangle: four lines, clockwise from the top left, closed.
pub fn rect_path(r: Rect) -> BezPath {
    let mut path = BezPath::new();
    path.move_to((r.x, r.y));
    path.line_to((r.x + r.width, r.y));
    path.line_to((r.x + r.width, r.y + r.height));
    path.line_to((r.x, r.y + r.height));
    path.close_path();
    path
}

/// The path of the ellipse inscribed in a rectangle.
pub fn ellipse_path(r: Rect) -> BezPath {
    kurbo::Ellipse::from_rect(to_kurbo_rect(r)).to_path(PATH_TOLERANCE)
}

/// The path of a rounded rectangle with elliptical corner radii, clockwise
/// from the top left.
///
/// Radii that do not fit the rectangle are scaled down together, by the
/// smallest ratio of a side to the sum of the two radii on it, as Skia does.
pub fn rounded_rect_path(r: RoundedRect) -> BezPath {
    if !r.is_rounded() {
        return rect_path(r.rect);
    }

    let rect = r.rect;
    let (mut tl, mut tr, mut br, mut bl) =
        (r.radii_top_left, r.radii_top_right, r.radii_bottom_right, r.radii_bottom_left);

    let mut scale = 1.0f64;
    for (side, sum) in [
        (rect.width, tl.x + tr.x),
        (rect.width, bl.x + br.x),
        (rect.height, tl.y + bl.y),
        (rect.height, tr.y + br.y),
    ] {
        if sum > side && sum > 0.0 {
            scale = scale.min(side / sum);
        }
    }
    if scale < 1.0 {
        let apply = |v: Vector| Vector::new(v.x * scale, v.y * scale);
        (tl, tr, br, bl) = (apply(tl), apply(tr), apply(br), apply(bl));
    }

    let (left, top, right, bottom) = (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height);
    let mut path = BezPath::new();

    // A quarter of an ellipse around `center`, starting at `start_angle`.
    let corner = |path: &mut BezPath, center: (f64, f64), radii: Vector, start_angle: f64| {
        if radii.x <= 0.0 || radii.y <= 0.0 {
            return;
        }
        let arc = kurbo::Arc::new(center, (radii.x, radii.y), start_angle, std::f64::consts::FRAC_PI_2, 0.0);
        arc.to_cubic_beziers(PATH_TOLERANCE, |p1, p2, p3| path.curve_to(p1, p2, p3));
    };

    use std::f64::consts::{FRAC_PI_2, PI};

    path.move_to((left + tl.x, top));
    path.line_to((right - tr.x, top));
    corner(&mut path, (right - tr.x, top + tr.y), tr, -FRAC_PI_2);
    path.line_to((right, bottom - br.y));
    corner(&mut path, (right - br.x, bottom - br.y), br, 0.0);
    path.line_to((left + bl.x, bottom));
    corner(&mut path, (left + bl.x, bottom - bl.y), bl, FRAC_PI_2);
    path.line_to((left, top + tl.y));
    corner(&mut path, (left + tl.x, top + tl.y), tl, PI);
    path.close_path();
    path
}

/// A copy of a path in which every figure is closed: what filling and hit
/// testing a fill see, since an open figure is filled as if it were closed.
pub fn closed_path(path: &BezPath) -> BezPath {
    let mut closed = BezPath::new();
    let mut open = false;

    for element in path.elements() {
        match element {
            PathEl::MoveTo(_) => {
                if open {
                    closed.close_path();
                }
                open = true;
            }
            PathEl::ClosePath => open = false,
            _ => {}
        }
        closed.push(*element);
    }
    if open {
        closed.close_path();
    }

    closed
}
