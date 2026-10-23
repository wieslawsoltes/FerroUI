//! Tests of the lighting and the effect graph of the WinUI composition port
//! (winui-materials.md, COMPOSITION-01 and COMPOSITION-02): content lit by a
//! multiply layer of the light map (exact multiplication, brightening, the
//! resolution of the map), specular highlights of the scene lighting
//! effect, lights whose coordinate space moves, the exact cross-fade, the
//! shadow effect and the effects drawn from the pixels of their sources. A
//! child module of the compositor tests, whose fixture renders the
//! composition visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::media::Colors;
use crate::platform::LtrbRect;
use crate::rendering::composition::effects::{
    ArithmeticCompositeEffect, BlendEffect, BlendEffectMode, CanvasComposite, ColorSourceEffect, CompositeEffect,
    CrossFadeEffect, GraphicsEffectSource, ShadowEffect,
};
use crate::rendering::composition::server::{
    LightAttenuation, LightColor, LightMap, LightSource, LightingMaterial,
};
use crate::rendering::composition::{
    CompositionBrush, CompositionEffectFactoryLoadStatus, CompositionEffectSourceParameter, CompositionVisual,
    ElementComposition, SpriteVisual,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{Matrix, Point, Rect, Vector, Vector3D};
use std::rc::Rc;

fn show(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) {
    ElementComposition::set_element_child_visual(&s.canvas, Some(visual.clone()));
    s.run_jobs();
}

/// Renders the server side of `visual` on a recording drawing context with
/// effects and layer effects.
fn render_with_layers(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    context.supports_layer_effects = true;
    s.server_object(visual.server()).render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

fn position_starting(entries: &[String], prefix: &str) -> usize {
    entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix} is drawn: {entries:?}"))
}

fn parameter(name: &str) -> Option<GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(name).into())
}

fn color_brush(s: &CompositorCanvas, color: crate::media::Color) -> Option<Rc<CompositionBrush>> {
    Some((*s.compositor.create_color_brush_with_color(color)).clone())
}

fn sprite(s: &CompositorCanvas, width: f64, height: f64, brush: Option<Rc<CompositionBrush>>) -> SpriteVisual {
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(width, height));
    sprite.set_brush(brush);
    sprite
}

// --- COMPOSITION-01: lighting ----------------------------------------------------------

#[test]
fn lit_content_is_drawn_in_a_multiply_layer_of_the_light_map() {
    let s = CompositorCanvas::new();
    let light = s.compositor.create_ambient_light();
    light.set_intensity(0.5);
    let lit = sprite(&s, 30.0, 20.0, color_brush(&s, Colors::BLUE));
    light.targets().add((*lit).clone());
    show(&s, &lit);

    // A sample every second device pixel: 15 x 10 for 30 x 20 at a scale of
    // 1, the light at half (a scale of 1).
    let entries = render_with_layers(&s, &lit);
    let push = position_starting(&entries, "PushMultiplyLayer 0, 0, 30, 20 15x10 1");
    let content = position_starting(&entries, "DrawRectangle Blue none 0, 0, 30, 20");
    let pop = position_starting(&entries, "PopMultiplyLayer");
    assert!(push < content && content < pop, "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushRenderOptions")), "no source-atop map: {entries:?}");

    // A light brighter than full brightens: the factors carry a scale.
    light.set_intensity(3.0);
    s.run_jobs();
    let entries = render_with_layers(&s, &lit);
    position_starting(&entries, "PushMultiplyLayer 0, 0, 30, 20 15x10 3");
}

#[test]
fn the_factors_of_a_light_map_multiply_each_channel() {
    let mut map = LightMap::new();
    map.add(LightSource::Ambient { color: LightColor::new(0.5, 1.0, 2.0) }, Matrix::IDENTITY);
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let factors = map.factors(rect, LightMap::size_for(rect, 1.0), &LightingMaterial::VISUAL).expect("not full");
    assert_eq!(2.0, factors.scale);
    let [r, g, b] = factors.factor_at(0.5, 0.5);
    assert!((r - 0.5).abs() < 0.01 && (g - 1.0).abs() < 0.01 && (b - 2.0).abs() < 0.01, "{r} {g} {b}");

    // A full white light changes nothing.
    let mut full = LightMap::new();
    full.add(LightSource::Ambient { color: LightColor::WHITE }, Matrix::IDENTITY);
    assert!(full.factors(rect, LightMap::size_for(rect, 1.0), &LightingMaterial::VISUAL).is_none());

    // The map follows the device scale, up to its largest size.
    assert_eq!(crate::PixelSize::new(1024, 2), LightMap::size_for(Rect::new(0.0, 0.0, 100_000.0, 1.0), 1.0));
}

#[test]
fn specular_lighting_adds_a_highlight_under_the_light() {
    let light = LightSource::Point {
        color: LightColor::WHITE,
        position: Vector3D::new(50.0, 50.0, 100.0),
        attenuation: LightAttenuation::NONE,
    };
    let diffuse_only = LightingMaterial { ambient: 0.0, diffuse: 0.5, specular: 0.0, shine: 16.0 };
    let with_specular = LightingMaterial { specular: 1.0, ..diffuse_only };
    let under = Point::new(50.0, 50.0);
    let diffuse = light.lit_at(under, &diffuse_only);
    let shiny = light.lit_at(under, &with_specular);
    assert!((diffuse.r - 0.5).abs() < 1e-9, "{diffuse:?}");
    assert!((shiny.r - 1.5).abs() < 1e-9, "the highlight is full under the light: {shiny:?}");
    // Away from the light the highlight falls off faster than the diffuse light.
    let aside = Point::new(150.0, 50.0);
    let diffuse_aside = light.lit_at(aside, &diffuse_only).r;
    let specular_aside = light.lit_at(aside, &with_specular).r - diffuse_aside;
    assert!(specular_aside > 0.0 && specular_aside < diffuse_aside, "{specular_aside} {diffuse_aside}");
    // The lighting of a visual has no highlight.
    assert_eq!(0.0, LightingMaterial::VISUAL.specular);
}

#[test]
fn a_light_whose_coordinate_space_moves_draws_its_targets_again() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let root = s.compositor.create_container_visual();
    root.set_size(Vector::new(200.0, 100.0));
    let space = s.compositor.create_container_visual();
    space.set_size(Vector::new(10.0, 10.0));
    root.children().insert_at_top((*space).clone());
    let target = sprite(&s, 100.0, 100.0, color_brush(&s, Colors::BLUE));
    root.children().insert_at_top((*target).clone());

    let light = s.compositor.create_point_light();
    light.set_offset(Vector3D::new(0.0, 0.0, 30.0));
    light.set_coordinate_space(Some(space.clone()));
    light.targets().add((*target).clone());
    show(&s, &root);
    assert!(log.count("DrawRectangle Blue none 0, 0, 100, 100") > 0, "{:?}", log.entries());

    // Only the (empty) coordinate space moves: the light moves with it, and
    // its target is drawn again.
    log.clear();
    space.set_offset(Vector3D::new(60.0, 40.0, 0.0));
    s.run_jobs();
    assert!(log.count("DrawRectangle Blue none 0, 0, 100, 100") > 0, "{:?}", log.entries());
}

// --- COMPOSITION-02: the effect graph --------------------------------------------------

#[test]
fn a_cross_fade_adds_its_weighted_sources_in_a_layer() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory(CrossFadeEffect {
        name: "Fade".into(),
        weight: 0.25,
        source1: parameter("a"),
        source2: parameter("b"),
    });
    let brush = factory.create_brush();
    brush.set_source_parameter("a", color_brush(&s, Colors::RED));
    brush.set_source_parameter("b", color_brush(&s, Colors::BLUE));
    let visual = sprite(&s, 100.0, 50.0, Some(brush.brush().clone()));
    show(&s, &visual);
    let entries = render_with_layers(&s, &visual);
    let layer = position_starting(&entries, "PushLayer 0, 0, 100, 50");
    let red = position_starting(&entries, "DrawRectangle Red");
    let plus = position_starting(&entries, "PushBlendLayer 0, 0, 100, 50 Plus");
    let blue = position_starting(&entries, "DrawRectangle Blue");
    assert!(layer < red && red < plus && plus < blue, "{entries:?}");
}

#[test]
fn a_shadow_effect_colours_the_alpha_of_its_source_and_blurs_it() {
    let s = CompositorCanvas::new();
    let factory = s.compositor.create_effect_factory_with_properties(
        ShadowEffect {
            name: "Shadow".into(),
            blur_amount: 4.0,
            shadow_color: Colors::GREEN,
            source: parameter("source"),
            ..Default::default()
        },
        &["Shadow.BlurAmount", "Shadow.ShadowColor"],
    );
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
    let brush = factory.create_brush();
    brush.set_source_parameter("source", color_brush(&s, Colors::BLUE));
    let visual = sprite(&s, 100.0, 50.0, Some(brush.brush().clone()));
    show(&s, &visual);
    let entries = render_with_layers(&s, &visual);
    let blur = position_starting(&entries, "PushEffect 0, 0, 100, 50 blur(");
    let matrix = position_starting(&entries, "PushColorMatrixLayer 0, 0, 100, 50 Straight");
    let source = position_starting(&entries, "DrawRectangle Blue");
    assert!(blur < matrix && matrix < source, "{entries:?}");
}

#[test]
fn effects_without_a_blend_layer_are_drawn_from_the_pixels_of_their_sources() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();
    let effects = [
        crate::rendering::composition::effects::GraphicsEffect::from(BlendEffect {
            name: String::new(),
            mode: BlendEffectMode::Subtract,
            background: parameter("a"),
            foreground: parameter("b"),
        }),
        ArithmeticCompositeEffect {
            name: String::new(),
            source1: parameter("a"),
            source2: parameter("b"),
            ..Default::default()
        }
        .into(),
        CompositeEffect {
            name: String::new(),
            mode: CanvasComposite::MaskInvert,
            sources: vec![
                CompositionEffectSourceParameter::new("a").into(),
                ColorSourceEffect { name: String::new(), color: Colors::WHITE }.into(),
            ],
        }
        .into(),
    ];
    for effect in effects {
        let factory = s.compositor.create_effect_factory(effect);
        assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
        let brush = factory.create_brush();
        brush.set_source_parameter("a", color_brush(&s, Colors::RED));
        brush.set_source_parameter("b", color_brush(&s, Colors::BLUE));
        let visual = sprite(&s, 100.0, 50.0, Some(brush.brush().clone()));
        log.clear();
        show(&s, &visual);
        // The sources are drawn into bitmaps of their own (the log of the
        // render interface), and the combined pixels drawn over the brush.
        let entries = log.entries();
        assert!(entries.iter().any(|e| e.starts_with("DrawRectangle Red")), "{entries:?}");
        assert!(
            entries.iter().any(|e| e.starts_with("DrawBitmap 1 0, 0, 100, 50 0, 0, 100, 50")),
            "{entries:?}"
        );
    }
}
