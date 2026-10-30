//! Tests of the fragments of the GPU mode: the content of a visual drawn
//! again from what its scene kept gives the pixels its commands give, at
//! the place it was made at moved by whole pixels, and is refused anywhere
//! else.
//!
//! Skipped with a message on a machine without an adapter that runs the
//! compute shaders of the mode, unless `FERROUI_VELLO_REQUIRE_GPU` demands
//! one (the same opt-in as the other GPU tests).

use crate::drawing_context_impl::{CreateInfo, DrawingContextImpl};
use crate::gpu::VelloWgpuDevice;
use crate::scene::{IVelloSceneSink, VelloScenePaint};
use crate::vello_options::VelloRenderingMode;
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadows, Color, PenLineCap, PenLineJoin};
use ferroui_base::platform::{IDrawingContextImpl, IDrawingContextImplWithFragments, IDrawingFragment};
use ferroui_base::{Matrix, Rect, RoundedRect, Vector};
use kurbo::{Affine, Shape};
use peniko::{BlendMode, Compose, Fill, Mix};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const SIZE: u16 = 96;

fn gpu_device(test: &str) -> Option<Arc<VelloWgpuDevice>> {
    let required = std::env::var("FERROUI_VELLO_REQUIRE_GPU")
        .is_ok_and(|value| !value.is_empty() && value != "0");
    if !cfg!(feature = "gpu") {
        assert!(!required, "FERROUI_VELLO_REQUIRE_GPU is set but the gpu feature is disabled ({test})");
        return None;
    }
    match VelloWgpuDevice::headless() {
        Ok(device) if device.supports_compute() => Some(device),
        Ok(_) => {
            assert!(!required, "FERROUI_VELLO_REQUIRE_GPU is set but the adapter has no compute shaders ({test})");
            println!("skipped: the adapter has no compute shaders ({test})");
            None
        }
        Err(error) => {
            assert!(!required, "FERROUI_VELLO_REQUIRE_GPU is set but no adapter is available ({test}): {error}");
            println!("skipped: no adapter ({test}): {error}");
            None
        }
    }
}

/// A drawing context of the GPU mode whose scene is rendered into `pixels`
/// when it is disposed.
fn gpu_context(
    device: &Arc<VelloWgpuDevice>, pixels: &Rc<RefCell<Vec<u8>>>, width: u16, height: u16,
) -> DrawingContextImpl {
    let sink = crate::gpu::create_window_scene_sink(
        device,
        &[VelloRenderingMode::Gpu],
        width,
        height,
        wgpu::TextureFormat::Rgba8Unorm,
    );
    let pixels = pixels.clone();
    DrawingContextImpl::new(
        CreateInfo {
            sink,
            backdrop: None,
            on_finished: Box::new(move |sink| sink.render_to_pixels(&mut pixels.borrow_mut())),
            scale_drawing_to_dpi: false,
            dpi: Vector::new(96.0, 96.0),
            rendering_modes: vec![VelloRenderingMode::Gpu],
        },
        Vec::new(),
    )
}

/// The content of a visual: a rounded rectangle with a border, an ellipse
/// and a line, with curves and strokes the sink flattens and outlines.
fn content(context: &mut dyn IDrawingContextImpl) {
    let fill = ImmutableSolidColorBrush::new(Color::from_argb(255, 30, 120, 200));
    let border = ImmutableSolidColorBrush::new(Color::from_argb(200, 200, 40, 40));
    let pen = ImmutablePen::new(Some(Rc::new(border)), 3.0, None, PenLineCap::Round, PenLineJoin::Miter, 10.0);
    context.draw_rectangle(
        Some(&fill),
        Some(&pen),
        RoundedRect::from_radius(Rect::new(4.3, 5.7, 40.0, 30.0), 7.5),
        &BoxShadows::default(),
    );
    context.draw_ellipse(Some(&fill), Some(&pen), Rect::new(20.25, 30.5, 30.0, 20.0));
    context.draw_line(Some(&pen), ferroui_base::Point::new(2.0, 50.0), ferroui_base::Point::new(45.5, 12.25));
}

fn frame(device: &Arc<VelloWgpuDevice>, draw: impl FnOnce(&mut DrawingContextImpl)) -> Vec<u8> {
    frame_with_size(device, SIZE, SIZE, draw)
}

fn frame_with_size(
    device: &Arc<VelloWgpuDevice>, width: u16, height: u16, draw: impl FnOnce(&mut DrawingContextImpl),
) -> Vec<u8> {
    let pixels = Rc::new(RefCell::new(vec![0u8; usize::from(width) * usize::from(height) * 4]));
    let mut context = gpu_context(device, &pixels, width, height);
    draw(&mut context);
    context.dispose();
    let pixels = pixels.borrow().clone();
    pixels
}

fn fragments(context: &mut DrawingContextImpl) -> &mut dyn IDrawingContextImplWithFragments {
    context.as_drawing_context_impl_with_fragments().expect("the GPU mode keeps fragments")
}

fn largest_difference(a: &[u8], b: &[u8]) -> u8 {
    assert_eq!(a.len(), b.len(), "the compared frames have identical dimensions");
    a.iter().zip(b).map(|(a, b)| a.abs_diff(*b)).max().unwrap_or(0)
}

fn pixel(pixels: &[u8], width: u16, x: usize, y: usize) -> [u8; 4] {
    let index = (y * usize::from(width) + x) * 4;
    pixels[index..index + 4].try_into().unwrap()
}

fn assert_pixel(pixels: &[u8], width: u16, x: usize, y: usize, expected: [u8; 4]) {
    let actual = pixel(pixels, width, x, y);
    assert!(actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
        "pixel ({x}, {y}) is {actual:?}, expected {expected:?}");
}

/// Makes the fragment of the content at `transform`.
fn make_fragment(device: &Arc<VelloWgpuDevice>, transform: Matrix) -> (Rc<dyn IDrawingFragment>, Vec<u8>) {
    let mut kept = None;
    let pixels = frame(device, |context| {
        context.set_transform(transform);
        fragments(context).begin_fragment();
        content(context);
        kept = fragments(context).end_fragment();
    });
    (kept.expect("the content is kept as a fragment"), pixels)
}

#[test]
fn a_fragment_gives_the_pixels_of_its_commands() {
    let Some(device) = gpu_device("a_fragment_gives_the_pixels_of_its_commands") else { return };
    let transform = Matrix::create_translation(3.0, 2.0);

    let direct = frame(&device, |context| {
        context.set_transform(transform);
        content(context);
    });
    let (fragment, recorded) = make_fragment(&device, transform);
    let drawn_again = frame(&device, |context| {
        context.set_transform(transform);
        assert!(fragments(context).draw_fragment(&fragment));
    });

    assert!(direct.iter().any(|value| *value != 0), "the content is drawn");
    assert_eq!(largest_difference(&direct, &recorded), 0, "making a fragment draws what the commands draw");
    assert_eq!(largest_difference(&direct, &drawn_again), 0, "a fragment drawn again draws the same pixels");
}

#[test]
fn a_fragment_moved_by_whole_pixels_gives_the_pixels_of_its_commands_there() {
    let Some(device) = gpu_device("a_fragment_moved_by_whole_pixels_gives_the_pixels_of_its_commands_there") else {
        return;
    };
    // Twice the scale, as on a display of two pixels a unit: a move of
    // 13.5 by -3.5 units is one of 27 by -7 pixels.
    let made_at = Matrix::create_translation(1.0, 0.5) * Matrix::create_scale(2.0, 2.0);
    let moved_to = Matrix::create_translation(14.5, -3.0) * Matrix::create_scale(2.0, 2.0);

    let (fragment, _) = make_fragment(&device, made_at);
    let direct = frame(&device, |context| {
        context.set_transform(moved_to);
        content(context);
    });
    let drawn_again = frame(&device, |context| {
        context.set_transform(moved_to);
        assert!(fragments(context).draw_fragment(&fragment));
    });

    assert!(largest_difference(&direct, &drawn_again) <= 1, "the moved fragment draws what the commands draw there");
}

#[test]
fn a_fragment_is_refused_where_its_pixels_would_differ() {
    let Some(device) = gpu_device("a_fragment_is_refused_where_its_pixels_would_differ") else { return };
    let (fragment, _) = make_fragment(&device, Matrix::create_translation(3.0, 2.0));

    let empty = vec![0u8; usize::from(SIZE) * usize::from(SIZE) * 4];
    for (case, refused) in [
        ("a move by part of a pixel", Box::new(|context: &mut DrawingContextImpl| {
            context.set_transform(Matrix::create_translation(3.5, 2.0));
        }) as Box<dyn Fn(&mut DrawingContextImpl)>),
        ("another scale", Box::new(|context: &mut DrawingContextImpl| {
            context.set_transform(Matrix::create_scale(1.5, 1.5));
        })),
        ("another opacity", Box::new(|context: &mut DrawingContextImpl| {
            context.set_transform(Matrix::create_translation(3.0, 2.0));
            context.push_opacity(0.5, None);
        })),
    ] {
        let pixels = frame(&device, |context| {
            refused(context);
            assert!(!fragments(context).draw_fragment(&fragment), "{case}: refused");
        });
        assert_eq!(pixels, empty, "{case}: nothing is drawn");
    }
}

fn rectangle(context: &mut dyn IDrawingContextImpl, bounds: Rect, color: Color) {
    context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::new(color)), None, RoundedRect::from_rect(bounds), &BoxShadows::default(),
    );
}

fn target_bounded_layer_content(context: &mut dyn IDrawingContextImpl, compositing: bool) {
    // Content recorded before the layer must survive abandoning capture.
    rectangle(context, Rect::new(44.0, 4.0, 8.0, 8.0), Color::from_rgb(0, 0, 255));
    let bounds = Rect::new(-20.0, 8.0, 150.0, 48.0);
    if compositing {
        assert!(context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, 0.5, bounds));
    } else {
        context.push_layer(bounds);
    }
    rectangle(context, Rect::new(-20.0, 8.0, 40.0, 16.0), Color::from_rgb(255, 0, 0));
    rectangle(context, Rect::new(80.0, 40.0, 50.0, 16.0), Color::from_rgb(0, 255, 0));
    if compositing {
        context.pop_compositing_layer();
    } else {
        context.pop_layer();
    }
}

#[test]
fn target_bounded_layers_redraw_after_fragment_moves_and_target_resizes() {
    let Some(device) = gpu_device("target_bounded_layers_redraw_after_fragment_moves_and_target_resizes") else {
        return;
    };
    for compositing in [false, true] {
        let mut kept = None;
        let recorded = frame(&device, |context| {
            fragments(context).begin_fragment();
            target_bounded_layer_content(context, compositing);
            kept = fragments(context).end_fragment();
        });
        let direct = frame(&device, |context| target_bounded_layer_content(context, compositing));
        assert_eq!(largest_difference(&direct, &recorded), 0, "capture preserves the first layer draw");
        assert_pixel(&recorded, SIZE, 48, 8, [0, 0, 255, 255]);

        for (width, transform, probe, color) in [
            (SIZE, Matrix::create_translation(20.0, 0.0), (5, 12), [255, 0, 0, 255]),
            (144, Matrix::IDENTITY, (120, 48), [0, 255, 0, 255]),
        ] {
            let direct = frame_with_size(&device, width, SIZE, |context| {
                context.set_transform(transform);
                target_bounded_layer_content(context, compositing);
            });
            let replayed = frame_with_size(&device, width, SIZE, |context| {
                context.set_transform(transform);
                if !kept.as_ref().is_some_and(|fragment| fragments(context).draw_fragment(fragment)) {
                    target_bounded_layer_content(context, compositing);
                }
            });
            assert_eq!(largest_difference(&direct, &replayed), 0,
                "a moved or resized target must not reuse the old target's layer clip");
            let expected = if compositing { color.map(|value| if value == 255 { 128 } else { 0 }) } else { color };
            assert_pixel(&replayed, width, probe.0, probe.1, expected);
        }
        assert!(kept.is_none(), "target-bounded layers conservatively decline fragment capture");
    }
}

fn clipped_compositing_content(context: &mut DrawingContextImpl, capture: bool) {
    rectangle(context, Rect::new(0.0, 0.0, 96.0, 96.0), Color::from_rgb(200, 100, 50));
    context.push_clip(Rect::new(8.0, 8.0, 48.0, 48.0));
    context.push_clip(Rect::new(12.0, 12.0, 24.0, 24.0));
    if capture {
        fragments(context).begin_fragment();
    }
    rectangle(context, Rect::new(14.0, 14.0, 4.0, 4.0), Color::from_rgb(255, 255, 255));
    assert!(context.try_push_compositing_layer(
        BitmapBlendingMode::Multiply, 1.0, Rect::new(20.0, 20.0, 44.0, 44.0),
    ));
    rectangle(context, Rect::new(20.0, 20.0, 44.0, 44.0), Color::from_rgb(128, 255, 255));
    context.pop_compositing_layer();
    if capture {
        assert!(fragments(context).end_fragment().is_none(), "the blended layer abandons capture");
    }
    context.pop_clip();
    rectangle(context, Rect::new(42.0, 20.0, 6.0, 6.0), Color::from_rgb(20, 180, 40));
    context.pop_clip();
    rectangle(context, Rect::new(72.0, 8.0, 12.0, 12.0), Color::from_rgb(0, 0, 255));
}

#[test]
fn fragment_compositing_preserves_outer_clips_and_following_draws() {
    let Some(device) = gpu_device("fragment_compositing_preserves_outer_clips_and_following_draws") else {
        return;
    };
    let direct = frame(&device, |context| clipped_compositing_content(context, false));
    let captured = frame(&device, |context| clipped_compositing_content(context, true));
    assert_eq!(largest_difference(&direct, &captured), 0, "the first captured draw has the same clip and blend scope");
    assert_pixel(&captured, SIZE, 28, 28, [100, 100, 50, 255]);
    assert_pixel(&captured, SIZE, 16, 16, [255, 255, 255, 255]);
    assert_pixel(&captured, SIZE, 10, 20, [200, 100, 50, 255]);
    assert_pixel(&captured, SIZE, 44, 22, [20, 180, 40, 255]);
    assert_pixel(&captured, SIZE, 76, 12, [0, 0, 255, 255]);
}

fn scene_rectangle(sink: &mut dyn IVelloSceneSink, bounds: kurbo::Rect, rgba: [u8; 4], blend: BlendMode) {
    let [red, green, blue, alpha] = rgba;
    sink.fill_rect(
        bounds, Affine::IDENTITY,
        &VelloScenePaint::solid(peniko::Color::from_rgba8(red, green, blue, alpha)), blend, false,
    );
}

fn clipped_fill_frame(device: &Arc<VelloWgpuDevice>, capture: bool, blend: BlendMode) -> Vec<u8> {
    let mut sink = crate::gpu::create_window_scene_sink(
        device, &[VelloRenderingMode::Gpu], SIZE, SIZE, wgpu::TextureFormat::Rgba8Unorm,
    );
    scene_rectangle(&mut *sink, kurbo::Rect::new(0.0, 0.0, 96.0, 96.0), [200, 100, 50, 255], BlendMode::default());
    sink.push_clip(&kurbo::Rect::new(8.0, 8.0, 56.0, 56.0).to_path(0.1), Fill::NonZero, Affine::IDENTITY, false);
    sink.push_clip(&kurbo::Rect::new(12.0, 12.0, 36.0, 36.0).to_path(0.1), Fill::NonZero, Affine::IDENTITY, false);
    if capture {
        assert!(sink.begin_fragment(Affine::IDENTITY));
    }
    scene_rectangle(&mut *sink, kurbo::Rect::new(14.0, 14.0, 18.0, 18.0), [255, 255, 255, 255], BlendMode::default());
    scene_rectangle(&mut *sink, kurbo::Rect::new(20.0, 20.0, 64.0, 64.0), [128, 255, 255, 255], blend);
    if capture {
        assert!(sink.end_fragment().is_none(), "a non-default fill abandons capture before closing outer clips");
    }
    sink.pop_clip();
    scene_rectangle(&mut *sink, kurbo::Rect::new(42.0, 20.0, 48.0, 26.0), [20, 180, 40, 255], BlendMode::default());
    sink.pop_clip();
    scene_rectangle(&mut *sink, kurbo::Rect::new(72.0, 8.0, 84.0, 20.0), [0, 0, 255, 255], BlendMode::default());
    let mut pixels = vec![0; usize::from(SIZE) * usize::from(SIZE) * 4];
    sink.render_to_pixels(&mut pixels);
    pixels
}

#[test]
fn fragment_blended_fills_preserve_outer_clips_and_following_draws() {
    let Some(device) = gpu_device("fragment_blended_fills_preserve_outer_clips_and_following_draws") else {
        return;
    };
    for (blend, expected) in [
        (BlendMode::new(Mix::Multiply, Compose::SrcOver), [100, 100, 50, 255]),
        (BlendMode::new(Mix::Normal, Compose::Copy), [128, 255, 255, 255]),
        (BlendMode::new(Mix::Normal, Compose::Clear), [0, 0, 0, 0]),
    ] {
        let direct = clipped_fill_frame(&device, false, blend);
        let captured = clipped_fill_frame(&device, true, blend);
        assert_eq!(largest_difference(&direct, &captured), 0, "capture must not change {blend:?} or leak a clip");
        assert_pixel(&captured, SIZE, 28, 28, expected);
        assert_pixel(&captured, SIZE, 16, 16, [255, 255, 255, 255]);
        assert_pixel(&captured, SIZE, 10, 20, [200, 100, 50, 255]);
        assert_pixel(&captured, SIZE, 44, 22, [20, 180, 40, 255]);
        assert_pixel(&captured, SIZE, 76, 12, [0, 0, 255, 255]);
    }
}

#[test]
fn ordinary_content_inside_an_outer_layer_keeps_reusable_fragments() {
    let Some(device) = gpu_device("ordinary_content_inside_an_outer_layer_keeps_reusable_fragments") else {
        return;
    };
    let bounds = Rect::new(0.0, 0.0, f64::from(SIZE), f64::from(SIZE));
    let mut kept = None;
    let captured = frame(&device, |context| {
        assert!(context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, 0.5, bounds));
        fragments(context).begin_fragment();
        content(context);
        kept = fragments(context).end_fragment();
        context.pop_compositing_layer();
    });
    let kept = kept.expect("ordinary content is reusable when its outer layer is not part of the fragment");
    let replayed = frame(&device, |context| {
        assert!(context.try_push_compositing_layer(BitmapBlendingMode::SourceOver, 0.5, bounds));
        assert!(fragments(context).draw_fragment(&kept));
        context.pop_compositing_layer();
    });
    assert!(captured.iter().any(|value| *value != 0));
    assert_eq!(largest_difference(&captured, &replayed), 0, "replaying ordinary content preserves outer opacity");
}
