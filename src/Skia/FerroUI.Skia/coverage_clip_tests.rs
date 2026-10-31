//! The clips of rounded rectangles that a GPU surface could not apply
//! analytically (under a rotation, or with complex radii) drawn as coverage
//! shaders, compared with Skia's own clip on a raster surface (the
//! reference).
//!
//! Both are compared with the exact coverage of each pixel (the area of
//! the pixel inside the clip, by 16 by 16 samples of the rounded rectangle
//! in the coordinates of the drawing). The coverage shader ramps linearly
//! over a device pixel across the edge, as Ganesh's analytic clips do; that
//! is the area exactly for an edge on an axis and differs from it by at
//! most 0.05 of a pixel (12 levels) for an edge at an angle, plus the first
//! order distance to the arc of a corner. Skia's raster clip is not exact
//! either (it flattens the arcs and samples the coverage); the coverage
//! shader is asserted within 16 levels of the exact coverage and never
//! further from it on average than Skia's clip. Off the edge (full and no
//! coverage) both are exact.
//!
//! The shadows are blurred, so there the clip's band is all that differs:
//! the mean difference from Skia's clip is asserted.

use crate::drawing_context_impl::{rounded_clip_counts, set_coverage_clips_on_raster};
use crate::PlatformRenderInterface;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadow, BoxShadows, Color};
use ferroui_base::platform::{IDrawingContextImpl, IPlatformRenderInterface, PixelFormat};
use ferroui_base::{CornerRadius, Matrix, PixelSize, Rect, RoundedRect, Vector};

pub(crate) const SIZE: i32 = 140;

/// The alpha of each pixel of a raster target drawn by `draw`, with the
/// clips of rounded rectangles as coverage shaders or as Skia's clips.
fn alphas(coverage_shaders: bool, draw: &dyn Fn(&mut dyn IDrawingContextImpl)) -> Vec<u8> {
    let previous = set_coverage_clips_on_raster(coverage_shaders);
    let target = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(SIZE, SIZE), Vector::new(96.0, 96.0));
    {
        let mut context = target.create_drawing_context();
        draw(&mut *context);
        context.dispose();
    }
    set_coverage_clips_on_raster(previous);
    let framebuffer = target.lock();
    let row_bytes = framebuffer.row_bytes() as usize;
    assert!(matches!(framebuffer.format(), PixelFormat::BGRA8888 | PixelFormat::RGBA8888));
    let mut result = vec![0u8; (SIZE * SIZE) as usize];
    framebuffer.with_data(&mut |data| {
        for y in 0..SIZE as usize {
            for x in 0..SIZE as usize {
                result[y * SIZE as usize + x] = data[y * row_bytes + x * 4 + 3];
            }
        }
    });
    framebuffer.dispose();
    result
}

/// A rotated card: rounded corners, under a rotation about its centre.
fn card_transform(angle: f64) -> Matrix {
    Matrix::create_translation(-50.0, -35.0) * Matrix::create_rotation(angle) * Matrix::create_translation(70.0, 70.0)
}

fn card() -> Rect {
    Rect::new(0.0, 0.0, 100.0, 70.0)
}

pub(crate) fn fill(context: &mut dyn IDrawingContextImpl) {
    let brush = ImmutableSolidColorBrush::new(Color::from_rgb(200, 40, 20));
    context.draw_rectangle(Some(&brush), None, RoundedRect::from_rect(Rect::new(-200.0, -200.0, 600.0, 600.0)), &BoxShadows::default());
}

/// A clip of a scene: its transform and its rounded rectangle.
#[derive(Clone, Copy)]
pub(crate) struct ClipScene {
    pub(crate) name: &'static str,
    pub(crate) transform: Matrix,
    pub(crate) radius: CornerRadius,
}

impl ClipScene {
    pub(crate) fn draw(&self, context: &mut dyn IDrawingContextImpl) {
        context.set_transform(self.transform);
        context.push_clip_rounded(RoundedRect::from_corner_radius(card(), self.radius));
        fill(context);
        context.pop_clip();
    }

    /// Whether `(x, y)` of the drawing is inside the rounded rectangle.
    fn contains(&self, x: f64, y: f64) -> bool {
        let rect = card();
        if x < rect.x || y < rect.y || x > rect.right() || y > rect.bottom() {
            return false;
        }
        let corners = [
            (self.radius.top_left, rect.x + self.radius.top_left, rect.y + self.radius.top_left, x < rect.x + self.radius.top_left && y < rect.y + self.radius.top_left),
            (self.radius.top_right, rect.right() - self.radius.top_right, rect.y + self.radius.top_right, x > rect.right() - self.radius.top_right && y < rect.y + self.radius.top_right),
            (self.radius.bottom_right, rect.right() - self.radius.bottom_right, rect.bottom() - self.radius.bottom_right, x > rect.right() - self.radius.bottom_right && y > rect.bottom() - self.radius.bottom_right),
            (self.radius.bottom_left, rect.x + self.radius.bottom_left, rect.bottom() - self.radius.bottom_left, x < rect.x + self.radius.bottom_left && y > rect.bottom() - self.radius.bottom_left),
        ];
        for (radius, cx, cy, inside_corner) in corners {
            if radius > 0.0 && inside_corner {
                return (x - cx).powi(2) + (y - cy).powi(2) <= radius * radius;
            }
        }
        true
    }

    /// The exact coverage of each pixel, as an alpha.
    pub(crate) fn exact(&self) -> Vec<u8> {
        let inverse = self.transform.try_invert().unwrap();
        let mut result = vec![0u8; (SIZE * SIZE) as usize];
        const SAMPLES: i32 = 16;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let mut inside = 0;
                for sy in 0..SAMPLES {
                    for sx in 0..SAMPLES {
                        let device = ferroui_base::Point::new(
                            x as f64 + (sx as f64 + 0.5) / SAMPLES as f64,
                            y as f64 + (sy as f64 + 0.5) / SAMPLES as f64,
                        );
                        let local = inverse.transform(device);
                        inside += i32::from(self.contains(local.x, local.y));
                    }
                }
                result[(y * SIZE + x) as usize] = (f64::from(inside) / f64::from(SAMPLES * SAMPLES) * 255.0).round() as u8;
            }
        }
        result
    }
}

pub(crate) fn clip_scenes() -> Vec<ClipScene> {
    vec![
        ClipScene { name: "circular corners, rotated", transform: card_transform(0.35), radius: CornerRadius::uniform(12.0) },
        ClipScene { name: "circular corners, a quarter turn and a bit", transform: card_transform(1.7), radius: CornerRadius::uniform(20.0) },
        ClipScene { name: "different corners, rotated", transform: card_transform(-0.5), radius: CornerRadius::new(4.0, 18.0, 30.0, 0.0) },
        ClipScene { name: "different corners, upright", transform: card_transform(0.0), radius: CornerRadius::new(4.0, 18.0, 30.0, 0.0) },
        ClipScene {
            name: "scaled and skewed",
            transform: Matrix::new(1.0, 0.2, -0.3, 0.8, 0.0, 0.0) * card_transform(0.2),
            radius: CornerRadius::uniform(16.0),
        },
    ]
}

/// The shadows of rotated cards: outer (outside the box) and inset.
pub(crate) fn shadow_scenes() -> Vec<(&'static str, Box<dyn Fn(&mut dyn IDrawingContextImpl)>)> {
    fn card_with(angle: f64, radius: f64, shadow: BoxShadow) -> Box<dyn Fn(&mut dyn IDrawingContextImpl)> {
        Box::new(move |context: &mut dyn IDrawingContextImpl| {
            context.set_transform(card_transform(angle));
            context.draw_rectangle(None, None, RoundedRect::from_corner_radius(card(), CornerRadius::uniform(radius)), &BoxShadows::new(shadow));
        })
    }
    vec![
        (
            "the outer shadow of a rotated card",
            card_with(0.3, 8.0, BoxShadow { offset_x: 0.0, offset_y: 10.0, blur: 28.0, spread: 0.0, color: Color::from_argb(0x88, 0, 0, 0), is_inset: false }),
        ),
        (
            "the inset shadow of a rotated card",
            card_with(-0.3, 14.0, BoxShadow { offset_x: 4.0, offset_y: 4.0, blur: 10.0, spread: 2.0, color: Color::from_argb(0xff, 0, 0, 0), is_inset: true }),
        ),
    ]
}

/// The largest and the mean difference of the alphas over the pixels where
/// either is partial (the band of the edge), and whether every other pixel
/// is equal.
#[allow(dead_code)]
pub(crate) fn compare_alphas(reference: &[u8], actual: &[u8]) -> (i32, f64, bool) {
    let (mut largest, mut sum, mut count, mut exact_elsewhere) = (0, 0i64, 0i64, true);
    for (r, a) in reference.iter().zip(actual) {
        let partial = (*r != 0 && *r != 255) || (*a != 0 && *a != 255);
        let difference = (i32::from(*r) - i32::from(*a)).abs();
        if partial {
            largest = largest.max(difference);
            sum += i64::from(difference);
            count += 1;
        } else if difference != 0 {
            exact_elsewhere = false;
        }
    }
    (largest, if count == 0 { 0.0 } else { sum as f64 / count as f64 }, exact_elsewhere)
}

#[test]
fn coverage_clips_match_the_exact_coverage() {
    for scene in clip_scenes() {
        let exact = scene.exact();
        let skia = alphas(false, &|context| scene.draw(context));
        let shader = alphas(true, &|context| scene.draw(context));
        let (shader_largest, shader_mean, _) = compare_alphas(&exact, &shader);
        let (_, skia_mean, _) = compare_alphas(&exact, &skia);
        let (_, _, off_the_edge) = compare_alphas(&skia, &shader);
        let name = scene.name;
        assert!(off_the_edge, "{name}: the pixels off the edge are the same");
        assert!(shader_largest <= 16, "{name}: within 16 levels of the exact coverage: {shader_largest}");
        assert!(shader_mean <= skia_mean + 0.5, "{name}: as close to the exact coverage as Skia's clip: {shader_mean} {skia_mean}");
    }
}

#[test]
fn the_shadows_of_rotated_cards_keep_their_clips() {
    for (name, draw) in shadow_scenes() {
        let skia = alphas(false, &*draw);
        let shader = alphas(true, &*draw);
        let (_, mean, _) = compare_alphas(&skia, &shader);
        assert!(mean <= 4.0, "{name}: the clip of the box differs by a few levels on average: {mean}");
        assert!(skia.iter().any(|alpha| *alpha > 0), "{name}: a shadow is drawn");
    }
}

#[test]
fn only_clips_a_gpu_surface_cannot_apply_analytically_are_coverage_shaders() {
    let previous = set_coverage_clips_on_raster(true);
    let (before, _) = rounded_clip_counts();
    let target = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(SIZE, SIZE), Vector::new(96.0, 96.0));
    let mut context = target.create_drawing_context();
    // Upright with the same radius at every corner: analytic.
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::uniform(10.0)));
    context.pop_clip();
    // A rectangle under a rotation: a rotated rectangle is a convex polygon, analytic.
    context.set_transform(card_transform(0.4));
    context.push_clip_rounded(RoundedRect::from_rect(card()));
    context.pop_clip();
    assert_eq!(before, rounded_clip_counts().0);
    // Rotated rounded corners and complex radii are not.
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::uniform(10.0)));
    context.pop_clip();
    context.set_transform(Matrix::IDENTITY);
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::new(2.0, 9.0, 4.0, 0.0)));
    context.pop_clip();
    assert_eq!(before + 2, rounded_clip_counts().0);
    context.dispose();
    set_coverage_clips_on_raster(previous);
    // Without the setting a raster surface keeps Skia's clips.
    let (before, masks) = rounded_clip_counts();
    let mut context = target.create_drawing_context();
    context.set_transform(card_transform(0.4));
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::uniform(10.0)));
    context.pop_clip();
    context.dispose();
    assert_eq!((before, masks), rounded_clip_counts());
}
