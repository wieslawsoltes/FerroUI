//! Tests of the WinUI effect brushes of phase 6 (not from upstream): effect
//! factories and their load status, effect brushes over colour brushes,
//! their animatable properties, layer visuals with an effect, and the
//! backdrop brush. A child module of the compositor tests, whose fixture
//! renders the composition visuals into a recording drawing context.

use super::CompositorCanvas;
use crate::animation::easings::LinearEasing;
use crate::media::{Colors, EffectExtensions, IEffect, ImmutableBlurEffect};
use crate::platform::LtrbRect;
use crate::rendering::composition::effects::{
    blur_amount_to_radius, CanvasComposite, ColorSourceEffect, CompositeEffect, CrossFadeEffect, GaussianBlurEffect,
    GraphicsEffectSource, OpacityEffect, SaturationEffect,
};
use crate::rendering::composition::server::{ServerCompositionEffectBrush, ServerLayerVisual};
use crate::rendering::composition::{
    CompositionBackdropBrush, CompositionEffectBrush, CompositionEffectFactoryError,
    CompositionEffectFactoryLoadStatus, CompositionEffectSourceParameter, CompositionGetValueStatus,
    CompositionVisual, ElementComposition, ICompositionObjectAnimations,
};
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::Vector;
use std::rc::Rc;
use std::time::Duration;

/// Shows `visual` as the child composition visual of the canvas and renders
/// a frame.
fn show(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) {
    ElementComposition::set_element_child_visual(&s.canvas, Some(visual.clone()));
    s.run_jobs();
}

/// Renders frames until `duration` of server time has passed, then commits
/// and renders once more.
fn run_for(s: &CompositorCanvas, duration: Duration) {
    let started = s.compositor.server().server_now();
    while s.compositor.server().server_now() < started + duration {
        s.render_loop.tick();
    }
    s.run_jobs();
}

/// Renders the server side of `visual` on a recording drawing context that
/// has effects and records them.
fn render_with_effects(s: &CompositorCanvas, visual: &Rc<CompositionVisual>) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    s.server_object(visual.server()).render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

fn position(entries: &[String], entry: &str) -> usize {
    entries.iter().position(|e| e == entry).unwrap_or_else(|| panic!("{entry} is drawn: {entries:?}"))
}

fn position_starting(entries: &[String], prefix: &str) -> usize {
    entries.iter().position(|e| e.starts_with(prefix)).unwrap_or_else(|| panic!("{prefix} is drawn: {entries:?}"))
}

fn parameter(name: &str) -> Option<GraphicsEffectSource> {
    Some(CompositionEffectSourceParameter::new(name).into())
}

/// The radius of the blur effect of the drawing contexts for a blur amount.
fn blur_text(blur_amount: f64) -> String {
    format!("blur({})", blur_amount_to_radius(blur_amount))
}

/// A sprite visual of 100 x 50 painted with `brush`.
fn sprite_with(s: &CompositorCanvas, brush: &CompositionEffectBrush) -> crate::rendering::composition::SpriteVisual {
    let sprite = s.compositor.create_sprite_visual();
    sprite.set_size(Vector::new(100.0, 50.0));
    sprite.set_brush(Some(brush.brush().clone()));
    sprite
}

// --- effect brushes ------------------------------------------------------------------

#[test]
fn a_blur_effect_brush_pushes_a_blur_around_its_source() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let factory = s.compositor.create_effect_factory(GaussianBlurEffect {
        name: "Blur".into(),
        blur_amount: 5.0,
        source: parameter("source"),
        ..Default::default()
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
    assert_eq!(None, factory.extended_error());
    let brush = factory.create_brush();
    assert_eq!(vec!["source".to_string()], brush.source_parameter_names());
    assert_eq!("CompositionEffectBrush", brush.type_name());
    assert!(CompositionEffectBrush::from_brush(&brush).is_some());

    let color = s.compositor.create_color_brush_with_color(Colors::BLUE);
    brush.set_source_parameter("source", Some((*color).clone()));
    let bound = brush.get_source_parameter("source").expect("the source parameter is bound");
    assert!(Rc::ptr_eq(&bound, &*color));
    let sprite = sprite_with(&s, &brush);
    show(&s, &sprite);

    // The server brush knows its property set and its source.
    let server = s.compositor.server().get::<ServerCompositionEffectBrush>(brush.server()).expect("the server brush exists");
    assert!(server.is_loaded() && server.properties().is_some() && server.source("source").is_some());

    // The canvas of the fixture has no effects: the source is drawn unblurred.
    assert_eq!(1, log.count("DrawRectangle Blue none 0, 0, 100, 50"), "{:?}", log.entries());

    // A drawing context with effects blurs the source, inside the rectangle
    // of the visual.
    let entries = render_with_effects(&s, &sprite);
    let push = position(&entries, &format!("PushEffect 0, 0, 100, 50 {}", blur_text(5.0)));
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    let pop = position(&entries, "PopEffect");
    assert!(push < draw && draw < pop, "{entries:?}");

    // An unbound source draws nothing.
    log.clear();
    brush.set_source_parameter("source", None);
    s.run_jobs();
    assert_eq!(0, log.count("DrawRectangle Blue"), "{:?}", log.entries());
}

#[test]
fn a_cross_fade_effect_brush_paints_its_sources_with_their_weights() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let factory = s.compositor.create_effect_factory_with_properties(
        CrossFadeEffect {
            name: "Crossfade".into(),
            weight: 0.25,
            source1: parameter("source1"),
            source2: parameter("source2"),
        },
        &["Crossfade.Weight"],
    );
    assert_eq!(CompositionEffectFactoryLoadStatus::Success, factory.load_status());
    let brush = factory.create_brush();
    brush.set_source_parameter("source1", Some((*s.compositor.create_color_brush_with_color(Colors::RED)).clone()));
    brush.set_source_parameter("source2", Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    // The animatable property is an entry of the properties of the brush,
    // with the value of the description.
    assert_eq!((CompositionGetValueStatus::Succeeded, 0.25), brush.properties().try_get_scalar("Crossfade.Weight"));
    let sprite = sprite_with(&s, &brush);
    show(&s, &sprite);

    // The first source with 1 - weight, then the second with the weight.
    let entries = log.entries();
    let first_opacity = position(&entries, "PushOpacity 0.75");
    let first = position(&entries, "DrawRectangle Red none 0, 0, 100, 50 shadows=0");
    let second_opacity = position(&entries, "PushOpacity 0.25");
    let second = position(&entries, "DrawRectangle Blue none 0, 0, 100, 50 shadows=0");
    assert!(first_opacity < first && first < second_opacity && second_opacity < second, "{entries:?}");

    // A new value of the entry redraws the brush: at weight 1 only the
    // second source shows.
    log.clear();
    brush.properties().insert_scalar("Crossfade.Weight", 1.0);
    s.run_jobs();
    let entries = log.entries();
    assert!(entries.iter().any(|e| e == "DrawRectangle Blue none 0, 0, 100, 50 shadows=0"), "{entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("DrawRectangle Red")), "{entries:?}");
}

#[test]
fn the_blur_amount_of_an_effect_brush_animates() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let factory = s.compositor.create_effect_factory_with_properties(
        GaussianBlurEffect { name: "Blur".into(), blur_amount: 2.0, source: parameter("source"), ..Default::default() },
        &["Blur.BlurAmount"],
    );
    let brush = factory.create_brush();
    brush.set_source_parameter("source", Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    assert_eq!((CompositionGetValueStatus::Succeeded, 2.0), brush.properties().try_get_scalar("Blur.BlurAmount"));
    let sprite = sprite_with(&s, &brush);
    show(&s, &sprite);
    let entries = render_with_effects(&s, &sprite);
    position(&entries, &format!("PushEffect 0, 0, 100, 50 {}", blur_text(2.0)));

    // `StartAnimation("Blur.BlurAmount")` on the brush animates the entry of
    // its properties; the brush is drawn again as the amount changes.
    let animation = s.compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, 10.0, Rc::new(LinearEasing::new()));
    animation.set_duration(Duration::from_millis(50));
    brush.start_animation("Blur.BlurAmount", &*animation);
    log.clear();
    // The animation is committed before its time is measured.
    s.run_jobs();
    run_for(&s, Duration::from_millis(150));
    assert!(log.count("DrawRectangle Blue none 0, 0, 100, 50") >= 2, "{:?}", log.entries());

    let entries = render_with_effects(&s, &sprite);
    position(&entries, &format!("PushEffect 0, 0, 100, 50 {}", blur_text(10.0)));

    // Stopping the animation stops it on the properties of the brush.
    brush.stop_animation("Blur.BlurAmount");
    s.run_jobs();
}

#[test]
fn an_unsupported_effect_reports_its_load_status_and_draws_nothing() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let factory = s.compositor.create_effect_factory(SaturationEffect {
        name: "Saturation".into(),
        saturation: 0.0,
        source: parameter("source"),
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Other, factory.load_status());
    assert_eq!(
        Some(&CompositionEffectFactoryError::EffectTypeNotSupported("SaturationEffect")),
        factory.extended_error()
    );
    let brush = factory.create_brush();
    brush.set_source_parameter("source", Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    let sprite = sprite_with(&s, &brush);
    show(&s, &sprite);
    assert_eq!(0, log.count("DrawRectangle Blue"), "{:?}", log.entries());
    let server = s.compositor.server().get::<ServerCompositionEffectBrush>(brush.server()).expect("the server brush exists");
    assert!(!server.is_loaded());

    // A composite mode other than source-over.
    let composite = s.compositor.create_effect_factory(CompositeEffect {
        mode: CanvasComposite::Xor,
        sources: vec![CompositionEffectSourceParameter::new("a").into()],
        ..Default::default()
    });
    assert_eq!(CompositionEffectFactoryLoadStatus::Other, composite.load_status());
    assert_eq!(
        Some(&CompositionEffectFactoryError::CompositeModeNotSupported(CanvasComposite::Xor)),
        composite.extended_error()
    );

    // An effect without its source.
    let missing = s.compositor.create_effect_factory(GaussianBlurEffect::default());
    assert_eq!(CompositionEffectFactoryLoadStatus::Other, missing.load_status());
    assert_eq!(
        Some(&CompositionEffectFactoryError::MissingSource { effect: "GaussianBlurEffect", name: String::new() }),
        missing.extended_error()
    );

    // An animatable property that is not one.
    let invalid = s.compositor.create_effect_factory_with_properties(
        GaussianBlurEffect { name: "Blur".into(), source: parameter("source"), ..Default::default() },
        &["Blur.Optimization"],
    );
    assert_eq!(CompositionEffectFactoryLoadStatus::Other, invalid.load_status());
    assert_eq!(
        Some(&CompositionEffectFactoryError::InvalidAnimatableProperty("Blur.Optimization".into())),
        invalid.extended_error()
    );

    // A graph nested too deeply.
    let mut effect: GraphicsEffectSource = CompositionEffectSourceParameter::new("source").into();
    for _ in 0..40 {
        effect = OpacityEffect { source: Some(effect), ..Default::default() }.into();
    }
    let GraphicsEffectSource::Effect(effect) = effect else { unreachable!("the root is an effect") };
    let complex = s.compositor.create_effect_factory(*effect);
    assert_eq!(CompositionEffectFactoryLoadStatus::EffectTooComplex, complex.load_status());
}

// --- layer visual effect -------------------------------------------------------------

#[test]
fn a_layer_visual_with_a_blur_effect_blurs_its_composed_children() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let factory = s.compositor.create_effect_factory(GaussianBlurEffect {
        name: "Blur".into(),
        blur_amount: 5.0,
        source: parameter("layer"),
        ..Default::default()
    });
    let effect = factory.create_brush();
    let layer = s.compositor.create_layer_visual();
    layer.set_size(Vector::new(100.0, 50.0));
    let child = s.compositor.create_sprite_visual();
    child.set_size(Vector::new(30.0, 20.0));
    child.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    layer.children().insert_at_top((*child).clone());
    layer.set_effect(Some(effect.clone()));
    assert!(Rc::ptr_eq(layer.effect().expect("the effect is set").brush(), effect.brush()));
    show(&s, &layer);

    let server = s.server_object(layer.server());
    assert!(server.content_as::<ServerLayerVisual>().and_then(|content| content.effect_brush()).is_some());

    // The canvas of the fixture has no effects: the children in the layer.
    let entries = log.entries();
    let push = position_starting(&entries, "PushLayer");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    assert!(push < draw, "{entries:?}");

    // The bounds of the layer grow by the padding of the blur.
    let blur = ImmutableBlurEffect::new(blur_amount_to_radius(5.0));
    let padding = EffectExtensions::get_effect_output_padding(Some(&blur as &dyn IEffect));
    let bounds = server.sub_tree_bounds().expect("the layer has bounds");
    assert!(
        bounds.left <= -padding.left + 1e-9 && bounds.right >= 30.0 + padding.right - 1e-9,
        "{bounds:?} {padding:?}"
    );

    // A drawing context with effects blurs the composed children, inside the
    // layer.
    let entries = render_with_effects(&s, &layer);
    let layer_push = position_starting(&entries, "PushLayer");
    let push = position_starting(&entries, "PushEffect");
    assert!(entries[push].ends_with(&blur_text(5.0)), "{entries:?}");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    let pop = position(&entries, "PopEffect");
    let layer_pop = position(&entries, "PopLayer");
    assert!(layer_push < push && push < draw && draw < pop && pop < layer_pop, "{entries:?}");
}

#[test]
fn a_layer_visual_effect_draws_its_other_sources_around_the_layer() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    // A red fill under the layer: the layer is the first source parameter.
    let factory = s.compositor.create_effect_factory(CompositeEffect {
        name: "Composite".into(),
        sources: vec![
            ColorSourceEffect { name: "Fill".into(), color: Colors::RED }.into(),
            CompositionEffectSourceParameter::new("layer").into(),
        ],
        ..Default::default()
    });
    let layer = s.compositor.create_layer_visual();
    layer.set_size(Vector::new(100.0, 50.0));
    let child = s.compositor.create_sprite_visual();
    child.set_size(Vector::new(30.0, 20.0));
    child.set_brush(Some((*s.compositor.create_color_brush_with_color(Colors::BLUE)).clone()));
    layer.children().insert_at_top((*child).clone());
    layer.set_effect(Some(factory.create_brush()));
    show(&s, &layer);

    let entries = log.entries();
    let fill = position(&entries, "DrawRectangle Red none 0, 0, 30, 20 shadows=0");
    let draw = position(&entries, "DrawRectangle Blue none 0, 0, 30, 20 shadows=0");
    assert!(fill < draw, "{entries:?}");

    // Without the effect the layer shows its children only.
    log.clear();
    layer.set_effect(None);
    s.run_jobs();
    let entries = log.entries();
    assert!(!entries.iter().any(|e| e.starts_with("DrawRectangle Red")), "{entries:?}");
    assert!(entries.iter().any(|e| e == "DrawRectangle Blue none 0, 0, 30, 20 shadows=0"), "{entries:?}");
}

// --- backdrop brush --------------------------------------------------------------------

#[test]
fn a_backdrop_brush_draws_nothing() {
    let s = CompositorCanvas::new();
    let log = s.render_interface.log().clone();

    let backdrop = s.compositor.create_backdrop_brush();
    assert_eq!("CompositionBackdropBrush", backdrop.type_name());
    assert!(CompositionBackdropBrush::from_brush(&backdrop).is_some());

    // As a sprite brush, and as the source of a blur.
    let factory = s.compositor.create_effect_factory(GaussianBlurEffect {
        name: "Blur".into(),
        source: parameter("backdrop"),
        ..Default::default()
    });
    let blur = factory.create_brush();
    blur.set_source_parameter("backdrop", Some((*backdrop).clone()));
    let root = s.compositor.create_container_visual();
    root.set_size(Vector::new(300.0, 100.0));
    let plain = s.compositor.create_sprite_visual();
    plain.set_size(Vector::new(100.0, 50.0));
    plain.set_brush(Some((*backdrop).clone()));
    let blurred = sprite_with(&s, &blur);
    root.children().insert_at_top((*plain).clone());
    root.children().insert_at_top((*blurred).clone());
    show(&s, &root);

    let entries = log.entries();
    assert!(!entries.iter().any(|e| e.starts_with("DrawRectangle") && e.contains("0, 0, 100, 50")), "{entries:?}");
}
