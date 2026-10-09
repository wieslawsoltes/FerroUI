//! The scenes of stage 6 of the Vello backend: box shadows, effects, scene
//! brushes, the interpolation of bitmaps and blending modes.
//!
//! As the scenes of `scenes.rs`: drawn through the contracts only, 200 by
//! 200 pixels, on a white target. A blur is a smooth ramp, and two
//! renderers that blur a little differently differ a little in every pixel
//! of the ramp without any pixel being far off: such a scene has, beside the
//! bound on the share of pixels beyond the tolerance, a bound on the mean
//! difference of all pixels, which a blur that is too wide, too narrow or
//! in the wrong place exceeds.

use crate::Backend;
use ferroui_base::media::effects::{ImmutableBlurEffect, ImmutableDropShadowEffect};
use ferroui_base::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use ferroui_base::media::immutable::{ImmutableImageBrush, ImmutablePen, ImmutableSolidColorBrush, ImmutableTransform};
use ferroui_base::media::{
    AcrylicBackgroundSource, AlignmentX, AlignmentY, BoxShadow, BoxShadows, Color, Colors, EdgeMode, IBrush,
    IExperimentalAcrylicMaterial, IImmutableBrush, ISceneBrush, ISceneBrushContent, ITileBrush, ITransform,
    ImmutableSceneBrush, RenderOptions, Stretch, TileMode,
};
use ferroui_base::platform::{AlphaFormat, IDrawingContextImpl, PixelFormat, SharedBitmapImpl};
use ferroui_base::{
    Matrix, PixelSize, Point, Rect, RelativePoint, RelativeRect, RelativeUnit, RoundedRect, Vector,
};
use std::any::Any;
use std::rc::Rc;
use std::sync::Arc;

/// The size of every scene in pixels.
pub const SCENE_SIZE: PixelSize = PixelSize::new(200, 200);

/// A scene with the two bounds of its difference from the Skia backend.
pub struct EffectScene {
    pub name: &'static str,
    pub draw: fn(&Backend, &mut dyn IDrawingContextImpl),
    /// The share of pixels (in percent) that may differ by more than the
    /// tolerance.
    pub share_bound: f64,
    /// The mean difference of the channels of all pixels (of 255) that the
    /// scene may have.
    pub mean_bound: f64,
}

const NAVY: Color = Color::from_argb(255, 20, 40, 120);
const ORANGE: Color = Color::from_argb(255, 240, 140, 20);
const TEAL: Color = Color::from_argb(255, 0, 150, 136);
const SHADE: Color = Color::from_argb(200, 0, 0, 0);

fn solid(color: Color) -> ImmutableSolidColorBrush {
    ImmutableSolidColorBrush::new(color)
}

fn no_shadows() -> BoxShadows {
    BoxShadows::default()
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::from_rect(Rect::new(x, y, width, height))
}

fn rounded(x: f64, y: f64, width: f64, height: f64, radius: f64) -> RoundedRect {
    RoundedRect::from_radius(Rect::new(x, y, width, height), radius)
}

fn background(context: &mut dyn IDrawingContextImpl) {
    context.clear(Colors::WHITE);
}

fn shadow(offset_x: f64, offset_y: f64, blur: f64, spread: f64, color: Color, is_inset: bool) -> BoxShadows {
    BoxShadows::new(BoxShadow { offset_x, offset_y, blur, spread, color, is_inset })
}

/// Four boxes with a shadow each: sharp and moved, blurred, blurred with a
/// spread, wide and moved with a negative spread.
fn four_shadows(context: &mut dyn IDrawingContextImpl, is_inset: bool, shape: fn(f64, f64, f64, f64) -> RoundedRect) {
    background(context);
    let fill = solid(Color::from_argb(255, 235, 235, 235));
    let shadows = [
        shadow(6.0, 4.0, 0.0, 0.0, NAVY, is_inset),
        shadow(0.0, 0.0, 8.0, 0.0, SHADE, is_inset),
        shadow(3.0, 5.0, 14.0, 6.0, SHADE, is_inset),
        shadow(-8.0, 6.0, 30.0, -4.0, TEAL, is_inset),
    ];
    for (index, shadows) in shadows.iter().enumerate() {
        let (x, y) = (25.0 + 100.0 * (index % 2) as f64, 25.0 + 100.0 * (index / 2) as f64);
        context.draw_rectangle(Some(&fill), None, shape(x, y, 50.0, 50.0), shadows);
    }
}

fn shadows_outset_rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, rect);
}

fn shadows_outset_rounded(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, |x, y, width, height| rounded(x, y, width, height, 10.0));
}

/// Corners that are elliptical and differ: the top left one square.
fn uneven(x: f64, y: f64, width: f64, height: f64) -> RoundedRect {
    RoundedRect::new(
        Rect::new(x, y, width, height),
        Vector::new(0.0, 0.0),
        Vector::new(22.0, 12.0),
        Vector::new(8.0, 8.0),
        Vector::new(14.0, 25.0),
    )
}

fn shadows_outset_elliptical(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, uneven);
}

fn shadows_outset_capsule(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, false, |x, y, width, height| rounded(x, y + 10.0, width, height - 20.0, 15.0));
}

fn shadows_inset_rectangle(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, true, rect);
}

fn shadows_inset_rounded(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, true, |x, y, width, height| rounded(x, y, width, height, 10.0));
}

fn shadows_inset_elliptical(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    four_shadows(context, true, uneven);
}

/// Several shadows of one box, outside and inside, under a transform, a
/// clip and an opacity.
fn shadows_combined(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let shadows = BoxShadows::with_rest(
        BoxShadow { offset_x: 4.0, offset_y: 6.0, blur: 12.0, spread: 2.0, color: SHADE, is_inset: false },
        &[
            BoxShadow { offset_x: -5.0, offset_y: -5.0, blur: 6.0, spread: 0.0, color: ORANGE, is_inset: false },
            BoxShadow { offset_x: 2.0, offset_y: 3.0, blur: 10.0, spread: 3.0, color: NAVY, is_inset: true },
        ],
    );

    context.push_clip_rounded(rounded(8.0, 8.0, 150.0, 184.0, 30.0));
    context.push_opacity(0.8, None);
    context.set_transform(Matrix::create_rotation(0.2) * Matrix::create_scale(1.4, 1.2) * Matrix::create_translation(50.0, 10.0));
    context.draw_rectangle(Some(&solid(Colors::WHITE)), None, rounded(10.0, 20.0, 70.0, 90.0, 12.0), &shadows);
    context.set_transform(Matrix::IDENTITY);
    context.pop_opacity();
    context.pop_clip();
}

fn with_effect(
    context: &mut dyn IDrawingContextImpl,
    clip_rect: Option<Rect>,
    effect: &dyn ferroui_base::media::effects::IEffect,
    draw: impl FnOnce(&mut dyn IDrawingContextImpl),
) {
    let effects = context.as_drawing_context_impl_with_effects().expect("a drawing context with effects");
    effects.push_effect(clip_rect, effect);
    draw(effects);
    effects.pop_effect();
}

/// Shapes blurred by a radius of 3, 10 and 24.
fn effect_blur(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_effect(context, None, &ImmutableBlurEffect::new(3.0), |context| {
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(15.0, 15.0, 70.0, 50.0), &no_shadows());
    });
    with_effect(context, None, &ImmutableBlurEffect::new(10.0), |context| {
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(110.0, 15.0, 70.0, 60.0));
    });
    with_effect(context, None, &ImmutableBlurEffect::new(24.0), |context| {
        context.draw_rectangle(Some(&solid(TEAL)), None, rounded(40.0, 105.0, 120.0, 60.0, 12.0), &no_shadows());
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(80.0, 120.0, 40.0, 30.0), &no_shadows());
    });
}

/// A blur under a transform and a rounded clip, with the clip rectangle
/// the compositor gives an effect: the bounds of the content and the
/// padding of the effect.
fn effect_blur_transformed(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_clip_rounded(rounded(20.0, 20.0, 160.0, 150.0, 40.0));
    context.set_transform(Matrix::create_rotation(0.35) * Matrix::create_scale(1.5, 1.5) * Matrix::create_translation(60.0, 0.0));
    with_effect(context, Some(Rect::new(9.0, 9.0, 82.0, 72.0)), &ImmutableBlurEffect::new(8.0), |context| {
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(20.0, 20.0, 60.0, 50.0), &no_shadows());
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(35.0, 30.0, 30.0, 30.0));
    });
    context.set_transform(Matrix::IDENTITY);
    context.pop_clip();
}

/// A blur whose clip rectangle is smaller than its content: the content is
/// cut before it is blurred.
fn effect_blur_bounded(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_effect(context, Some(Rect::new(40.0, 40.0, 120.0, 120.0)), &ImmutableBlurEffect::new(12.0), |context| {
        context.draw_rectangle(Some(&solid(NAVY)), None, rect(10.0, 70.0, 180.0, 60.0), &no_shadows());
    });
}

/// Drop shadows: sharp, blurred, translucent and of a translucent shape.
fn effect_drop_shadow(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    with_effect(context, None, &ImmutableDropShadowEffect::new(8.0, 6.0, 0.0, NAVY, 1.0), |context| {
        context.draw_rectangle(Some(&solid(ORANGE)), None, rect(15.0, 15.0, 60.0, 50.0), &no_shadows());
    });
    with_effect(context, None, &ImmutableDropShadowEffect::new(6.0, 8.0, 10.0, Colors::BLACK, 0.7), |context| {
        context.draw_ellipse(Some(&solid(TEAL)), None, Rect::new(110.0, 15.0, 70.0, 55.0));
    });
    with_effect(context, None, &ImmutableDropShadowEffect::new(-10.0, 12.0, 20.0, NAVY, 1.0), |context| {
        context.draw_rectangle(
            Some(&ImmutableSolidColorBrush::with_opacity(ORANGE, 0.6)),
            None,
            rounded(50.0, 105.0, 110.0, 55.0, 14.0),
            &no_shadows(),
        );
    });
}

/// A drop shadow under a transform, a clip and an opacity.
fn effect_drop_shadow_transformed(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_clip(Rect::new(10.0, 10.0, 165.0, 180.0));
    context.push_opacity(0.75, None);
    context.set_transform(Matrix::create_scale(1.6, 1.3) * Matrix::create_rotation(-0.25) * Matrix::create_translation(10.0, 70.0));
    with_effect(context, None, &ImmutableDropShadowEffect::new(6.0, 6.0, 8.0, Colors::BLACK, 0.8), |context| {
        context.draw_rectangle(Some(&solid(TEAL)), None, rounded(20.0, 10.0, 70.0, 50.0, 8.0), &no_shadows());
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(60.0, 40.0, 40.0, 40.0));
    });
    context.set_transform(Matrix::IDENTITY);
    context.pop_opacity();
    context.pop_clip();
}

/// A brush that paints a scene: an ellipse, a rounded rectangle and a line
/// in 60 by 40 units, as the content of a visual brush or a drawing brush.
struct SceneBrush {
    parameters: Rc<ImmutableSceneBrush>,
    scalable: bool,
}

/// The content of a [`SceneBrush`].
struct SceneBrushContent {
    parameters: Rc<ImmutableSceneBrush>,
    scalable: bool,
}

macro_rules! brush_members {
    () => {
        fn opacity(&self) -> f64 {
            self.parameters.opacity()
        }
        fn transform(&self) -> Option<Rc<dyn ITransform>> {
            self.parameters.transform()
        }
        fn transform_origin(&self) -> RelativePoint {
            self.parameters.transform_origin()
        }
        fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
            None
        }
        fn as_any(&self) -> &dyn Any {
            self
        }
    };
}

impl IBrush for SceneBrush {
    brush_members!();

    fn as_tile_brush(&self) -> Option<&dyn ITileBrush> {
        Some(self)
    }
    fn as_scene_brush(&self) -> Option<&dyn ISceneBrush> {
        Some(self)
    }
}

impl IImmutableBrush for SceneBrush {}

impl ITileBrush for SceneBrush {
    fn alignment_x(&self) -> AlignmentX {
        self.parameters.alignment_x()
    }
    fn alignment_y(&self) -> AlignmentY {
        self.parameters.alignment_y()
    }
    fn destination_rect(&self) -> RelativeRect {
        self.parameters.destination_rect()
    }
    fn source_rect(&self) -> RelativeRect {
        self.parameters.source_rect()
    }
    fn stretch(&self) -> Stretch {
        self.parameters.stretch()
    }
    fn tile_mode(&self) -> TileMode {
        self.parameters.tile_mode()
    }
}

impl ISceneBrush for SceneBrush {
    fn create_content(&self) -> Option<Rc<dyn ISceneBrushContent>> {
        Some(Rc::new(SceneBrushContent { parameters: self.parameters.clone(), scalable: self.scalable }))
    }
}

impl IBrush for SceneBrushContent {
    brush_members!();
}

impl IImmutableBrush for SceneBrushContent {}

impl ISceneBrushContent for SceneBrushContent {
    fn brush(&self) -> Rc<dyn ITileBrush> {
        self.parameters.clone()
    }
    fn rect(&self) -> Rect {
        Rect::new(0.0, 0.0, 60.0, 40.0)
    }
    fn render(&self, context: &mut dyn IDrawingContextImpl, transform: Option<Matrix>) {
        if let Some(transform) = transform {
            context.set_transform(transform);
        }
        context.draw_rectangle(Some(&solid(TEAL)), None, rounded(2.0, 2.0, 34.0, 36.0, 8.0), &no_shadows());
        context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(22.5, 6.5, 35.0, 27.0));
        context.draw_line(
            Some(&ImmutablePen::with_brush(Some(Rc::new(solid(NAVY))), 3.0)),
            Point::new(4.0, 36.0),
            Point::new(56.0, 4.0),
        );
    }
    fn use_scalable_rasterization(&self) -> bool {
        self.scalable
    }
    fn dispose(&self) {}
}

/// The parameters of a scene brush.
struct SceneBrushSpec {
    scalable: bool,
    tile_mode: TileMode,
    stretch: Stretch,
    destination_rect: Option<RelativeRect>,
    source_rect: Option<RelativeRect>,
    alignment: (AlignmentX, AlignmentY),
    opacity: f64,
    transform: Option<Matrix>,
}

impl SceneBrushSpec {
    /// A tile of 48 by 32 units that is repeated.
    fn tiled(scalable: bool, tile_mode: TileMode) -> Self {
        Self {
            scalable,
            tile_mode,
            stretch: Stretch::Fill,
            destination_rect: Some(RelativeRect::new(6.0, 4.0, 48.0, 32.0, RelativeUnit::Absolute)),
            source_rect: None,
            alignment: (AlignmentX::Center, AlignmentY::Center),
            opacity: 1.0,
            transform: None,
        }
    }

    fn brush(self) -> SceneBrush {
        let parameters = ImmutableImageBrush::new(
            None,
            self.alignment.0,
            self.alignment.1,
            self.destination_rect,
            self.opacity,
            self.transform.map(|transform| Rc::new(ImmutableTransform::new(transform))),
            RelativePoint::default(),
            self.source_rect,
            self.stretch,
            self.tile_mode,
            None,
        );

        SceneBrush { parameters: Rc::new(ImmutableSceneBrush::new(&parameters)), scalable: self.scalable }
    }
}

fn fill_with_scene_brush(context: &mut dyn IDrawingContextImpl, spec: SceneBrushSpec) {
    background(context);
    context.draw_rectangle(Some(&spec.brush()), None, rect(10.0, 10.0, 180.0, 180.0), &no_shadows());
}

fn scene_brush_single(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(true, TileMode::None));
}

fn scene_brush_tile(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(true, TileMode::Tile));
}

fn scene_brush_flip_x(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(true, TileMode::FlipX));
}

fn scene_brush_flip_y(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(true, TileMode::FlipY));
}

fn scene_brush_flip_xy(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(true, TileMode::FlipXY));
}

/// The picture of the content at its own size as the tile.
fn scene_brush_surface_single(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(false, TileMode::None));
}

fn scene_brush_surface_tile(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(false, TileMode::Tile));
}

fn scene_brush_surface_flip_xy(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    fill_with_scene_brush(context, SceneBrushSpec::tiled(false, TileMode::FlipXY));
}

/// The whole area as one tile, the content keeping its aspect ratio at the
/// bottom right; then a part of the content, stretched.
fn scene_brush_stretched(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let uniform = SceneBrushSpec {
        stretch: Stretch::Uniform,
        destination_rect: None,
        alignment: (AlignmentX::Right, AlignmentY::Bottom),
        ..SceneBrushSpec::tiled(true, TileMode::None)
    };
    context.draw_rectangle(Some(&uniform.brush()), None, rect(10.0, 10.0, 180.0, 90.0), &no_shadows());

    let part = SceneBrushSpec {
        destination_rect: None,
        source_rect: Some(RelativeRect::new(0.25, 0.1, 0.5, 0.8, RelativeUnit::Relative)),
        opacity: 0.7,
        ..SceneBrushSpec::tiled(true, TileMode::None)
    };
    context.draw_ellipse(Some(&part.brush()), None, Rect::new(30.0, 105.0, 140.0, 85.0));
}

/// A tiled scene brush with a transform of its own, under a transform of
/// the context, as the fill and the pen of a shape.
fn scene_brush_transformed(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    let fill = SceneBrushSpec {
        transform: Some(Matrix::create_rotation(0.3) * Matrix::create_scale(1.2, 0.9)),
        ..SceneBrushSpec::tiled(true, TileMode::Tile)
    };
    let stroke = SceneBrushSpec::tiled(true, TileMode::FlipXY);

    context.set_transform(Matrix::create_scale(1.5, 1.5) * Matrix::create_rotation(-0.15) * Matrix::create_translation(5.0, 35.0));
    context.draw_rectangle(
        Some(&fill.brush()),
        Some(&ImmutablePen::with_brush(Some(Rc::new(stroke.brush())), 12.0)),
        rounded(15.0, 10.0, 95.0, 70.0, 12.0),
        &no_shadows(),
    );
    context.set_transform(Matrix::IDENTITY);
}

/// A material color.
struct Material {
    background_source: AcrylicBackgroundSource,
    tint_color: Color,
    material_color: Color,
}

impl IExperimentalAcrylicMaterial for Material {
    fn background_source(&self) -> AcrylicBackgroundSource {
        self.background_source
    }
    fn tint_color(&self) -> Color {
        self.tint_color
    }
    fn tint_opacity(&self) -> f64 {
        1.0
    }
    fn material_color(&self) -> Color {
        self.material_color
    }
    fn fallback_color(&self) -> Color {
        Colors::GRAY
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Rectangles of acrylic materials: opaque, translucent over a shape, and
/// one that replaces what is under it.
fn acrylic(_: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(20.0, 20.0, 160.0, 160.0));

    let acrylic = context.as_drawing_context_with_acrylic_like_support().expect("a drawing context with acrylic");
    acrylic.draw_rectangle_with_material(
        &Material { background_source: AcrylicBackgroundSource::None, tint_color: NAVY, material_color: TEAL },
        rounded(10.0, 10.0, 80.0, 80.0, 14.0),
    );
    acrylic.draw_rectangle_with_material(
        &Material {
            background_source: AcrylicBackgroundSource::None,
            tint_color: Color::from_argb(120, 20, 40, 120),
            material_color: Color::from_argb(90, 255, 255, 255),
        },
        rect(100.5, 30.5, 85.0, 70.0),
    );
    acrylic.draw_rectangle_with_material(
        &Material {
            background_source: AcrylicBackgroundSource::Digger,
            tint_color: Color::from_argb(60, 0, 150, 136),
            material_color: Color::from_argb(40, 0, 0, 0),
        },
        rounded(40.0, 110.0, 120.0, 70.0, 20.0),
    );
}

/// A bitmap of `size` by `size` pixels with detail of every scale: blocks
/// of four colors, a ramp, and lines a pixel wide.
fn detail_bitmap(backend: &Backend, size: i32) -> Arc<SharedBitmapImpl> {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let block = ((x * 8 / size) + (y * 8 / size)) % 4;
            let (mut r, mut g, mut b) = match block {
                0 => (230, 60, 40),
                1 => (250, 220, 60),
                2 => (40, 90, 200),
                _ => (30, 160, 120),
            };
            // A ramp across the bitmap.
            g = (g + x * 60 / size).min(255);
            // Lines of a pixel, every fourth.
            if x % 4 == 0 || y % 4 == 1 {
                (r, g, b) = (r / 4, g / 4, b / 4);
            }
            // BGRA.
            data.extend_from_slice(&[b as u8, g as u8, r as u8, 255]);
        }
    }

    backend.interface.load_bitmap_from_pixels(
        PixelFormat::BGRA8888,
        AlphaFormat::Premul,
        &data,
        PixelSize::new(size, size),
        Vector::new(96.0, 96.0),
        size * 4,
    )
}

fn interpolation(backend: &Backend, context: &mut dyn IDrawingContextImpl, mode: BitmapInterpolationMode, upscaled: bool) {
    background(context);
    context.push_render_options(RenderOptions { bitmap_interpolation_mode: mode, ..RenderOptions::default() });

    if upscaled {
        // A part of a small bitmap, 8.4 times as large: no middle of a
        // pixel of the target lies on the edge between two pixels of the
        // bitmap, where the nearest pixel is either and the two backends
        // choose differently.
        let bitmap = detail_bitmap(backend, 24);
        context.draw_bitmap(&*bitmap, 1.0, Rect::new(2.0, 2.0, 20.0, 20.0), Rect::new(15.0, 15.0, 168.0, 168.0));
    } else {
        // A large bitmap at 0.29 of its size, at 0.7 by 0.21, and at 0.45
        // under a rotation; at positions that keep the middles of the
        // pixels of the target off the edges of the pixels of the bitmap.
        let bitmap = detail_bitmap(backend, 256);
        let whole = Rect::new(0.0, 0.0, 256.0, 256.0);
        context.draw_bitmap(&*bitmap, 1.0, whole, Rect::new(8.3, 8.3, 75.3, 75.3));
        context.draw_bitmap(&*bitmap, 1.0, whole, Rect::new(8.3, 140.3, 180.3, 54.3));
        context.set_transform(Matrix::create_rotation(0.3) * Matrix::create_translation(105.0, -5.0));
        context.draw_bitmap(&*bitmap, 1.0, whole, Rect::new(0.3, 0.3, 115.3, 115.3));
        context.set_transform(Matrix::IDENTITY);
    }

    context.pop_render_options();
}

macro_rules! interpolation_scenes {
    ($($name:ident => $mode:ident, $upscaled:expr;)*) => {
        $(
            fn $name(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
                interpolation(backend, context, BitmapInterpolationMode::$mode, $upscaled);
            }
        )*
    };
}

interpolation_scenes! {
    interpolation_none_upscaled => None, true;
    interpolation_low_upscaled => LowQuality, true;
    interpolation_medium_upscaled => MediumQuality, true;
    interpolation_high_upscaled => HighQuality, true;
    interpolation_none_downscaled => None, false;
    interpolation_low_downscaled => LowQuality, false;
    interpolation_medium_downscaled => MediumQuality, false;
    interpolation_high_downscaled => HighQuality, false;
}

/// A bitmap with opaque, translucent and transparent pixels drawn in a
/// blending mode over opaque, translucent and transparent pixels: the
/// target is not cleared.
fn blending(backend: &Backend, context: &mut dyn IDrawingContextImpl, mode: BitmapBlendingMode) {
    context.draw_rectangle(Some(&solid(TEAL)), None, rect(10.0, 10.0, 110.0, 110.0), &no_shadows());
    context.draw_ellipse(
        Some(&ImmutableSolidColorBrush::with_opacity(NAVY, 0.6)),
        None,
        Rect::new(60.0, 60.0, 130.0, 130.0),
    );
    context.draw_rectangle(Some(&solid(Color::from_argb(255, 250, 220, 60))), None, rect(20.0, 150.0, 60.0, 40.0), &no_shadows());

    let bitmap = backend.interface.create_render_target_bitmap(PixelSize::new(80, 80), Vector::new(96.0, 96.0));
    let mut bitmap_context = bitmap.create_drawing_context();
    bitmap_context.draw_ellipse(Some(&solid(ORANGE)), None, Rect::new(4.0, 4.0, 56.0, 56.0));
    bitmap_context.draw_rectangle(
        Some(&ImmutableSolidColorBrush::with_opacity(Color::from_argb(255, 200, 40, 160), 0.5)),
        None,
        rect(30.0, 30.0, 46.0, 46.0),
        &no_shadows(),
    );
    bitmap_context.dispose();

    context.push_render_options(RenderOptions { bitmap_blending_mode: mode, ..RenderOptions::default() });
    context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 80.0, 80.0), Rect::new(30.0, 30.0, 150.0, 150.0));
    // And with an opacity, smaller.
    context.draw_bitmap(&*bitmap, 0.6, Rect::new(0.0, 0.0, 80.0, 80.0), Rect::new(5.0, 120.0, 70.0, 70.0));
    context.pop_render_options();
    bitmap.dispose();
}

macro_rules! blending_scenes {
    ($($name:ident => $mode:ident;)*) => {
        $(
            fn $name(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
                blending(backend, context, BitmapBlendingMode::$mode);
            }
        )*
    };
}

blending_scenes! {
    blend_source_over => SourceOver;
    blend_source => Source;
    blend_destination => Destination;
    blend_destination_over => DestinationOver;
    blend_source_in => SourceIn;
    blend_destination_in => DestinationIn;
    blend_source_out => SourceOut;
    blend_destination_out => DestinationOut;
    blend_source_atop => SourceAtop;
    blend_destination_atop => DestinationAtop;
    blend_xor => Xor;
    blend_plus => Plus;
    blend_screen => Screen;
    blend_overlay => Overlay;
    blend_darken => Darken;
    blend_lighten => Lighten;
    blend_color_dodge => ColorDodge;
    blend_color_burn => ColorBurn;
    blend_hard_light => HardLight;
    blend_soft_light => SoftLight;
    blend_difference => Difference;
    blend_exclusion => Exclusion;
    blend_multiply => Multiply;
    blend_hue => Hue;
    blend_saturation => Saturation;
    blend_color => Color;
    blend_luminosity => Luminosity;
}

/// Aliased edges: shapes, a stroke, a clip of a geometry and a bitmap
/// without anti-aliasing.
fn edge_mode_aliased(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.push_render_options(RenderOptions { edge_mode: EdgeMode::Aliased, ..RenderOptions::default() });
    context.draw_rectangle(Some(&solid(TEAL)), None, rounded(10.3, 10.6, 80.0, 60.0, 14.0), &no_shadows());
    context.draw_ellipse(
        None,
        Some(&ImmutablePen::with_brush(Some(Rc::new(solid(NAVY))), 5.0)),
        Rect::new(105.5, 12.5, 80.0, 60.0),
    );
    context.draw_line(
        Some(&ImmutablePen::with_brush(Some(Rc::new(solid(ORANGE))), 3.0)),
        Point::new(12.0, 85.0),
        Point::new(188.0, 110.0),
    );

    let bitmap = detail_bitmap(backend, 24);
    context.set_transform(Matrix::create_rotation(0.2) * Matrix::create_translation(60.0, 105.0));
    context.draw_bitmap(&*bitmap, 1.0, Rect::new(0.0, 0.0, 24.0, 24.0), Rect::new(0.0, 0.0, 90.0, 70.0));
    context.set_transform(Matrix::IDENTITY);
    context.pop_render_options();
}

/// Bitmaps created from pixels in each format and alpha format the
/// backends read: sixteen bits a pixel, thirty-two without alpha, and the
/// two orders of four channels premultiplied, not premultiplied and opaque.
fn pixel_formats(backend: &Backend, context: &mut dyn IDrawingContextImpl) {
    background(context);
    context.draw_ellipse(Some(&solid(TEAL)), None, Rect::new(20.0, 20.0, 160.0, 160.0));

    // The color and the alpha of a pixel of a pattern of 12 by 12.
    let pattern = |x: u32, y: u32| ((x * 21) as u8, (y * 21) as u8, ((x + y) * 10) as u8, (40 + (x * y * 3) % 216) as u8);
    let bitmap = |format: PixelFormat, alpha_format: AlphaFormat| {
        let mut data = Vec::new();
        for y in 0..12 {
            for x in 0..12 {
                let (r, g, b, a) = pattern(x, y);
                // The fourth byte of an opaque pixel is 255, but for the
                // format that has no alpha, where it is not looked at. (Of
                // a pixel that is declared opaque and has another byte
                // there, the raster path of Skia copies the byte into the
                // target; the Vello backend reads the pixel as opaque.)
                let a = if alpha_format == AlphaFormat::Opaque && format != PixelFormat::RGB32 { 255 } else { a };
                if format == PixelFormat::RGB565 {
                    let value = ((r as u16 >> 3) << 11) | ((g as u16 >> 2) << 5) | (b as u16 >> 3);
                    data.extend_from_slice(&value.to_le_bytes());
                } else {
                    // Premultiplied pixels have colors no larger than their
                    // alpha.
                    let (r, g, b) = if alpha_format == AlphaFormat::Premul {
                        let premultiply = |channel: u8| ((channel as u32 * a as u32 + 127) / 255) as u8;
                        (premultiply(r), premultiply(g), premultiply(b))
                    } else {
                        (r, g, b)
                    };
                    data.extend_from_slice(&if format == PixelFormat::BGRA8888 { [b, g, r, a] } else { [r, g, b, a] });
                }
            }
        }
        let stride = if format == PixelFormat::RGB565 { 24 } else { 48 };
        backend.interface.load_bitmap_from_pixels(format, alpha_format, &data, PixelSize::new(12, 12), Vector::new(96.0, 96.0), stride)
    };

    let bitmaps = [
        bitmap(PixelFormat::RGB565, AlphaFormat::Opaque),
        bitmap(PixelFormat::RGB32, AlphaFormat::Opaque),
        bitmap(PixelFormat::RGBA8888, AlphaFormat::Premul),
        bitmap(PixelFormat::BGRA8888, AlphaFormat::Premul),
        bitmap(PixelFormat::RGBA8888, AlphaFormat::Unpremul),
        bitmap(PixelFormat::BGRA8888, AlphaFormat::Unpremul),
        bitmap(PixelFormat::RGBA8888, AlphaFormat::Opaque),
        bitmap(PixelFormat::BGRA8888, AlphaFormat::Opaque),
    ];

    context.push_render_options(RenderOptions {
        bitmap_interpolation_mode: BitmapInterpolationMode::None,
        ..RenderOptions::default()
    });
    for (index, bitmap) in bitmaps.iter().enumerate() {
        let (x, y) = (4.0 + 49.0 * (index % 4) as f64, 28.0 + 78.0 * (index / 4) as f64);
        context.draw_bitmap(&**bitmap, 1.0, Rect::new(0.0, 0.0, 12.0, 12.0), Rect::new(x, y, 48.0, 60.0));
    }
    context.pop_render_options();
}

/// The scenes with the bounds of each.
pub fn scenes() -> Vec<EffectScene> {
    macro_rules! scene {
        ($draw:ident, $share_bound:expr, $mean_bound:expr) => {
            EffectScene { name: stringify!($draw), draw: $draw, share_bound: $share_bound, mean_bound: $mean_bound }
        };
    }

    vec![
        scene!(shadows_outset_rectangle, 0.05, 0.54),
        scene!(shadows_outset_rounded, 0.19, 0.58),
        scene!(shadows_outset_elliptical, 0.06, 0.50),
        scene!(shadows_outset_capsule, 0.13, 0.48),
        scene!(shadows_inset_rectangle, 0.05, 0.41),
        scene!(shadows_inset_rounded, 0.22, 0.52),
        scene!(shadows_inset_elliptical, 0.07, 0.44),
        scene!(shadows_combined, 0.05, 0.53),
        scene!(effect_blur, 0.05, 0.55),
        scene!(effect_blur_transformed, 0.05, 0.32),
        scene!(effect_blur_bounded, 0.05, 0.69),
        scene!(effect_drop_shadow, 0.09, 0.39),
        scene!(effect_drop_shadow_transformed, 0.08, 0.34),
        scene!(scene_brush_single, 0.10, 0.09),
        scene!(scene_brush_tile, 1.04, 1.07),
        scene!(scene_brush_flip_x, 1.07, 1.08),
        scene!(scene_brush_flip_y, 1.11, 1.07),
        scene!(scene_brush_flip_xy, 1.13, 1.08),
        scene!(scene_brush_surface_single, 0.05, 0.07),
        scene!(scene_brush_surface_tile, 0.05, 0.46),
        scene!(scene_brush_surface_flip_xy, 0.05, 0.46),
        scene!(scene_brush_stretched, 0.32, 0.41),
        scene!(scene_brush_transformed, 0.53, 0.56),
        scene!(acrylic, 0.09, 0.38),
        scene!(interpolation_none_upscaled, 0.05, 0.05),
        scene!(interpolation_low_upscaled, 0.05, 0.34),
        scene!(interpolation_medium_upscaled, 0.05, 0.34),
        scene!(interpolation_high_upscaled, 0.05, 0.05),
        scene!(interpolation_none_downscaled, 0.05, 0.12),
        scene!(interpolation_low_downscaled, 0.05, 0.33),
        scene!(interpolation_medium_downscaled, 0.05, 0.82),
        scene!(interpolation_high_downscaled, 0.05, 0.82),
        scene!(blend_source_over, 0.20, 0.37),
        scene!(blend_source, 0.18, 0.43),
        scene!(blend_destination, 0.05, 0.12),
        scene!(blend_destination_over, 0.05, 0.27),
        scene!(blend_source_in, 0.14, 0.33),
        scene!(blend_destination_in, 0.14, 0.29),
        scene!(blend_source_out, 0.05, 0.20),
        scene!(blend_destination_out, 0.14, 0.29),
        scene!(blend_source_atop, 0.11, 0.25),
        scene!(blend_destination_atop, 0.18, 0.42),
        scene!(blend_xor, 0.12, 0.37),
        scene!(blend_plus, 0.20, 0.40),
        scene!(blend_screen, 0.20, 0.50),
        scene!(blend_overlay, 0.05, 0.37),
        scene!(blend_darken, 0.05, 0.29),
        scene!(blend_lighten, 0.20, 0.38),
        scene!(blend_color_dodge, 0.24, 0.37),
        scene!(blend_color_burn, 0.05, 0.31),
        scene!(blend_hard_light, 0.17, 0.52),
        scene!(blend_soft_light, 0.05, 0.30),
        scene!(blend_difference, 0.19, 0.42),
        scene!(blend_exclusion, 0.19, 0.63),
        scene!(blend_multiply, 0.05, 0.43),
        scene!(blend_hue, 0.05, 0.36),
        scene!(blend_saturation, 0.05, 0.30),
        scene!(blend_color, 0.05, 0.35),
        scene!(blend_luminosity, 0.10, 0.41),
        scene!(edge_mode_aliased, 0.16, 0.26),
        scene!(pixel_formats, 0.07, 0.15),
    ]
}
