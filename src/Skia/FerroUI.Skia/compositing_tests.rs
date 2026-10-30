//! Pixel regressions for the public isolated group operation, including its
//! recorded form. These use Skia's raster surface through FerroUI's public API.

use crate::{PlatformRenderInterface, SkiaOptions};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::media::immutable::{ImmutableImageBrush, ImmutableSolidColorBrush};
use ferroui_base::media::{
    Color, Colors, DrawingContext, DrawingGroup, IBrush, ISceneBrushContent, ITileBrush,
    PlatformDrawingContext,
};
use ferroui_base::platform::{IPlatformRenderInterface, IRenderTargetBitmapImpl, PixelFormat};
use ferroui_base::rendering::composition::drawing::RenderDataDrawingContext;
use ferroui_base::{FerroLocator, Matrix, PixelSize, Rect, Vector};
use std::rc::Rc;
use std::sync::Arc;
#[test]
fn compositing_layer_luminance_to_alpha_preserves_transparency_and_recorded_state() {
    use ferroui_base::media::{IEffect, ImmutableLuminanceToAlphaEffect};
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(PlatformRenderInterface::default()));
    let draw = |context: &mut DrawingContext<'_>| {
        let effect: Rc<dyn IEffect> = Rc::new(ImmutableLuminanceToAlphaEffect);
        let opacity = context
            .try_push_compositing_layer(
                BitmapBlendingMode::SourceOver,
                0.5,
                Rect::new(0.0, 0.0, 100.0, 70.0),
            )
            .unwrap();
        let state = context.push_effect(&effect, Rect::new(0.0, 0.0, 100.0, 70.0));
        for (index, color) in [
            Color::from_argb(128, 255, 255, 255),
            Colors::RED,
            Color::from_argb(128, 0, 255, 0),
            Colors::BLACK,
            Color::from_argb(64, 0, 0, 255),
        ]
        .into_iter()
        .enumerate()
        {
            context.fill_rectangle(
                &brush(color),
                Rect::new(index as f64 * 20.0, 0.0, 20.0, 70.0),
                0.0,
            );
        }
        context.pop(state);
        context.pop(opacity);
        context.fill_rectangle(
            &brush(Colors::YELLOW),
            Rect::new(0.0, 80.0, 100.0, 20.0),
            0.0,
        );
    };
    for save_layers in [false, true] {
        let direct = render(save_layers, draw);
        for (x, alpha) in [(10, 64), (30, 27), (50, 46), (70, 0), (90, 2)] {
            near(pixel(&*direct, x, 30), [0, 0, 0, alpha]);
        }
        near(pixel(&*direct, 50, 90), [255, 255, 0, 255]);
        let group = DrawingGroup::new();
        {
            let mut context = group.open();
            draw(&mut context);
        }
        let recorded = render(save_layers, |context| group.draw(context));
        for (x, y) in [(10, 30), (30, 30), (50, 30), (70, 30), (90, 30), (50, 90)] {
            near(pixel(&*recorded, x, y), pixel(&*direct, x, y));
        }
    }
    scope.dispose();
}

fn brush(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

fn render(
    save_opacity_layers: bool,
    draw: impl FnOnce(&mut DrawingContext<'_>),
) -> Arc<dyn IRenderTargetBitmapImpl> {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable().bind_to_self(Rc::new(SkiaOptions {
        use_opacity_save_layer: save_opacity_layers,
        ..SkiaOptions::default()
    }));
    let bitmap = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(100, 100), Vector::new(96.0, 96.0));
    {
        let mut implementation = bitmap.create_drawing_context();
        implementation.clear(Colors::TRANSPARENT);
        {
            let mut core = PlatformDrawingContext::borrowed(&mut *implementation);
            let mut context = DrawingContext::new(&mut core);
            draw(&mut context);
        }
        implementation.dispose();
    }
    scope.dispose();
    bitmap
}

fn pixel(bitmap: &dyn IRenderTargetBitmapImpl, x: usize, y: usize) -> [u8; 4] {
    let buffer = bitmap.as_readable_bitmap().unwrap().lock();
    let offset = y * buffer.row_bytes() as usize + x * 4;
    let mut rgba = [0; 4];
    buffer.with_data(&mut |data| rgba.copy_from_slice(&data[offset..offset + 4]));
    if buffer.format() == PixelFormat::BGRA8888 {
        rgba.swap(0, 2);
    }
    buffer.dispose();
    rgba
}

fn near(actual: [u8; 4], expected: [u8; 4]) {
    assert!(
        actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1),
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn compositing_layer_opacity_applies_once_to_overlapping_vectors_in_both_opacity_modes() {
    for save_layers in [false, true] {
        let bitmap = render(save_layers, |context| {
            let _group = context
                .try_push_compositing_layer(
                    BitmapBlendingMode::SourceOver,
                    0.5,
                    Rect::new(0.0, 0.0, 100.0, 100.0),
                )
                .unwrap();
            context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 60.0, 100.0), 0.0);
            context.fill_rectangle(&brush(Colors::RED), Rect::new(40.0, 0.0, 60.0, 100.0), 0.0);
            // The public context drop balances a still-open group.
        });
        for x in [20, 50, 80] {
            near(pixel(&*bitmap, x, 50), [128, 0, 0, 128]);
        }
    }
}

#[test]
fn compositing_layer_multiplies_the_completed_group_with_the_existing_backdrop() {
    let bitmap = render(false, |context| {
        context.fill_rectangle(
            &brush(Color::from_argb(255, 128, 128, 255)),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            0.0,
        );
        let group = context
            .try_push_compositing_layer(
                BitmapBlendingMode::Multiply,
                1.0,
                Rect::new(0.0, 0.0, 100.0, 100.0),
            )
            .unwrap();
        context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 60.0, 100.0), 0.0);
        context.fill_rectangle(&brush(Colors::BLUE), Rect::new(40.0, 0.0, 60.0, 100.0), 0.0);
        context.pop(group);
    });
    // Ignoring Multiply would leave red at 255. Blending each primitive
    // separately would turn the overlap black instead of the final blue.
    near(pixel(&*bitmap, 20, 50), [128, 0, 0, 255]);
    near(pixel(&*bitmap, 50, 50), [0, 0, 255, 255]);
    near(pixel(&*bitmap, 80, 50), [0, 0, 255, 255]);
}

#[test]
fn compositing_layer_nested_opacity_consumes_inherited_opacity_once_and_restores_siblings() {
    for save_layers in [false, true] {
        let bitmap = render(save_layers, |context| {
            let opacity = context.push_opacity(0.5);
            let outer = context
                .try_push_compositing_layer(
                    BitmapBlendingMode::SourceOver,
                    0.5,
                    Rect::new(0.0, 0.0, 60.0, 100.0),
                )
                .unwrap();
            let inner = context
                .try_push_compositing_layer(
                    BitmapBlendingMode::SourceOver,
                    0.5,
                    Rect::new(0.0, 0.0, 30.0, 100.0),
                )
                .unwrap();
            context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 30.0, 100.0), 0.0);
            context.pop(inner);
            context.fill_rectangle(&brush(Colors::RED), Rect::new(30.0, 0.0, 30.0, 100.0), 0.0);
            context.pop(outer);
            context.fill_rectangle(&brush(Colors::RED), Rect::new(60.0, 0.0, 20.0, 100.0), 0.0);
            context.pop(opacity);
            context.fill_rectangle(&brush(Colors::RED), Rect::new(80.0, 0.0, 20.0, 100.0), 0.0);
        });
        for (x, alpha) in [(15, 32), (45, 64), (70, 128), (90, 255)] {
            near(pixel(&*bitmap, x, 50), [alpha, 0, 0, alpha]);
        }
    }
}

#[test]
fn compositing_layer_advisory_bounds_do_not_clip_and_explicit_transform_clip_scopes_restore() {
    let bitmap = render(false, |context| {
        let transform = context.push_transform(Matrix::create_translation(20.0, 10.0));
        let clip = context.push_clip(Rect::new(0.0, 0.0, 20.0, 20.0));
        let group = context
            .try_push_compositing_layer(
                BitmapBlendingMode::SourceOver,
                1.0,
                Rect::new(50.0, 50.0, 1.0, 1.0),
            )
            .unwrap();
        context.fill_rectangle(&brush(Colors::BLUE), Rect::new(0.0, 0.0, 40.0, 40.0), 0.0);
        context.pop(group);
        context.pop(clip);
        context.pop(transform);
        context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 10.0, 10.0), 0.0);
    });
    assert_eq!(pixel(&*bitmap, 25, 15), [0, 0, 255, 255]);
    assert_eq!(pixel(&*bitmap, 45, 15), [0, 0, 0, 0]);
    assert_eq!(pixel(&*bitmap, 5, 5), [255, 0, 0, 255]);
}

#[test]
fn compositing_layer_public_recording_replays_identical_pixels() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(PlatformRenderInterface::default()));
    let scene = |context: &mut DrawingContext<'_>| {
        context.fill_rectangle(
            &brush(Colors::WHITE),
            Rect::new(0.0, 0.0, 100.0, 100.0),
            0.0,
        );
        let outer = context
            .try_push_compositing_layer(
                BitmapBlendingMode::Multiply,
                0.5,
                Rect::new(0.0, 0.0, 100.0, 100.0),
            )
            .unwrap();
        context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 60.0, 100.0), 0.0);
        context.fill_rectangle(&brush(Colors::BLUE), Rect::new(40.0, 0.0, 60.0, 100.0), 0.0);
        context.pop(outer);
    };
    let immediate = render(false, scene);
    let mut recorder = RenderDataDrawingContext::new(None);
    {
        let mut context = DrawingContext::new(&mut recorder);
        scene(&mut context);
    }
    let tile: Rc<dyn ITileBrush> = Rc::new(ImmutableImageBrush::from_bitmap(None));
    let content = recorder
        .get_immediate_scene_brush_content(tile, None, true)
        .unwrap();
    let recorded = PlatformRenderInterface::default()
        .create_render_target_bitmap(PixelSize::new(100, 100), Vector::new(96.0, 96.0));
    let mut implementation = recorded.create_drawing_context();
    implementation.clear(Colors::TRANSPARENT);
    content.render(&mut *implementation, None);
    implementation.dispose();
    for x in [20, 50, 80] {
        assert_eq!(pixel(&*recorded, x, 50), pixel(&*immediate, x, 50));
    }
    near(pixel(&*recorded, 20, 50), [255, 128, 128, 255]);
    near(pixel(&*recorded, 50, 50), [128, 128, 255, 255]);
    scope.dispose();
}

#[test]
fn compositing_layer_drawing_group_structural_wrappers_keep_the_external_backdrop() {
    let scope = FerroLocator::enter_scope();
    FerroLocator::current_mutable()
        .bind::<dyn IPlatformRenderInterface>()
        .to_constant(Rc::new(PlatformRenderInterface::default()));
    let group = DrawingGroup::new();
    {
        let mut context = group.open();
        let _layer = context
            .try_push_compositing_layer(
                BitmapBlendingMode::Multiply,
                1.0,
                Rect::new(0.0, 0.0, 100.0, 100.0),
            )
            .unwrap();
        context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 60.0, 100.0), 0.0);
        context.fill_rectangle(&brush(Colors::BLUE), Rect::new(40.0, 0.0, 60.0, 100.0), 0.0);
    }
    for save_layers in [false, true] {
        let bitmap = render(save_layers, |context| {
            context.fill_rectangle(
                &brush(Color::from_argb(255, 128, 128, 255)),
                Rect::new(0.0, 0.0, 100.0, 100.0),
                0.0,
            );
            group.draw(context);
        });
        near(pixel(&*bitmap, 20, 50), [128, 0, 0, 255]);
        near(pixel(&*bitmap, 50, 50), [0, 0, 255, 255]);
    }
    scope.dispose();
}

#[test]
fn compositing_layer_invalid_backend_push_preserves_opacity_transform_and_clip() {
    for save_layers in [false, true] {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable().bind_to_self(Rc::new(SkiaOptions {
            use_opacity_save_layer: save_layers,
            ..SkiaOptions::default()
        }));
        let bitmap = PlatformRenderInterface::default()
            .create_render_target_bitmap(PixelSize::new(100, 100), Vector::new(96.0, 96.0));
        let mut implementation = bitmap.create_drawing_context();
        implementation.clear(Colors::TRANSPARENT);
        implementation.set_transform(Matrix::create_translation(20.0, 10.0));
        implementation.push_clip(Rect::new(0.0, 0.0, 20.0, 20.0));
        implementation.push_opacity(0.5, None);
        let transform = implementation.transform();
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        for (opacity, bounds) in [
            (f64::NAN, bounds),
            (f64::INFINITY, bounds),
            (-0.1, bounds),
            (1.1, bounds),
            (0.5, Rect::new(f64::NAN, 0.0, 10.0, 10.0)),
            (0.5, Rect::new(0.0, 0.0, -1.0, 10.0)),
            (0.5, Rect::new(0.0, 0.0, 10.0, f64::INFINITY)),
        ] {
            assert!(!implementation.try_push_compositing_layer(
                BitmapBlendingMode::Multiply,
                opacity,
                bounds
            ));
            assert_eq!(implementation.transform(), transform);
        }
        {
            let mut core = PlatformDrawingContext::borrowed(&mut *implementation);
            let mut context = DrawingContext::new(&mut core);
            context.fill_rectangle(&brush(Colors::RED), Rect::new(0.0, 0.0, 40.0, 40.0), 0.0);
        }
        implementation.pop_opacity();
        implementation.pop_clip();
        implementation.set_transform(Matrix::IDENTITY);
        {
            let mut core = PlatformDrawingContext::borrowed(&mut *implementation);
            let mut context = DrawingContext::new(&mut core);
            context.fill_rectangle(&brush(Colors::BLUE), Rect::new(0.0, 0.0, 10.0, 10.0), 0.0);
        }
        implementation.dispose();
        near(pixel(&*bitmap, 25, 15), [128, 0, 0, 128]);
        assert_eq!(pixel(&*bitmap, 45, 15), [0, 0, 0, 0]);
        assert_eq!(pixel(&*bitmap, 5, 5), [0, 0, 255, 255]);
        scope.dispose();
    }
}
