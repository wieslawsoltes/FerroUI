//! The clips of rounded rectangles drawn as coverage shaders, as every
//! rounded clip is on a GPU surface, compared with Skia's own clip on a
//! raster surface: rotated, skewed, with complex radii, nested with a shared
//! edge, and the clips of box shadows.
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
        rounded_rect_contains(card(), self.radius, x, y)
    }

    /// The exact coverage of each pixel, as an alpha.
    pub(crate) fn exact(&self) -> Vec<u8> {
        exact_coverage(self.transform, &|x, y| self.contains(x, y))
    }
}

/// Whether `(x, y)` is inside `rect` with the corners of `radius`.
fn rounded_rect_contains(rect: Rect, radius: CornerRadius, x: f64, y: f64) -> bool {
    if x < rect.x || y < rect.y || x > rect.right() || y > rect.bottom() {
        return false;
    }
    let corners = [
        (radius.top_left, rect.x + radius.top_left, rect.y + radius.top_left, x < rect.x + radius.top_left && y < rect.y + radius.top_left),
        (radius.top_right, rect.right() - radius.top_right, rect.y + radius.top_right, x > rect.right() - radius.top_right && y < rect.y + radius.top_right),
        (radius.bottom_right, rect.right() - radius.bottom_right, rect.bottom() - radius.bottom_right, x > rect.right() - radius.bottom_right && y > rect.bottom() - radius.bottom_right),
        (radius.bottom_left, rect.x + radius.bottom_left, rect.bottom() - radius.bottom_left, x < rect.x + radius.bottom_left && y > rect.bottom() - radius.bottom_left),
    ];
    for (r, cx, cy, inside_corner) in corners {
        if r > 0.0 && inside_corner {
            return (x - cx).powi(2) + (y - cy).powi(2) <= r * r;
        }
    }
    true
}

/// The exact coverage of each pixel under `transform` of the area `inside`
/// says, by 16 by 16 samples, as an alpha.
pub(crate) fn exact_coverage(transform: Matrix, inside: &dyn Fn(f64, f64) -> bool) -> Vec<u8> {
    let inverse = transform.try_invert().unwrap();
    let mut result = vec![0u8; (SIZE * SIZE) as usize];
    const SAMPLES: i32 = 16;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut count = 0;
            for sy in 0..SAMPLES {
                for sx in 0..SAMPLES {
                    let device = ferroui_base::Point::new(
                        x as f64 + (sx as f64 + 0.5) / SAMPLES as f64,
                        y as f64 + (sy as f64 + 0.5) / SAMPLES as f64,
                    );
                    let local = inverse.transform(device);
                    count += i32::from(inside(local.x, local.y));
                }
            }
            result[(y * SIZE + x) as usize] = (f64::from(count) / f64::from(SAMPLES * SAMPLES) * 255.0).round() as u8;
        }
    }
    result
}

/// A card clipping to its rounded corners and a picture in it clipping to
/// its own, the two sharing their right edge: the clips Ganesh merges into
/// one rounded rectangle with different corners (the software masks
/// measured in the Ferro Store).
pub(crate) struct NestedClips {
    pub(crate) transform: Matrix,
}

impl NestedClips {
    const CARD: Rect = Rect { x: 5.0, y: 5.0, width: 120.0, height: 90.0 };
    const PICTURE: Rect = Rect { x: 60.0, y: 20.0, width: 65.0, height: 110.0 };

    pub(crate) fn draw(&self, context: &mut dyn IDrawingContextImpl) {
        context.set_transform(self.transform);
        context.push_clip_rounded(RoundedRect::from_corner_radius(Self::CARD, CornerRadius::uniform(13.0)));
        context.push_clip_rounded(RoundedRect::from_corner_radius(Self::PICTURE, CornerRadius::uniform(10.0)));
        fill(context);
        context.pop_clip();
        context.pop_clip();
    }

    /// The exact coverage of each clip, multiplied: anti-aliased clips
    /// combine their coverages by multiplying them (Ganesh's analytic clips
    /// and Skia's raster clips alike), which at the shared edge is less
    /// than the area of the intersection (a quarter for a pixel half covered
    /// by both).
    pub(crate) fn exact(&self) -> Vec<u8> {
        let card = exact_coverage(self.transform, &|x, y| rounded_rect_contains(Self::CARD, CornerRadius::uniform(13.0), x, y));
        let picture =
            exact_coverage(self.transform, &|x, y| rounded_rect_contains(Self::PICTURE, CornerRadius::uniform(10.0), x, y));
        card.iter().zip(&picture).map(|(a, b)| ((u32::from(*a) * u32::from(*b) + 127) / 255) as u8).collect()
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
fn nested_rounded_clips_sharing_an_edge_match_the_exact_coverage() {
    let scene = NestedClips { transform: Matrix::create_translation(3.25, 4.5) };
    let exact = scene.exact();
    let shader = alphas(true, &|context| scene.draw(context));
    let skia = alphas(false, &|context| scene.draw(context));
    let (largest, mean, _) = compare_alphas(&exact, &shader);
    let (_, skia_mean, _) = compare_alphas(&exact, &skia);
    assert!(largest <= 16, "within 16 levels of the exact coverage: {largest}");
    assert!(mean <= skia_mean + 0.5, "as close to the exact coverage as Skia's clip: {mean} {skia_mean}");
    assert!(shader.iter().any(|alpha| *alpha == 255));
}

#[test]
fn on_a_gpu_surface_every_rounded_clip_is_a_coverage_shader() {
    let previous = set_coverage_clips_on_raster(true);
    let (before, _) = rounded_clip_counts();
    let target = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(SIZE, SIZE), Vector::new(96.0, 96.0));
    let mut context = target.create_drawing_context();
    // A rectangle, upright or rotated, is not rounded: Skia's clip (a
    // scissor or a convex polygon).
    context.push_clip_rounded(RoundedRect::from_rect(card()));
    context.pop_clip();
    context.set_transform(card_transform(0.4));
    context.push_clip_rounded(RoundedRect::from_rect(card()));
    context.pop_clip();
    assert_eq!(before, rounded_clip_counts().0);
    // Rounded corners, rotated or upright, simple or complex, are shaders.
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::uniform(10.0)));
    context.pop_clip();
    context.set_transform(Matrix::IDENTITY);
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::uniform(10.0)));
    context.pop_clip();
    context.push_clip_rounded(RoundedRect::from_corner_radius(card(), CornerRadius::new(2.0, 9.0, 4.0, 0.0)));
    context.pop_clip();
    assert_eq!(before + 3, rounded_clip_counts().0);
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

/// The prewarm of the runtime effects compiles every program, draws each
/// once, and leaves the target as it was (the corner it drew in cleared).
#[test]
fn the_prewarm_compiles_every_program_and_leaves_nothing_drawn() {
    let target = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(SIZE, SIZE), Vector::new(96.0, 96.0));
    let mut context = target.create_drawing_context();
    context.set_transform(Matrix::create_translation(10.0, 10.0));
    let skia = context.as_any_mut().downcast_mut::<crate::DrawingContextImpl>().unwrap();
    assert_eq!(7, skia.prewarm_runtime_effects());
    // The transform of the context is kept.
    assert_eq!(Matrix::create_translation(10.0, 10.0), context.transform());
    context.dispose();
    let framebuffer = target.lock();
    let mut any = false;
    framebuffer.with_data(&mut |data| any = data.iter().any(|byte| *byte != 0));
    framebuffer.dispose();
    assert!(!any, "nothing is left drawn");
}
