//! Tests of custom effects on `Visual.Effect` (#455, E1; not from
//! upstream): the snapshot of a custom effect, or of a custom effect class,
//! crosses the batch stream to the server visual as a value, and the render
//! walk pushes it around the content, which a drawing context without a
//! factory for it draws unchanged. A child module of the compositor tests,
//! whose fixture renders the composition visuals.

use super::{CompositorCanvas, TestBorder};
use crate::media::effects::{
    CustomEffect, CustomEffectClass, Effect, EffectExtensions, EffectParameterValue, IEffect, ImmutableCustomEffect,
};
use crate::media::{Color, Colors};
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
