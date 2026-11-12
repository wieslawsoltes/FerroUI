//! The fragments of the Skia drawing context (`fragments.rs`): a drawing kept as a picture gives the pixels its
//! commands give, in the frame it is made in and in the frames after, where it was made and moved by whole
//! pixels; anywhere else, and in another state of the context, the context refuses it; and what is not drawn
//! the same again is not kept.

use super::placement;
use crate::drawing_context_impl::set_coverage_clips_on_raster;
use crate::helpers::drawing_context_helper::wrap_skia_surface;
use crate::{DrawingContextImpl, ISkiaApiLeaseFeature, ImmutableBitmap, PictureRenderTarget, WriteableBitmapImpl};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{BoxShadows, Color};
use ferroui_base::platform::{
    AlphaFormat, FragmentKeeping, IBitmapImpl, IDrawingContextImpl, IDrawingContextImplWithFragments,
    IDrawingFragment, PixelFormat,
};
use ferroui_base::{CornerRadius, Matrix, PixelSize, Rect, RoundedRect, Size, Vector};
use skia_safe as sk;
use std::any::TypeId;
use std::rc::Rc;

const DPI: Vector = Vector::new(96.0, 96.0);
/// The side of the frames, in pixels.
const SIDE: i32 = 64;

fn solid(color: u32) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(Color::from_uint32(color))
}

fn fill(context: &mut DrawingContextImpl, color: u32, bounds: Rect) {
    context.draw_rectangle(Some(&solid(color)), None, RoundedRect::from_rect(bounds), &BoxShadows::default());
}

/// A mark drawn after a drawing: it is where the transform of the context puts it.
fn mark(context: &mut DrawingContextImpl) {
    fill(context, 0xff00_0000, Rect::new(40.0, 40.0, 6.5, 6.5));
}

/// A drawing with what the content of a visual has: fills, a rounded shape with antialiased edges under a
/// clip, a translucent part and an ellipse. Its pushes and pops are balanced.
fn scene(context: &mut DrawingContextImpl) {
    fill(context, 0xff20_60c0, Rect::new(2.0, 3.0, 30.0, 12.0));
    context.push_clip(Rect::new(4.0, 4.0, 20.0, 20.0));
    let rounded = RoundedRect::from_corner_radius(Rect::new(0.5, 0.5, 28.5, 22.0), CornerRadius::uniform(5.0));
    context.draw_rectangle(Some(&solid(0xffe0_4020)), None, rounded, &BoxShadows::default());
    context.pop_clip();
    context.push_opacity(0.5, None);
    fill(context, 0xff10_a040, Rect::new(10.0, 18.0, 25.0, 9.0));
    context.pop_opacity();
    context.draw_ellipse(Some(&solid(0xff80_20a0)), None, Rect::new(30.0, 20.0, 14.0, 10.0));
}

/// The pixels of the frame `draw` draws into a context that records, over white.
fn pixels_of(draw: impl FnOnce(&mut DrawingContextImpl)) -> Vec<u8> {
    let target = PictureRenderTarget::new(None, None, DPI);
    let mut context = target.create_drawing_context(Size::new(f64::from(SIDE), f64::from(SIDE)), false);
    draw(&mut context);
    context.dispose();
    let picture = target.get_picture();
    let mut surface = sk::surfaces::raster_n32_premul((SIDE, SIDE)).unwrap();
    surface.canvas().clear(sk::Color::WHITE);
    surface.canvas().draw_picture(&picture, None, None);
    let info = sk::ImageInfo::new((SIDE, SIDE), sk::ColorType::RGBA8888, sk::AlphaType::Premul, None);
    let mut pixels = vec![0u8; (SIDE * SIDE * 4) as usize];
    assert!(surface.read_pixels(&info, &mut pixels, (SIDE * 4) as usize, (0, 0)));
    pixels
}

/// The pixels of `draw` drawn with the transform `transform`, and the mark after it.
fn drawn_at(transform: Matrix, draw: impl FnOnce(&mut DrawingContextImpl)) -> Vec<u8> {
    pixels_of(|context| {
        context.set_transform(transform);
        draw(context);
        mark(context);
    })
}

/// The fragment the context makes of `draw` with the transform `transform`, and the pixels of the frame it was
/// made in (with the mark after the drawing).
fn fragment_at(
    transform: Matrix,
    draw: impl FnOnce(&mut DrawingContextImpl),
) -> (Option<Rc<dyn IDrawingFragment>>, Vec<u8>) {
    let mut made = None;
    let pixels = drawn_at(transform, |context| {
        context.begin_fragment();
        draw(context);
        made = context.end_fragment();
        // The context has the transform it had.
        assert!(context.transform() == transform, "the transform after a fragment was made");
    });
    (made, pixels)
}

/// The pixels of a frame that draws `fragment` with the transform `transform`, and the mark after it; `None`
/// when the context refuses the fragment (and then draws nothing of it).
fn kept_at(transform: Matrix, fragment: &Rc<dyn IDrawingFragment>) -> Option<Vec<u8>> {
    let mut drawn = false;
    let pixels = drawn_at(transform, |context| {
        drawn = context.draw_fragment(fragment);
        assert!(context.transform() == transform, "the transform after a fragment was drawn");
    });
    let nothing = drawn_at(transform, |_| {});
    assert!(drawn || pixels == nothing, "a fragment that was refused drew something");
    drawn.then_some(pixels)
}

fn translation(x: f64, y: f64) -> Matrix {
    Matrix::create_translation(x, y)
}

#[test]
fn a_fragment_is_what_its_commands_draw() {
    let origin = translation(5.0, 7.0);
    let direct = drawn_at(origin, scene);
    let (fragment, making) = fragment_at(origin, scene);
    assert!(making == direct, "the frame the fragment was made in");
    let fragment = fragment.expect("the drawing is kept");
    assert!(kept_at(origin, &fragment).expect("drawn where it was made") == direct, "the frames after");
    // More than once in one frame, as two visuals with one content.
    let twice = pixels_of(|context| {
        for origin in [origin, translation(9.0, 30.0)] {
            context.set_transform(origin);
            assert!(context.draw_fragment(&fragment));
        }
    });
    let direct_twice = pixels_of(|context| {
        for origin in [origin, translation(9.0, 30.0)] {
            context.set_transform(origin);
            scene(context);
        }
    });
    assert!(twice == direct_twice, "the fragment drawn twice in one frame");
}

#[test]
fn a_fragment_is_drawn_where_its_drawing_moved_to_by_whole_pixels() {
    // Made at a fraction of a pixel: the move, not the place, is in whole pixels.
    let (fragment, _) = fragment_at(translation(5.25, 7.5), scene);
    let fragment = fragment.expect("the drawing is kept");
    for (x, y) in [(12.25, 3.5), (0.25, 20.5), (-3.75, 7.5)] {
        let moved = kept_at(translation(x, y), &fragment).expect("moved by whole pixels");
        assert!(moved == drawn_at(translation(x, y), scene), "moved to {x}, {y}");
    }
    // Under a scale too, when only the place differs.
    let scaled = |x: f64, y: f64| Matrix::create_scale(1.5, 1.5) * translation(x, y);
    let (fragment, _) = fragment_at(scaled(4.0, 2.0), scene);
    let fragment = fragment.expect("the drawing is kept");
    let moved = kept_at(scaled(1.0, 9.0), &fragment).expect("moved by whole pixels under a scale");
    assert!(moved == drawn_at(scaled(1.0, 9.0), scene), "moved under a scale");
}

#[test]
fn a_fragment_is_refused_where_its_pixels_would_differ() {
    let (fragment, _) = fragment_at(translation(5.0, 7.0), scene);
    let fragment = fragment.expect("the drawing is kept");
    // A move by a fraction of a pixel: the edges fall on the pixels in another way.
    assert!(kept_at(translation(5.5, 7.0), &fragment).is_none());
    assert!(kept_at(translation(5.0, 7.125), &fragment).is_none());
    // Another scale, a rotation.
    assert!(kept_at(Matrix::create_scale(2.0, 2.0) * translation(5.0, 7.0), &fragment).is_none());
    assert!(kept_at(Matrix::create_rotation(0.25) * translation(5.0, 7.0), &fragment).is_none());
    // Another opacity of the context: the fills of the fragment have the one it was made with.
    let translucent = |fragment: &Rc<dyn IDrawingFragment>| {
        let mut drawn = false;
        let pixels = pixels_of(|context| {
            context.push_opacity(0.5, None);
            context.set_transform(translation(5.0, 7.0));
            drawn = context.draw_fragment(fragment);
            context.pop_opacity();
        });
        drawn.then_some(pixels)
    };
    assert!(translucent(&fragment).is_none());
    // A fragment made with that opacity is drawn with it.
    let mut made = None;
    let direct = pixels_of(|context| {
        context.push_opacity(0.5, None);
        context.set_transform(translation(5.0, 7.0));
        context.begin_fragment();
        scene(context);
        made = context.end_fragment();
        context.pop_opacity();
    });
    let made = made.expect("the drawing is kept");
    assert!(translucent(&made).expect("drawn with the opacity it was made with") == direct);
    assert!(kept_at(translation(5.0, 7.0), &made).is_none());
}

#[test]
fn a_fragment_laid_out_in_the_pixels_of_the_device_stays_where_it_is() {
    // The clip of a rounded rectangle with corners of its own is a coverage laid out in the pixels of the
    // device (on a raster target only when the tests ask for it).
    let before = set_coverage_clips_on_raster(true);
    let clipped = |context: &mut DrawingContextImpl| {
        let corners = CornerRadius::new(2.0, 6.0, 9.0, 4.0);
        context.push_clip_rounded(RoundedRect::from_corner_radius(Rect::new(3.0, 3.0, 30.0, 24.0), corners));
        scene(context);
        context.pop_clip();
    };
    let origin = translation(5.0, 7.0);
    let direct = drawn_at(origin, clipped);
    let (fragment, making) = fragment_at(origin, clipped);
    let here = fragment.as_ref().map(|fragment| kept_at(origin, fragment));
    let moved = fragment.as_ref().map(|fragment| kept_at(translation(9.0, 7.0), fragment));
    set_coverage_clips_on_raster(before);
    assert!(making == direct, "the frame the fragment was made in");
    assert!(here.expect("the drawing is kept").expect("drawn where it was made") == direct);
    assert!(moved.expect("the drawing is kept").is_none(), "a fragment that cannot be moved was moved");
}

#[test]
fn what_is_not_drawn_the_same_again_is_not_kept() {
    let origin = translation(5.0, 7.0);
    let written = WriteableBitmapImpl::new(PixelSize::new(4, 4), DPI, PixelFormat::RGBA8888, AlphaFormat::Premul);
    let grey = [0x80_u8; 64];
    let fixed =
        ImmutableBitmap::from_pixels(PixelSize::new(4, 4), DPI, 16, PixelFormat::RGBA8888, AlphaFormat::Premul, &grey)
            .expect("a bitmap");
    let bitmap = |context: &mut DrawingContextImpl, bitmap: &dyn IBitmapImpl| {
        scene(context);
        context.draw_bitmap(bitmap, 1.0, Rect::new(0.0, 0.0, 4.0, 4.0), Rect::new(8.0, 30.0, 16.0, 16.0));
    };
    // A bitmap that cannot change is kept, one that is written to is not.
    let (fragment, making) = fragment_at(origin, |context| bitmap(context, &fixed));
    assert!(making == drawn_at(origin, |context| bitmap(context, &fixed)));
    let kept = kept_at(origin, &fragment.expect("a bitmap that cannot change is kept"));
    assert!(kept.expect("drawn where it was made") == making);
    let (fragment, making) = fragment_at(origin, |context| bitmap(context, &*written));
    assert!(making == drawn_at(origin, |context| bitmap(context, &*written)));
    assert!(fragment.is_none(), "a bitmap that is written to was kept");

    // What is drawn with the canvas itself.
    let with_canvas = |context: &mut DrawingContextImpl| {
        scene(context);
        let paint = sk::Paint::new(sk::Color4f::new(0.2, 0.4, 0.6, 1.0), None);
        context.canvas().draw_rect(sk::Rect::from_xywh(3.0, 30.0, 12.0, 9.0), &paint);
    };
    let (fragment, making) = fragment_at(origin, with_canvas);
    assert!(making == drawn_at(origin, with_canvas));
    assert!(fragment.is_none(), "a drawing with the canvas itself was kept");

    // A drawing that asks for the Skia API.
    let with_lease = |context: &mut DrawingContextImpl| {
        scene(context);
        let _ = context.get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>());
    };
    let (fragment, making) = fragment_at(origin, with_lease);
    assert!(making == drawn_at(origin, with_lease));
    assert!(fragment.is_none(), "a drawing that asked for the Skia API was kept");
}

#[test]
fn a_pop_of_a_push_from_before_the_fragment_ends_it() {
    // The contract has balanced drawings only; one that pops what was pushed before it is drawn as it is
    // without a fragment, and nothing of it is kept.
    let clip = Rect::new(0.0, 0.0, 18.0, 40.0);
    let direct = pixels_of(|context| {
        context.set_transform(translation(5.0, 7.0));
        context.push_clip(clip);
        scene(context);
        context.pop_clip();
        mark(context);
        fill(context, 0xff40_4040, Rect::new(0.0, 44.0, 50.0, 4.0));
    });
    let mut made = None;
    let captured = pixels_of(|context| {
        context.set_transform(translation(5.0, 7.0));
        context.push_clip(clip);
        context.begin_fragment();
        scene(context);
        context.pop_clip();
        mark(context);
        made = context.end_fragment();
        assert!(context.transform() == translation(5.0, 7.0));
        fill(context, 0xff40_4040, Rect::new(0.0, 44.0, 50.0, 4.0));
    });
    assert!(captured == direct);
    assert!(made.is_none(), "a drawing that popped a push from before it was kept");
}

#[test]
fn a_push_left_open_is_not_kept() {
    let mut made = None;
    pixels_of(|context| {
        context.set_transform(translation(5.0, 7.0));
        context.begin_fragment();
        context.push_clip(Rect::new(0.0, 0.0, 18.0, 40.0));
        scene(context);
        made = context.end_fragment();
        // The push is one of the context still.
        context.pop_clip();
        mark(context);
    });
    assert!(made.is_none(), "a drawing that left a push open was kept");
}

#[test]
fn only_a_context_that_records_makes_fragments() {
    let target = PictureRenderTarget::new(None, None, DPI);
    let mut recording = target.create_drawing_context(Size::new(f64::from(SIDE), f64::from(SIDE)), false);
    assert_eq!(recording.fragment_keeping(), FragmentKeeping::Settled);
    recording.dispose();

    let mut surface = sk::surfaces::raster_n32_premul((SIDE, SIDE)).unwrap();
    surface.canvas().clear(sk::Color::WHITE);
    let (fragment, _) = fragment_at(translation(5.0, 7.0), scene);
    let fragment = fragment.expect("the drawing is kept");
    {
        let mut context = wrap_skia_surface(&surface, DPI);
        assert_eq!(context.fragment_keeping(), FragmentKeeping::None);
        context.set_transform(translation(5.0, 7.0));
        context.begin_fragment();
        scene(&mut context);
        assert!(context.end_fragment().is_none(), "a context that draws to a surface made a fragment");
        // It draws the fragments that were made where it can: a picture is what its commands draw there too.
        context.set_transform(translation(5.0, 37.0));
        assert!(context.draw_fragment(&fragment));
        mark(&mut context);
        context.dispose();
    }
    let info = sk::ImageInfo::new((SIDE, SIDE), sk::ColorType::RGBA8888, sk::AlphaType::Premul, None);
    let mut pixels = vec![0u8; (SIDE * SIDE * 4) as usize];
    assert!(surface.read_pixels(&info, &mut pixels, (SIDE * 4) as usize, (0, 0)));
    let mut direct = sk::surfaces::raster_n32_premul((SIDE, SIDE)).unwrap();
    direct.canvas().clear(sk::Color::WHITE);
    {
        let mut context = wrap_skia_surface(&direct, DPI);
        for y in [7.0, 37.0] {
            context.set_transform(translation(5.0, y));
            scene(&mut context);
        }
        mark(&mut context);
        context.dispose();
    }
    let mut expected = vec![0u8; (SIDE * SIDE * 4) as usize];
    assert!(direct.read_pixels(&info, &mut expected, (SIDE * 4) as usize, (0, 0)));
    assert!(pixels == expected, "a surface drawn to with and without fragments");
}

#[test]
fn a_picture_is_placed_by_a_move_of_whole_pixels_only() {
    let at = |x: f32, y: f32| sk::M44::translate(x, y, 0.0);
    let scaled = |x: f32, y: f32, scale: f32| sk::M44::concat(&at(x, y), &sk::M44::scale(scale, scale, 1.0));
    // Where it was made, movable or not.
    assert!(placement(&at(3.0, 4.0), &at(3.0, 4.0), false) == Some(sk::M44::new_identity()));
    assert!(placement(&scaled(3.5, 4.0, 2.0), &scaled(3.5, 4.0, 2.0), false) == Some(sk::M44::new_identity()));
    // Moved by whole pixels, from a place at a fraction of one too.
    assert!(placement(&at(3.5, 4.25), &at(10.5, 1.25), true) == Some(at(7.0, -3.0)));
    assert!(placement(&scaled(3.5, 4.0, 2.0), &scaled(-1.5, 9.0, 2.0), true) == Some(at(-5.0, 5.0)));
    // Not when it cannot be moved, by a fraction of a pixel, or with another matrix apart from the place.
    assert!(placement(&at(3.0, 4.0), &at(10.0, 1.0), false).is_none());
    assert!(placement(&at(3.0, 4.0), &at(3.5, 4.0), true).is_none());
    assert!(placement(&at(3.0, 4.0), &at(3.0, 4.75), true).is_none());
    assert!(placement(&scaled(3.0, 4.0, 2.0), &scaled(5.0, 4.0, 2.5), true).is_none());
    assert!(placement(&at(3.0, 4.0), &at(f32::NAN, 4.0), true).is_none());
    // Not under a perspective, which a move after it does not commute with.
    let perspective = |x: f32| {
        sk::M44::row_major(&[1.0, 0.0, 0.0, x, 0.0, 1.0, 0.0, 4.0, 0.0, 0.0, 1.0, 0.0, 0.001, 0.0, 0.0, 1.0])
    };
    assert!(placement(&perspective(3.0), &perspective(8.0), true).is_none());
}
