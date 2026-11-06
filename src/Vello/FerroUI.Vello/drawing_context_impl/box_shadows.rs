//! The box shadows of a rectangle.
//!
//! The geometry is that of the Skia backend: an outset shadow is the box
//! grown by the spread, moved by the offset and blurred, drawn everywhere
//! but in the box; an inset shadow is everything around the box shrunk by
//! the spread, moved and blurred, drawn in the box only. The Skia backend
//! draws both with a paint that carries a blur filter. Here a shadow is
//! drawn
//!
//! - as a plain fill when it has no blur,
//! - by the renderer's blurred rounded rectangle, in closed form, when the
//!   shadow is one for which that form agrees with the Gaussian
//!   ([`closed_form_radius`]),
//! - otherwise as an image: the shape is drawn on the processor into a
//!   scene of its own, through a blur layer, and the picture of it is
//!   composed under the clip of the shadow ([`DrawingContextImpl::draw_blurred_path`]).

use super::DrawingContextImpl;
use crate::scene::{
    VelloSceneBrush, VelloSceneFilter, VelloSceneImage, VelloScenePaint,
};
use crate::vello_extensions::{rect_path, rounded_rect_path, to_kurbo_rect};
use ferroui_base::media::{BoxShadow, BoxShadows};
use ferroui_base::{Matrix, Rect, RoundedRect, Vector};
use kurbo::{Affine, BezPath, Shape};
use peniko::color::{AlphaColor, Srgb};
use peniko::{BlendMode, Extend, Fill, ImageQuality};

/// The standard deviation of the Gaussian of a blur radius, as Skia
/// converts one (`SkBlurMask::ConvertRadiusToSigma`).
pub(crate) fn blur_radius_to_sigma(radius: f64) -> f32 {
    if radius <= 0.0 {
        return 0.0;
    }
    0.288675 * radius as f32 + 0.5
}

/// How far, in standard deviations, a blur reaches: beyond it less than a
/// four-hundredth of the color is left.
pub(super) const BLUR_REACH: f64 = 3.0;

/// A rounded rectangle with every side moved outwards by `delta` (inwards
/// when it is negative), as `SkRRect::outset` and `SkRRect::inset` do it: a
/// corner that is round has its radii changed by as much and a corner that
/// is square stays square. `None` when nothing is left of the rectangle.
pub(crate) fn outset_rounded_rect(rect: &RoundedRect, delta: f64) -> Option<RoundedRect> {
    let r = rect.rect;
    let (width, height) = (r.width + 2.0 * delta, r.height + 2.0 * delta);
    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let radii = |radii: Vector| {
        let x = if radii.x != 0.0 { radii.x + delta } else { 0.0 };
        let y = if radii.y != 0.0 { radii.y + delta } else { 0.0 };
        // A corner with a radius that is not positive is square.
        if x <= 0.0 || y <= 0.0 {
            Vector::new(0.0, 0.0)
        } else {
            Vector::new(x, y)
        }
    };

    Some(RoundedRect {
        rect: Rect::new(r.x - delta, r.y - delta, width, height),
        radii_top_left: radii(rect.radii_top_left),
        radii_top_right: radii(rect.radii_top_right),
        radii_bottom_left: radii(rect.radii_bottom_left),
        radii_bottom_right: radii(rect.radii_bottom_right),
    })
}

/// The area around a box that casts an inset shadow into it: the box grown
/// by the blur (and by a negative spread), and by the offset on the side the
/// shadow comes from. Adapted from Chromium by the original.
pub(crate) fn area_casting_shadow_in_hole(
    hole_rect: Rect,
    shadow_blur: f64,
    shadow_spread: f64,
    offset_x: f64,
    offset_y: f64,
) -> Rect {
    let mut bounds = hole_rect.inflate(shadow_blur);

    if shadow_spread < 0.0 {
        bounds = bounds.inflate(-shadow_spread);
    }

    let offset_bounds = bounds.translate(Vector::new(-offset_x, -offset_y));
    bounds.union(offset_bounds)
}

/// The corner radius with which the blur of a rounded rectangle in closed
/// form (`IVelloSceneSink::fill_blurred_rounded_rect`) stands for the
/// Gaussian blur of the shape, or `None` when it does not.
///
/// The closed form is an approximation by a distance field. Measured against
/// the Gaussian blur of the same shape (the test
/// `the_closed_form_of_a_blurred_rounded_rectangle_agrees_with_the_gaussian`),
/// it stays within 8 of 255 when the four corners have one circular radius
/// that is at most a quarter of the shorter side, that side is at least
/// five standard deviations long and the deviation is at least a pixel; a
/// capsule or a shape that is thin against its blur is up to 22 of 255 off
/// and is blurred as an image instead, and so is a blur of less than a
/// pixel, where the closed form, which is evaluated at the middle of a
/// pixel, lacks the anti-aliasing of the edge.
pub(crate) fn closed_form_radius(shape: &RoundedRect, std_deviation: f64) -> Option<f64> {
    let min_edge = shape.rect.width.min(shape.rect.height);
    if std_deviation < 1.0 || min_edge < 5.0 * std_deviation {
        return None;
    }

    let radius = shape.radii_top_left.x;
    let corners =
        [shape.radii_top_left, shape.radii_top_right, shape.radii_bottom_left, shape.radii_bottom_right];
    if corners.iter().any(|corner| corner.x != radius || corner.y != radius) {
        return None;
    }

    (radius >= 0.0 && radius <= min_edge / 4.0).then_some(radius)
}

/// The largest factor by which a transform stretches a length: the larger
/// of its singular values.
pub(crate) fn largest_scale(transform: Affine) -> f64 {
    let [a, b, c, d, _, _] = transform.as_coeffs();
    let sum = a * a + b * b + c * c + d * d;
    let difference = ((a * a + b * b - c * c - d * d).powi(2) + 4.0 * (a * c + b * d).powi(2)).sqrt();
    ((sum + difference) / 2.0).sqrt()
}

/// A rectangle with its sides in order, as Skia sorts the rectangle it is
/// asked to draw.
fn sorted(rect: Rect) -> Rect {
    let (left, right) = (rect.x.min(rect.x + rect.width), rect.x.max(rect.x + rect.width));
    let (top, bottom) = (rect.y.min(rect.y + rect.height), rect.y.max(rect.y + rect.height));
    Rect::new(left, top, right - left, bottom - top)
}

impl DrawingContextImpl {
    /// The color a shadow is drawn with: its alpha times the opacity of the
    /// context, as a byte.
    fn box_shadow_color(&self, shadow: &BoxShadow) -> AlphaColor<Srgb> {
        let color = shadow.color;
        AlphaColor::from_rgba8(color.r, color.g, color.b, (color.a as f64 * self.current_opacity) as u8)
    }

    /// Draws the shadows of a box that lie outside of it (`inset` false:
    /// before the box is filled) or inside of it (`inset` true: after).
    pub(super) fn draw_box_shadows(&mut self, rect: &RoundedRect, box_shadows: &BoxShadows, inset: bool) {
        for box_shadow in box_shadows.iter() {
            if box_shadow == BoxShadow::default() || box_shadow.is_inset != inset {
                continue;
            }

            if inset {
                self.draw_inset_box_shadow(rect, &box_shadow);
            } else {
                self.draw_outset_box_shadow(rect, &box_shadow);
            }
        }
    }

    fn draw_outset_box_shadow(&mut self, rect: &RoundedRect, shadow: &BoxShadow) {
        let color = self.box_shadow_color(shadow);
        let std_deviation = blur_radius_to_sigma(shadow.blur) as f64;
        let is_rounded = rect.is_rounded();

        // The shadow: the box grown by the spread.
        let shape = if is_rounded {
            outset_rounded_rect(rect, shadow.spread)
        } else {
            let grown = sorted(rect.rect.inflate(shadow.spread));
            (grown.width > 0.0 && grown.height > 0.0).then(|| RoundedRect::from_rect(grown))
        };

        // Everything but the box, in the pixels of the target: the edge of
        // a rounded box is anti-aliased and that of a rectangle is not, as
        // the Skia backend clips them.
        let transform = self.device_transform();
        let mut clip = self.target_path();
        clip.extend(transform * rounded_rect_path(*rect));
        let anti_alias = self.edge_anti_alias(is_rounded);
        self.sink().push_clip(&clip, Fill::EvenOdd, Affine::IDENTITY, anti_alias);

        if let Some(shape) = shape {
            self.with_box_shadow_offset(shadow, |context| {
                context.draw_shadow_shape(&rounded_rect_path(shape), Fill::NonZero, Some(&shape), std_deviation, color);
            });
        }

        self.sink().pop_clip();
    }

    fn draw_inset_box_shadow(&mut self, rect: &RoundedRect, shadow: &BoxShadow) {
        let color = self.box_shadow_color(shadow);
        let std_deviation = blur_radius_to_sigma(shadow.blur) as f64;

        let outer_rect =
            area_casting_shadow_in_hole(rect.rect, shadow.blur, shadow.spread, shadow.offset_x, shadow.offset_y);

        // What casts the shadow: the area around the box, with the box
        // shrunk by the spread as its hole. A box of which the spread
        // leaves nothing has no hole.
        let mut shape = rect_path(outer_rect);
        let hole = if shadow.spread != 0.0 { outset_rounded_rect(rect, -shadow.spread) } else { Some(*rect) };
        if let Some(hole) = hole {
            shape.extend(rounded_rect_path(hole));
        }

        // Inside the box only.
        let transform = self.device_transform();
        self.sink().push_clip(&rounded_rect_path(*rect), Fill::NonZero, transform, true);

        self.with_box_shadow_offset(shadow, |context| {
            context.draw_shadow_shape(&shape, Fill::EvenOdd, None, std_deviation, color);
        });

        self.sink().pop_clip();
    }

    /// Draws with the transform of the context followed by the offset of a
    /// shadow: the offset is in the space the transform maps to, as in the
    /// original.
    fn with_box_shadow_offset(&mut self, shadow: &BoxShadow, draw: impl FnOnce(&mut Self)) {
        let old_transform = self.current_transform;
        self.current_transform = old_transform * Matrix::create_translation(shadow.offset_x, shadow.offset_y);
        draw(self);
        self.current_transform = old_transform;
    }

    /// Draws a shape blurred by a Gaussian of `std_deviation`, in one
    /// color. `rounded` is the shape as a rounded rectangle when it is one.
    fn draw_shadow_shape(
        &mut self,
        path: &BezPath,
        fill_rule: Fill,
        rounded: Option<&RoundedRect>,
        std_deviation: f64,
        color: AlphaColor<Srgb>,
    ) {
        let transform = self.device_transform();

        // Without a blur the paint of the Skia backend has no filter: the
        // shape is filled, with anti-aliased edges whatever the edge mode.
        if std_deviation <= 0.0 {
            self.sink().fill(path, fill_rule, transform, &VelloScenePaint::solid(color), BlendMode::default(), true);
            return;
        }

        if self.sink().filter_capabilities().blurred_rounded_rects {
            if let Some((shape, radius)) =
                rounded.and_then(|shape| Some((shape, closed_form_radius(shape, std_deviation)?)))
            {
                self.sink().fill_blurred_rounded_rect(
                    to_kurbo_rect(shape.rect),
                    radius,
                    std_deviation,
                    false,
                    transform,
                    color,
                );
                return;
            }
        }

        self.draw_blurred_path(path, fill_rule, std_deviation, color);
    }

    /// Draws the fill of a path blurred by a Gaussian of `std_deviation`
    /// (in the units of the path), in one color.
    ///
    /// The shape is drawn through a blur layer by the CPU renderer, which
    /// every build of the backend has, into a scene of its own that is as
    /// large as the blur reaches inside the target, and the picture is
    /// composed into the scene of the context: under its clips, in every
    /// rendering mode.
    pub(super) fn draw_blurred_path(
        &mut self,
        path: &BezPath,
        fill_rule: Fill,
        std_deviation: f64,
        color: AlphaColor<Srgb>,
    ) {
        let transform = self.device_transform();
        let reach = (BLUR_REACH * std_deviation * largest_scale(transform)).ceil() + 1.0;

        let target = kurbo::Rect::new(0.0, 0.0, self.sink().width() as f64, self.sink().height() as f64);
        let bounds = (transform * path).bounding_box().inflate(reach, reach).expand().intersect(target);
        if !(bounds.width() >= 1.0 && bounds.height() >= 1.0) {
            return;
        }
        let (width, height) = (bounds.width() as u16, bounds.height() as u16);

        // What of the shape lies beyond the scene still blurs into it: the
        // renderer draws as much of the content of a filter layer as its
        // filter reaches.
        let scene_transform = Affine::translate((-bounds.x0, -bounds.y0)) * transform;

        // A picture the CPU renderer makes is kept: the same shadow at
        // another place on whole pixels, or in the next frame, is the same
        // picture.
        let key = (!self.filter_scene_on_device())
            .then(|| ShadowKey::new(path, fill_rule, scene_transform, std_deviation, color, width, height));
        if let Some(image) = key.as_ref().and_then(ShadowPictures::get) {
            self.draw_image_at_pixels(image, bounds.x0, bounds.y0);
            return;
        }

        let mut scene = self.create_filter_scene(width, height);
        scene.push_filter_layer(&VelloSceneFilter::Blur { std_deviation: std_deviation as f32 }, scene_transform);
        scene.fill(path, fill_rule, scene_transform, &VelloScenePaint::solid(color), BlendMode::default(), true);
        scene.pop_layer();

        match key {
            Some(key) => {
                let image = self.render_filter_scene_to_image(&mut *scene, crate::perf::Phase::ShadowAsImage);
                ShadowPictures::insert(key, image.clone());
                self.draw_image_at_pixels(image, bounds.x0, bounds.y0);
            }
            None => {
                self.draw_filter_scene(&mut *scene, bounds.x0, bounds.y0, width, height, crate::perf::Phase::ShadowAsImage)
            }
        }
    }

    /// Draws an image with its pixels on the pixels of the scene of the
    /// context, its top left pixel at (`x`, `y`).
    pub(super) fn draw_image_at_pixels(&mut self, image: peniko::ImageData, x: f64, y: f64) {
        let (width, height) = (image.width as f64, image.height as f64);
        let brush = VelloSceneBrush::Image(VelloSceneImage {
            image,
            x_extend: Extend::Pad,
            y_extend: Extend::Pad,
            quality: ImageQuality::Low,
            alpha: 1.0,
        });

        self.draw_brush_at_pixels(brush, x, y, width, height);
    }

    /// Fills the rectangle of `width` by `height` pixels at (`x`, `y`) of
    /// the scene of the context with a brush whose first pixel lies there.
    pub(super) fn draw_brush_at_pixels(&mut self, brush: VelloSceneBrush, x: f64, y: f64, width: f64, height: f64) {
        let path = rect_path(Rect::new(x, y, width, height));
        let paint = VelloScenePaint { brush, transform: Affine::translate((x, y)) };

        self.sink().fill(&path, Fill::NonZero, Affine::IDENTITY, &paint, BlendMode::default(), false);
    }
}

/// What makes the picture of a blurred shape: the shape in the pixels of
/// its scene, how it is filled, the blur, the color and the size of the
/// scene. The numbers are kept as their bits: a picture is reused for the
/// very same shape only.
#[derive(Clone, PartialEq, Eq, Hash)]
struct ShadowKey {
    path: Vec<u64>,
    fill_rule: u8,
    std_deviation: u64,
    color: [u32; 4],
    size: (u16, u16),
}

impl ShadowKey {
    fn new(
        path: &BezPath,
        fill_rule: Fill,
        transform: Affine,
        std_deviation: f64,
        color: AlphaColor<Srgb>,
        width: u16,
        height: u16,
    ) -> Self {
        let mut points = Vec::with_capacity(path.elements().len() * 5);
        let mut push = |tag: u64, element_points: &[kurbo::Point]| {
            points.push(tag);
            for point in element_points {
                let point = transform * *point;
                points.push(point.x.to_bits());
                points.push(point.y.to_bits());
            }
        };
        for element in path.elements() {
            match *element {
                kurbo::PathEl::MoveTo(p) => push(0, &[p]),
                kurbo::PathEl::LineTo(p) => push(1, &[p]),
                kurbo::PathEl::QuadTo(a, b) => push(2, &[a, b]),
                kurbo::PathEl::CurveTo(a, b, c) => push(3, &[a, b, c]),
                kurbo::PathEl::ClosePath => push(4, &[]),
            }
        }
        Self {
            path: points,
            fill_rule: matches!(fill_rule, Fill::EvenOdd) as u8,
            std_deviation: std_deviation.to_bits(),
            color: color.components.map(f32::to_bits),
            size: (width, height),
        }
    }
}

/// The pictures of blurred shapes the CPU renderer made, kept on the thread
/// that draws: a box shadow without a closed form is the same picture from
/// frame to frame. The pictures used least recently go once they hold more
/// than [`ShadowPictures::BYTES`].
struct ShadowPictures {
    pictures: std::collections::HashMap<ShadowKey, (peniko::ImageData, u64)>,
    bytes: usize,
    uses: u64,
}

ferroui_base::ferro_thread_local! {
    static SHADOW_PICTURES: std::cell::RefCell<Option<ShadowPictures>> = const { std::cell::RefCell::new(None) };
}

impl ShadowPictures {
    /// The most pixel bytes the pictures of a thread hold.
    const BYTES: usize = 32 * 1024 * 1024;

    fn get(key: &ShadowKey) -> Option<peniko::ImageData> {
        SHADOW_PICTURES
            .try_with(|pictures| {
                let mut pictures = pictures.borrow_mut();
                let pictures = pictures.as_mut()?;
                pictures.uses += 1;
                let uses = pictures.uses;
                let (image, used) = pictures.pictures.get_mut(key)?;
                *used = uses;
                Some(image.clone())
            })
            .ok()
            .flatten()
    }

    fn insert(key: ShadowKey, image: peniko::ImageData) {
        let size = image.data.data().len();
        if size > Self::BYTES / 4 {
            return;
        }
        let _ = SHADOW_PICTURES.try_with(|pictures| {
            let mut pictures = pictures.borrow_mut();
            let pictures = pictures.get_or_insert_with(|| ShadowPictures {
                pictures: std::collections::HashMap::new(),
                bytes: 0,
                uses: 0,
            });
            pictures.uses += 1;
            if let Some((old, _)) = pictures.pictures.insert(key, (image, pictures.uses)) {
                pictures.bytes -= old.data.data().len();
            }
            pictures.bytes += size;

            if pictures.bytes > Self::BYTES {
                let mut by_use: Vec<(u64, ShadowKey)> =
                    pictures.pictures.iter().map(|(key, (_, used))| (*used, key.clone())).collect();
                by_use.sort_unstable_by_key(|(used, _)| *used);
                for (_, key) in by_use {
                    if pictures.bytes <= Self::BYTES / 2 {
                        break;
                    }
                    if let Some((image, _)) = pictures.pictures.remove(&key) {
                        pictures.bytes -= image.data.data().len();
                    }
                }
            }
        });
    }
}
