//! Tests of custom effects on `Visual.Effect` (#455, E1 and E2; not from
//! upstream): the snapshot of a custom effect, or of a custom effect class,
//! crosses the batch stream to the server visual as a value; the render walk
//! pushes an effect without a graph around the content, which a drawing
//! context without a factory for it draws unchanged, and draws an effect
//! with a graph as its graph around the content, the values of the snapshot
//! as its members. A child module of the compositor tests, whose fixture
//! renders the composition visuals.

use super::{CompositorCanvas, TestBorder};
use crate::media::effects::library::{
    BrightnessContrastEffect, GlowEffect, GrayscaleEffect, InvertEffect, SaturationEffect, SepiaEffect, TintEffect,
};
use crate::media::effects::{
    CustomEffect, CustomEffectClass, Effect, EffectExtensions, EffectParameterValue, IEffect, ImmutableCustomEffect,
    CUSTOM_EFFECT_CONTENT,
};
use crate::media::{Color, ColorMatrix, Colors};
use crate::rendering::composition::effects::{
    CompositionEffectFactoryError, CompositionEffectSourceParameter, GraphicsEffect, OpacityEffect,
};
use crate::platform::LtrbRect;
use crate::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Thickness, Visual};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, Default, PartialEq)]
struct Glow {
    radius: f64,
    color: Color,
}

impl CustomEffect for Glow {
    const NAME: &'static str = "composition-test-glow";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("radius", self.radius.into());
        f("color", self.color.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "radius" => value.as_float().map(|radius| self.radius = radius).is_some(),
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            _ => false,
        }
    }

    fn padding(&self) -> Thickness {
        Thickness::uniform(self.radius)
    }
}

fn glow(radius: f64, color: Color) -> ImmutableCustomEffect {
    ImmutableCustomEffect::new(Glow { radius, color })
}

/// A mutable custom effect class whose snapshot is a [`Glow`].
#[repr(C)]
pub(crate) struct CustomGlowTestEffect {
    base: Effect,
    radius: Cell<f64>,
}

ferro_class!(CustomGlowTestEffect: Effect);
ferro_impl_classes!(CustomGlowTestEffect: FerroObjectImpl);

impl CustomGlowTestEffect {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: Effect::construct_custom::<Self>(), radius: Cell::new(2.0) })
    }

    pub(crate) fn set_radius(&self, value: f64) {
        self.radius.set(value);
        self.raise_invalidated();
    }
}

impl CustomEffectClass for CustomGlowTestEffect {
    fn to_custom_effect(&self) -> ImmutableCustomEffect {
        glow(self.radius.get(), Colors::RED)
    }
}

/// The custom effect the server side of `visual` has.
fn server_effect(s: &CompositorCanvas, visual: &Visual) -> Option<ImmutableCustomEffect> {
    let effect = s.server_visual(visual).effect()?;
    Some(effect.as_custom_effect().expect("the effect of the server visual is a custom effect").clone())
}

/// Renders the server side of `visual` on a recording drawing context that
/// has effects and records them.
fn render(s: &CompositorCanvas, visual: &Visual) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    s.server_visual(visual).render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

#[test]
fn a_custom_effect_crosses_the_batch_stream_as_its_snapshot() {
    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    let effect = glow(4.0, Colors::RED);
    child.set_effect(Some(effect.clone().into()));
    s.run_jobs();

    let transported = server_effect(&s, &child).expect("the server visual has the effect");
    assert_eq!(effect, transported);
    assert_eq!(Some(&Glow { radius: 4.0, color: Colors::RED }), transported.downcast_ref::<Glow>());

    // A new value replaces the snapshot of the server.
    child.set_effect(Some(glow(8.0, Colors::BLUE).into()));
    s.run_jobs();
    assert_eq!(Some(glow(8.0, Colors::BLUE)), server_effect(&s, &child));

    child.set_effect(None);
    s.run_jobs();
    assert_eq!(None, server_effect(&s, &child));
}

#[test]
fn a_custom_effect_class_crosses_the_batch_stream_as_its_snapshot() {
    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    let class = CustomGlowTestEffect::new();
    let effect: Rc<dyn IEffect> = class.clone().into();
    child.set_effect(Some(effect));
    s.run_jobs();
    assert_eq!(Some(glow(2.0, Colors::RED)), server_effect(&s, &child));

    // A change of the class sends a new snapshot; the one sent before is a
    // value and does not change.
    let before = server_effect(&s, &child).unwrap();
    class.set_radius(6.0);
    s.run_jobs();
    assert_eq!(Some(glow(6.0, Colors::RED)), server_effect(&s, &child));
    assert_eq!(glow(2.0, Colors::RED), before);
}

#[test]
fn the_bounds_of_a_custom_effect_include_its_padding() {
    let effect: Rc<dyn IEffect> = glow(5.0, Colors::RED).into();
    assert_eq!(Thickness::uniform(5.0), EffectExtensions::get_effect_output_padding(Some(&*effect)));
    let class: Rc<dyn IEffect> = CustomGlowTestEffect::new().into();
    assert_eq!(Thickness::uniform(2.0), EffectExtensions::get_effect_output_padding(Some(&*class)));
}

#[test]
fn a_custom_effect_without_a_factory_leaves_the_content_unchanged() {
    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    s.run_jobs();
    let plain = render(&s, &child);
    assert!(plain.iter().any(|e| e.starts_with("DrawRectangle Red")), "{plain:?}");

    let effect = glow(4.0, Colors::RED);
    child.set_effect(Some(effect.clone().into()));
    s.run_jobs();
    let entries = render(&s, &child);

    // The walk pushes the effect around the content...
    let text = effect.to_string();
    let push = entries
        .iter()
        .position(|e| e.starts_with("PushEffect ") && e.ends_with(&text))
        .unwrap_or_else(|| panic!("the effect is pushed: {entries:?}"));
    let draw = entries.iter().position(|e| e.starts_with("DrawRectangle Red")).expect("the content is drawn");
    let pop = entries.iter().position(|e| e == "PopEffect").expect("the effect is popped");
    assert!(push < draw && draw < pop, "{entries:?}");

    // ...and the content is the same as without it.
    let without: Vec<String> =
        entries.iter().filter(|e| !e.starts_with("PushEffect") && e.as_str() != "PopEffect").cloned().collect();
    assert_eq!(plain, without);
}

/// Renders the server side of `visual` on a recording drawing context with
/// effects and layer effects, which records the effects and the matrices of
/// the colour matrix layers.
fn render_layers(s: &CompositorCanvas, visual: &Visual) -> Vec<String> {
    let log = DrawingLog::new();
    let mut context = MockDrawingContextImpl::new(log.clone());
    context.supports_effects = true;
    context.log_effects = true;
    context.supports_layer_effects = true;
    context.log_matrices = true;
    s.server_visual(visual).render(&mut context, LtrbRect::INFINITE, None, true, false, false);
    log.entries()
}

/// The matrix as the recording drawing context records it.
fn matrix_text(matrix: &ColorMatrix) -> String {
    let members: Vec<String> =
        matrix.to_row_major_4x5().iter().map(|value| format!("{}", (value * 1000.0).round() / 1000.0)).collect();
    format!("[{}]", members.join(" "))
}

/// The entries without those that start with one of `prefixes`.
fn without(entries: &[String], prefixes: &[&str]) -> Vec<String> {
    entries.iter().filter(|e| !prefixes.iter().any(|prefix| e.starts_with(prefix))).cloned().collect()
}

/// Sets `effect` on the child and renders it; asserts that a colour matrix
/// layer with `matrix` wraps the content and that nothing else changed.
fn assert_drawn_as_matrix(effect: ImmutableCustomEffect, matrix: &ColorMatrix) {
    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    s.run_jobs();
    let plain = render_layers(&s, &child);

    child.set_effect(Some(effect.clone().into()));
    s.run_jobs();
    let entries = render_layers(&s, &child);
    let expected = format!("Premultiplied {}", matrix_text(matrix));
    let push = entries
        .iter()
        .position(|e| e.starts_with("PushColorMatrixLayer ") && e.ends_with(&expected))
        .unwrap_or_else(|| panic!("{effect}: a colour matrix layer with {expected}: {entries:?}"));
    let draw = entries.iter().position(|e| e.starts_with("DrawRectangle Red")).expect("the content is drawn");
    let pop = entries.iter().position(|e| e == "PopColorMatrixLayer").expect("the layer is popped");
    assert!(push < draw && draw < pop, "{effect}: {entries:?}");
    assert!(!entries.iter().any(|e| e.starts_with("PushEffect")), "{effect}: not an effect of the context: {entries:?}");
    assert_eq!(plain, without(&entries, &["PushColorMatrixLayer", "PopColorMatrixLayer"]), "{effect}");
}

#[test]
fn the_colour_effects_of_the_library_are_drawn_as_their_graphs() {
    let tint = TintEffect { color: Colors::RED, strength: 0.5 };
    assert_drawn_as_matrix(ImmutableCustomEffect::new(tint.clone()), &tint.matrix());
    let grayscale = GrayscaleEffect { amount: 0.75 };
    assert_drawn_as_matrix(ImmutableCustomEffect::new(grayscale.clone()), &grayscale.matrix());
    let sepia = SepiaEffect { amount: 0.6 };
    assert_drawn_as_matrix(ImmutableCustomEffect::new(sepia.clone()), &sepia.matrix());
    let saturation = SaturationEffect { saturation: 1.8 };
    assert_drawn_as_matrix(ImmutableCustomEffect::new(saturation.clone()), &saturation.matrix());
    let contrast = BrightnessContrastEffect { brightness: 0.1, contrast: 1.4 };
    assert_drawn_as_matrix(ImmutableCustomEffect::new(contrast.clone()), &contrast.matrix());
    let invert = InvertEffect { amount: 0.8 };
    assert_drawn_as_matrix(ImmutableCustomEffect::new(invert.clone()), &invert.matrix());
}

#[test]
fn a_new_snapshot_changes_the_values_of_the_members_of_the_graph() {
    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    let first = TintEffect { color: Colors::RED, strength: 0.25 };
    child.set_effect(Some(ImmutableCustomEffect::new(first.clone()).into()));
    s.run_jobs();
    let entries = render_layers(&s, &child);
    assert!(entries.iter().any(|e| e.ends_with(&matrix_text(&first.matrix()))), "{entries:?}");

    let second = TintEffect { color: Colors::BLUE, strength: 1.0 };
    child.set_effect(Some(ImmutableCustomEffect::new(second.clone()).into()));
    s.run_jobs();
    let entries = render_layers(&s, &child);
    assert!(entries.iter().any(|e| e.ends_with(&matrix_text(&second.matrix()))), "{entries:?}");
    assert!(!entries.iter().any(|e| e.ends_with(&matrix_text(&first.matrix()))), "{entries:?}");
}

#[test]
fn a_neutral_colour_effect_draws_the_content_unchanged() {
    // The neutral values make identity matrices, which push no layer.
    for effect in [
        ImmutableCustomEffect::new(TintEffect::neutral().unwrap()),
        ImmutableCustomEffect::new(InvertEffect::neutral().unwrap()),
        ImmutableCustomEffect::new(BrightnessContrastEffect::neutral().unwrap()),
        ImmutableCustomEffect::new(SepiaEffect::neutral().unwrap()),
    ] {
        let s = CompositorCanvas::new();
        let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
        s.canvas.add(&child);
        s.run_jobs();
        let plain = render_layers(&s, &child);
        child.set_effect(Some(effect.clone().into()));
        s.run_jobs();
        assert_eq!(plain, render_layers(&s, &child), "{effect}");
    }
}

#[test]
fn a_glow_is_drawn_as_a_drop_shadow_of_the_content() {
    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    let glow = GlowEffect { color: Colors::BLUE, blur_radius: 6.0, intensity: 0.5 };
    child.set_effect(Some(ImmutableCustomEffect::new(glow.clone()).into()));
    s.run_jobs();
    let entries = render_layers(&s, &child);

    let push = entries
        .iter()
        .position(|e| e.starts_with("PushEffect ") && e.contains("drop-shadow(0 0 "))
        .unwrap_or_else(|| panic!("a drop shadow is pushed: {entries:?}"));
    let radius: f64 = entries[push]
        .rsplit("drop-shadow(0 0 ")
        .next()
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(|radius| radius.parse().ok())
        .expect("the radius of the shadow");
    assert!((radius - 6.0).abs() < 1e-9, "{radius}");
    let draw = entries.iter().position(|e| e.starts_with("DrawRectangle Red")).expect("the content is drawn");
    let pop = entries.iter().position(|e| e == "PopEffect").expect("the shadow is popped");
    assert!(push < draw && draw < pop, "{entries:?}");
    // The content is drawn once, by the drop shadow.
    assert_eq!(1, entries.iter().filter(|e| e.starts_with("DrawRectangle Red")).count(), "{entries:?}");
    // The bounds of the visual include the reach of the shadow.
    assert_eq!(
        glow.padding(),
        EffectExtensions::get_effect_output_padding(Some(&glow.drop_shadow() as &dyn IEffect))
    );
}

/// An effect whose graph is bound to its parameter by the name of an
/// effect of the graph: the default binding.
#[derive(Clone, Debug, PartialEq)]
struct Fade {
    opacity: f64,
}

impl Default for Fade {
    fn default() -> Self {
        Self { opacity: 1.0 }
    }
}

impl CustomEffect for Fade {
    const NAME: &'static str = "composition-test-fade";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("opacity", self.opacity.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "opacity" => value.as_float().map(|opacity| self.opacity = opacity).is_some(),
            _ => false,
        }
    }

    fn graph() -> Option<GraphicsEffect> {
        let content = CompositionEffectSourceParameter::new(CUSTOM_EFFECT_CONTENT);
        Some(OpacityEffect { name: "opacity".into(), opacity: 1.0, source: Some(content.into()) }.into())
    }
}

#[test]
fn a_parameter_sets_the_member_of_the_effect_named_after_it() {
    let effect = ImmutableCustomEffect::new(Fade { opacity: 0.25 });
    assert!(effect.has_graph());
    assert_eq!(vec![("opacity", "opacity.Opacity".to_string())], effect.graph_members());

    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    child.set_effect(Some(effect.into()));
    s.run_jobs();
    let entries = render_layers(&s, &child);
    let push = entries.iter().position(|e| e == "PushOpacity 0.25").unwrap_or_else(|| panic!("{entries:?}"));
    let draw = entries.iter().position(|e| e.starts_with("DrawRectangle Red")).expect("the content is drawn");
    let pop = entries.iter().rposition(|e| e == "PopOpacity").expect("the opacity is popped");
    assert!(push < draw && draw < pop, "{entries:?}");
}

/// An effect whose graph has a source other than the content.
#[derive(Clone, Debug, Default, PartialEq)]
struct TwoSources;

impl CustomEffect for TwoSources {
    const NAME: &'static str = "composition-test-two-sources";

    fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }

    fn graph() -> Option<GraphicsEffect> {
        let other = CompositionEffectSourceParameter::new("other");
        Some(OpacityEffect { name: "Opacity".into(), opacity: 0.5, source: Some(other.into()) }.into())
    }
}

#[test]
fn a_graph_that_does_not_load_is_drawn_as_an_effect_without_a_graph() {
    let effect = ImmutableCustomEffect::new(TwoSources);
    assert!(!effect.has_graph());
    assert_eq!(
        Some(CompositionEffectFactoryError::MissingSource {
            effect: "CompositionEffectSourceParameter",
            name: "other".to_string()
        }),
        effect.graph_error()
    );

    let s = CompositorCanvas::new();
    let child = TestBorder::filled(10.0, 10.0, 40.0, 30.0);
    s.canvas.add(&child);
    child.set_effect(Some(effect.clone().into()));
    s.run_jobs();
    let entries = render_layers(&s, &child);
    let text = effect.to_string();
    assert!(entries.iter().any(|e| e.starts_with("PushEffect ") && e.ends_with(&text)), "{entries:?}");
    assert!(entries.iter().any(|e| e == "PopEffect"), "{entries:?}");
}
