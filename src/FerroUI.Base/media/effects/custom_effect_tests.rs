//! Tests of the custom effect model (#455, E1; not from upstream): the
//! snapshot's equality, padding and interpolation, parsing and its round
//! trips through the registry, the mutable effect classes, and the report of
//! a backend without a factory. The transport to the render thread is
//! tested with the compositor (`composition_custom_effect_tests.rs`), the
//! transitions and animations with the other effect animations
//! (`animation/tests/effect_tests.rs`).

use super::*;
use crate::animation::animators::ColorAnimator;
use crate::media::{Color, Colors};
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, Thickness};
use std::cell::Cell;
use std::rc::Rc;

/// A colour effect with a neutral value.
#[derive(Clone, Debug, PartialEq)]
struct Tint {
    color: Color,
    strength: f64,
}

impl Default for Tint {
    fn default() -> Self {
        Self { color: Colors::BLUE, strength: 0.55 }
    }
}

impl CustomEffect for Tint {
    const NAME: &'static str = "test-tint";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("color", self.color.into());
        f("strength", self.strength.into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "color" => value.as_color().map(|color| self.color = color).is_some(),
            "strength" => value.as_float().map(|strength| self.strength = strength).is_some(),
            _ => false,
        }
    }

    fn neutral() -> Option<Self> {
        Some(Self { color: Colors::WHITE, strength: 0.0 })
    }
}

/// An effect with a parameter of each other kind and a padding.
#[derive(Clone, Debug, Default, PartialEq)]
struct Spotlight {
    center: Point,
    radius: f64,
    enabled: bool,
    mode: i32,
    label: String,
}

impl CustomEffect for Spotlight {
    const NAME: &'static str = "test-spotlight";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        f("center", self.center.into());
        f("radius", self.radius.into());
        f("enabled", self.enabled.into());
        f("mode", self.mode.into());
        f("label", self.label.as_str().into());
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        match name {
            "center" => value.as_point().map(|center| self.center = center).is_some(),
            "radius" => value.as_float().map(|radius| self.radius = radius).is_some(),
            "enabled" => value.as_bool().map(|enabled| self.enabled = enabled).is_some(),
            "mode" => value.as_int().map(|mode| self.mode = mode).is_some(),
            "label" => value.as_text().map(|label| self.label = label.to_string()).is_some(),
            _ => false,
        }
    }

    fn padding(&self) -> Thickness {
        Thickness::uniform(self.radius)
    }
}

/// An effect whose padding is out of range.
#[derive(Clone, Debug, Default, PartialEq)]
struct BadPadding;

impl CustomEffect for BadPadding {
    const NAME: &'static str = "test-bad-padding";

    fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}

    fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
        false
    }

    fn padding(&self) -> Thickness {
        Thickness::new(-3.0, f64::NAN, f64::INFINITY, 2.0)
    }
}

fn tint(color: Color, strength: f64) -> ImmutableCustomEffect {
    ImmutableCustomEffect::new(Tint { color, strength })
}

// --- the snapshot ----------------------------------------------------------------

#[test]
fn snapshots_compare_by_value() {
    let a = tint(Colors::RED, 0.5);
    assert_eq!(a, tint(Colors::RED, 0.5));
    assert_eq!(a, a.clone());
    assert_ne!(a, tint(Colors::RED, 0.25));
    assert_ne!(a, ImmutableCustomEffect::new(Spotlight::default()));

    let same: Rc<dyn IEffect> = tint(Colors::RED, 0.5).into();
    assert!(a.equals(Some(&*same)));
    assert!(!a.equals(Some(&ImmutableBlurEffect::new(0.5))));
    assert!(!a.equals(None));
}

#[test]
fn a_snapshot_is_an_immutable_effect() {
    let effect: Rc<dyn IEffect> = tint(Colors::RED, 0.5).into();
    assert!(effect.as_custom_effect().is_some());
    assert!(effect.as_immutable_effect().is_some());
    assert!(effect.as_mutable_effect().is_none());
    assert!(effect.as_blur_effect().is_none());

    let immutable = EffectExtensions::to_immutable(&effect);
    let custom = immutable.as_custom_effect().expect("the immutable form is the snapshot");
    assert_eq!(&tint(Colors::RED, 0.5), custom);
    assert!(custom.is::<Tint>() && !custom.is::<Spotlight>());
    assert_eq!(std::any::TypeId::of::<Tint>(), custom.value_type_id());
    assert_eq!(Some(&Tint { color: Colors::RED, strength: 0.5 }), custom.downcast_ref::<Tint>());
    assert_eq!("IEffect(test-tint(color=Red, strength=0.5))", format!("{effect:?}"));
}

#[test]
fn a_snapshot_lists_its_parameters() {
    let effect = tint(Colors::RED, 0.5);
    assert_eq!(
        vec![("color", EffectParameterValue::Color(Colors::RED)), ("strength", EffectParameterValue::Float(0.5))],
        effect.parameters()
    );
    assert_eq!(Some(EffectParameterValue::Float(0.5)), effect.parameter("Strength"));
    assert_eq!(None, effect.parameter("radius"));
}

#[test]
fn the_padding_of_a_custom_effect_is_that_of_its_snapshot() {
    let spotlight = ImmutableCustomEffect::new(Spotlight { radius: 6.0, ..Spotlight::default() });
    assert_eq!(Thickness::uniform(6.0), EffectExtensions::get_effect_output_padding(Some(&spotlight)));
    assert_eq!(Thickness::default(), EffectExtensions::get_effect_output_padding(Some(&tint(Colors::RED, 1.0))));

    // Each side is at least zero, and a side that is not finite is zero.
    let bad = ImmutableCustomEffect::new(BadPadding);
    assert_eq!(Thickness::new(0.0, 0.0, 0.0, 2.0), EffectExtensions::get_effect_output_padding(Some(&bad)));
}

// --- interpolation ---------------------------------------------------------------

#[test]
fn the_default_interpolation_interpolates_each_parameter_by_its_kind() {
    let from = tint(Colors::BLACK, 0.0);
    let to = tint(Colors::WHITE, 1.0);
    let half = from.interpolate(&to, 0.5).expect("two tints interpolate");
    let half = half.downcast_ref::<Tint>().unwrap();
    assert_eq!(0.5, half.strength);
    assert_eq!(ColorAnimator::interpolate_core(0.5, &Colors::BLACK, &Colors::WHITE), half.color);

    let from = ImmutableCustomEffect::new(Spotlight {
        center: Point::new(0.0, 10.0),
        radius: 2.0,
        enabled: false,
        mode: 0,
        label: "a".into(),
    });
    let to = ImmutableCustomEffect::new(Spotlight {
        center: Point::new(10.0, 30.0),
        radius: 6.0,
        enabled: true,
        mode: 3,
        label: "b".into(),
    });
    let quarter = from.interpolate(&to, 0.25).unwrap();
    let quarter = quarter.downcast_ref::<Spotlight>().unwrap();
    assert_eq!(Point::new(2.5, 15.0), quarter.center);
    assert_eq!(3.0, quarter.radius);
    assert_eq!(1, quarter.mode);
    // Booleans and text switch at half.
    assert!(!quarter.enabled);
    assert_eq!("a", quarter.label);

    let three_quarters = from.interpolate(&to, 0.75).unwrap();
    let three_quarters = three_quarters.downcast_ref::<Spotlight>().unwrap();
    assert!(three_quarters.enabled);
    assert_eq!("b", three_quarters.label);
    assert_eq!(2, three_quarters.mode);
}

#[test]
fn effects_of_different_types_do_not_interpolate() {
    assert!(tint(Colors::RED, 1.0).interpolate(&ImmutableCustomEffect::new(Spotlight::default()), 0.5).is_none());
}

#[test]
fn a_type_may_have_a_neutral_value() {
    assert_eq!(Some(tint(Colors::WHITE, 0.0)), tint(Colors::RED, 1.0).neutral());
    assert_eq!(None, ImmutableCustomEffect::new(Spotlight::default()).neutral());
}

#[test]
fn parameter_values_interpolate_within_their_kind() {
    use EffectParameterValue as V;
    assert_eq!(Some(V::Float(5.0)), V::Float(0.0).interpolate(&V::Float(10.0), 0.5));
    assert_eq!(Some(V::Int(-2)), V::Int(0).interpolate(&V::Int(-4), 0.5));
    assert_eq!(
        Some(V::Rect(crate::Rect::new(1.0, 2.0, 3.0, 4.0))),
        V::Rect(crate::Rect::new(0.0, 0.0, 0.0, 0.0)).interpolate(&V::Rect(crate::Rect::new(2.0, 4.0, 6.0, 8.0)), 0.5)
    );
    assert_eq!(None, V::Float(0.0).interpolate(&V::Int(1), 0.5));
    assert_eq!(None, V::Bool(false).interpolate(&V::Bool(true), 0.5));
}

// --- parsing ---------------------------------------------------------------------

fn parse_custom(s: &str) -> ImmutableCustomEffect {
    let effect = Effect::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"));
    effect.as_custom_effect().unwrap_or_else(|| panic!("{s} is a custom effect")).clone()
}

#[test]
fn a_registered_effect_parses() {
    CustomEffectRegistry::register::<Tint>().unwrap();
    // Registering again does nothing.
    CustomEffectRegistry::register::<Tint>().unwrap();
    assert!(CustomEffectRegistry::is_registered("test-tint"));
    assert!(CustomEffectRegistry::is_registered("TestTint"));
    assert!(CustomEffectRegistry::names().contains(&"test-tint"));

    let effect = parse_custom("test-tint(color=#0F9D8E, strength=0.55)");
    assert_eq!(tint(Color::from_rgb(0x0F, 0x9D, 0x8E), 0.55), effect);

    // Names ignore case, `-` and `_`; a missing parameter keeps its default.
    let effect = parse_custom(" Test_Tint ( STRENGTH = 0.25 ) ");
    assert_eq!(tint(Colors::BLUE, 0.25), effect);
    assert_eq!(tint(Colors::BLUE, 0.55), parse_custom("test-tint()"));
}

#[test]
fn parsing_round_trips() {
    CustomEffectRegistry::register::<Tint>().unwrap();
    CustomEffectRegistry::register::<Spotlight>().unwrap();
    for effect in [
        tint(Color::from_rgb(0x0F, 0x9D, 0x8E), 0.55),
        tint(Colors::RED, 1.0 / 3.0),
        ImmutableCustomEffect::new(Spotlight {
            center: Point::new(10.5, -20.0),
            radius: 5.0,
            enabled: true,
            mode: 2,
            label: "two words, a comma".into(),
        }),
        ImmutableCustomEffect::new(Spotlight { label: "plain".into(), ..Spotlight::default() }),
    ] {
        let text = effect.to_string();
        assert_eq!(effect, parse_custom(&text), "{text}");
    }
    assert_eq!(
        "test-spotlight(center=10,20, radius=5, enabled=true, mode=2, label=plain)",
        ImmutableCustomEffect::new(Spotlight {
            center: Point::new(10.0, 20.0),
            radius: 5.0,
            enabled: true,
            mode: 2,
            label: "plain".into(),
        })
        .to_string()
    );
}

#[test]
fn composite_values_parse_with_commas_spaces_or_parentheses() {
    CustomEffectRegistry::register::<Spotlight>().unwrap();
    for s in ["test-spotlight(center=10,20)", "test-spotlight(center=10 20)", "test-spotlight(center=(10, 20))"] {
        assert_eq!(Point::new(10.0, 20.0), parse_custom(s).downcast_ref::<Spotlight>().unwrap().center, "{s}");
    }
}

#[test]
fn invalid_custom_effect_strings_fail() {
    CustomEffectRegistry::register::<Tint>().unwrap();
    for s in [
        "test-tint(radius=1)",
        "test-tint(strength=strong)",
        "test-tint(color=nocolor)",
        "test-tint(strength=1",
        "test-tint(strength=1) x",
        "not-registered(strength=1)",
    ] {
        assert!(Effect::parse(s).is_err(), "{s}");
    }
    // The built-in effects still parse.
    assert!(Effect::parse("blur(3)").unwrap().as_blur_effect().is_some());
    assert!(Effect::parse("drop-shadow(1 2 3 red)").unwrap().as_drop_shadow_effect().is_some());
}

#[test]
fn registration_rejects_reserved_invalid_and_taken_names() {
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Named<const N: usize>;

    macro_rules! named {
        ($n:literal, $name:literal) => {
            impl CustomEffect for Named<$n> {
                const NAME: &'static str = $name;
                fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}
                fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
                    false
                }
            }
        };
    }
    named!(0, "blur");
    named!(1, "Drop_Shadow");
    named!(2, "bad name");
    named!(3, "");
    named!(4, "test-taken");
    named!(5, "TEST_TAKEN");

    assert_eq!(Err(CustomEffectRegistrationError::ReservedName("blur")), CustomEffectRegistry::register::<Named<0>>());
    assert_eq!(
        Err(CustomEffectRegistrationError::ReservedName("Drop_Shadow")),
        CustomEffectRegistry::register::<Named<1>>()
    );
    assert_eq!(
        Err(CustomEffectRegistrationError::InvalidName("bad name")),
        CustomEffectRegistry::register::<Named<2>>()
    );
    assert_eq!(Err(CustomEffectRegistrationError::InvalidName("")), CustomEffectRegistry::register::<Named<3>>());
    assert_eq!(Ok(()), CustomEffectRegistry::register::<Named<4>>());
    assert_eq!(
        Err(CustomEffectRegistrationError::NameTaken("TEST_TAKEN")),
        CustomEffectRegistry::register::<Named<5>>()
    );
    assert!(!CustomEffectRegistry::is_registered("blur"));
}

#[test]
fn parameter_values_parse_what_they_write() {
    use EffectParameterValue as V;
    for value in [
        V::Bool(true),
        V::Int(-7),
        V::Float(0.125),
        V::Color(Color::from_argb(0x80, 0x0F, 0x9D, 0x8E)),
        V::Color(Colors::TEAL),
        V::Point(Point::new(1.5, -2.0)),
        V::Vector(crate::Vector::new(3.0, 4.0)),
        V::Size(crate::Size::new(5.0, 6.0)),
        V::Rect(crate::Rect::new(1.0, 2.0, 3.0, 4.0)),
        V::Text("dissolve".into()),
        V::Text("with space".into()),
    ] {
        let text = value.to_string();
        assert_eq!(Some(value.clone()), value.parse_like(&text), "{text}");
    }
    assert_eq!(Some(V::Bool(false)), V::Bool(true).parse_like("False"));
    assert_eq!(None, V::Int(0).parse_like("1.5"));
    assert_eq!(None, V::Point(Point::default()).parse_like("1,2,3"));
    assert_eq!(None, V::Float(0.0).parse_like(""));
}

// --- mutable effect classes ------------------------------------------------------

/// A mutable custom effect class: its values freeze into a [`Tint`].
#[repr(C)]
pub(crate) struct CustomTintTestEffect {
    base: Effect,
    strength: Cell<f64>,
}

ferro_class!(CustomTintTestEffect: Effect);
ferro_impl_classes!(CustomTintTestEffect: FerroObjectImpl);

impl CustomTintTestEffect {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: Effect::construct_custom::<Self>(), strength: Cell::new(0.5) })
    }

    pub(crate) fn set_strength(&self, value: f64) {
        self.strength.set(value);
        self.raise_invalidated();
    }
}

impl CustomEffectClass for CustomTintTestEffect {
    fn to_custom_effect(&self) -> ImmutableCustomEffect {
        tint(Colors::RED, self.strength.get())
    }
}

#[test]
fn a_custom_effect_class_freezes_into_its_snapshot() {
    let class = CustomTintTestEffect::new();
    assert!(class.is_custom());
    assert!(!BlurEffect::new().is_custom());
    assert_eq!(Some(tint(Colors::RED, 0.5)), class.custom_snapshot());

    let effect: Rc<dyn IEffect> = class.clone().into();
    assert!(effect.as_mutable_effect().is_some());
    assert!(effect.as_custom_effect().is_none());
    assert_eq!(Some(tint(Colors::RED, 0.5)), EffectExtensions::to_custom_effect(&*effect));

    let immutable = EffectExtensions::to_immutable(&effect);
    assert_eq!(Some(&tint(Colors::RED, 0.5)), immutable.as_custom_effect());
    assert!(EffectExtensions::effect_equals(Some(&*immutable), Some(&*effect)));

    // A change of the class makes the snapshot it was frozen into stale.
    class.set_strength(0.75);
    assert!(!EffectExtensions::effect_equals(Some(&*immutable), Some(&*effect)));
    assert_eq!(Some(&tint(Colors::RED, 0.75)), EffectExtensions::to_immutable(&effect).as_custom_effect());
}

#[test]
fn a_custom_effect_class_raises_invalidated() {
    let class = CustomTintTestEffect::new();
    let count = Rc::new(Cell::new(0));
    let counter = count.clone();
    let _subscription = class.invalidated(move || counter.set(counter.get() + 1));
    class.set_strength(0.25);
    assert_eq!(1, count.get());
}

#[test]
fn the_padding_of_a_custom_effect_class_is_that_of_its_snapshot() {
    let effect: Rc<dyn IEffect> = CustomTintTestEffect::new().into();
    assert_eq!(Thickness::default(), EffectExtensions::get_effect_output_padding(Some(&*effect)));
}

// --- backends without a factory --------------------------------------------------

#[test]
fn an_unhandled_effect_is_reported_once_per_backend() {
    #[derive(Clone, Debug, Default, PartialEq)]
    struct Reported;

    impl CustomEffect for Reported {
        const NAME: &'static str = "test-reported";
        fn for_each_parameter(&self, _: &mut dyn FnMut(&'static str, EffectParameterValue)) {}
        fn set_parameter(&mut self, _: &str, _: EffectParameterValue) -> bool {
            false
        }
    }

    let effect = ImmutableCustomEffect::new(Reported);
    assert!(effect.report_unhandled("TestBackendA"));
    assert!(!effect.report_unhandled("TestBackendA"));
    assert!(!ImmutableCustomEffect::new(Reported).report_unhandled("TestBackendA"));
    assert!(effect.report_unhandled("TestBackendB"));
}
