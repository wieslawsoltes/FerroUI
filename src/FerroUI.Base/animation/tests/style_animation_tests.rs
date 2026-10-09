use super::*;
use crate::animation::{Animation, FillMode, KeyFrame};
use crate::reactive::{LightweightSubject, Observable};
use crate::controls::ResourceKey;
use crate::data::core::Value;
use crate::data::model::Model;
use crate::data::{BindingBase, ReflectionBinding};
use crate::styling::{ControlTheme, Selectors, Setter, SetterValue, Style};

fn create_opacity_animation(from: SetterValue, to: SetterValue) -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.set_fill_mode(FillMode::Both);
    animation.children().add(KeyFrame::with_key_time(
        seconds(0.0),
        [Setter::with_value(Visual::opacity_property(), from) as _],
    ));
    animation.children().add(KeyFrame::with_key_time(
        seconds(1.0),
        [Setter::with_value(Visual::opacity_property(), to) as _],
    ));
    animation
}

fn create_root(animation: &Ref<Animation>, selector: crate::styling::Selector) -> (Ref<TestRoot>, Ref<Border>) {
    let style = Style::with_selector(selector);
    style.add_animation(animation);
    let root = TestRoot::new();
    root.styles().add(style);
    let target = Border::new();
    root.set_child(&target);
    (root, target)
}

/// A root whose resources hold a control theme for borders with
/// `animation`, and a border in it.
fn create_themed_root(animation: &Ref<Animation>) -> (Ref<TestRoot>, Ref<Border>) {
    let theme = ControlTheme::for_type::<Border>();
    theme.add_animation(animation);
    let root = TestRoot::new();
    let theme: BoxedValue = Rc::new(theme);
    root.resources().add(ResourceKey::Type(Border::TYPE), Some(theme));
    let target = Border::new();
    root.set_child(&target);
    (root, target)
}

#[test]
fn control_theme_applies_animation() {
    let clock = start();
    let animation =
        create_opacity_animation(SetterValue::Value(Rc::new(1.0_f64)), SetterValue::Value(Rc::new(0.0_f64)));
    let (_root, target) = create_themed_root(&animation);

    clock.pulse(TimeSpan::ZERO);
    assert_eq!(target.opacity(), 1.0);

    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    clock.pulse(seconds(1.0));
    assert_eq!(target.opacity(), 0.0);
}

struct KeyFrameValues {
    from: f64,
    to: f64,
}

crate::ferro_model!(KeyFrameValues, |b| b
    .read_only::<Value<f64>>("From", |vm| vm.from)
    .read_only::<Value<f64>>("To", |vm| vm.to));

#[test]
fn control_theme_applies_animation_with_bound_key_frame_values() {
    let clock = start();
    let from_binding: Rc<dyn BindingBase> = ReflectionBinding::new("From");
    let to_binding: Rc<dyn BindingBase> = ReflectionBinding::new("To");
    let animation =
        create_opacity_animation(SetterValue::BindingBase(from_binding), SetterValue::BindingBase(to_binding));

    let theme = ControlTheme::for_type::<Border>();
    theme.add_animation(&animation);
    let root = TestRoot::new();
    let theme: BoxedValue = Rc::new(theme);
    root.resources().add(ResourceKey::Type(Border::TYPE), Some(theme));
    let target = Border::new();
    let values: BoxedValue = Model::new_model(KeyFrameValues { from: 1.0, to: 0.0 });
    target.set_data_context(Some(values));
    root.set_child(&target);

    clock.pulse(TimeSpan::ZERO);
    assert_eq!(target.opacity(), 1.0);

    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    clock.pulse(seconds(1.0));
    assert_eq!(target.opacity(), 0.0);
}

#[test]
fn style_applies_animation_with_observable_key_frame_values() {
    let clock = start();
    let from: Rc<dyn crate::reactive::IObservable<BoxedValue>> = Observable::single_value(Rc::new(1.0_f64) as BoxedValue);
    let to: Rc<dyn crate::reactive::IObservable<BoxedValue>> = Observable::single_value(Rc::new(0.0_f64) as BoxedValue);
    let animation = create_opacity_animation(SetterValue::Binding(from), SetterValue::Binding(to));
    let (_root, target) = create_root(&animation, Selectors::of_type::<Border>());

    clock.pulse(TimeSpan::ZERO);
    assert_eq!(target.opacity(), 1.0);

    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    clock.pulse(seconds(1.0));
    assert_eq!(target.opacity(), 0.0);
}

#[test]
fn style_applies_animation() {
    let clock = start();
    let animation =
        create_opacity_animation(SetterValue::Value(Rc::new(1.0_f64)), SetterValue::Value(Rc::new(0.0_f64)));
    let (_root, target) = create_root(&animation, Selectors::of_type::<Border>());

    clock.pulse(TimeSpan::ZERO);
    assert_eq!(target.opacity(), 1.0);

    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    clock.pulse(seconds(1.0));
    assert_eq!(target.opacity(), 0.0);
}

#[test]
fn bound_key_frame_value_follows_its_source() {
    let clock = start();
    let to = LightweightSubject::<BoxedValue>::new();
    let animation = create_opacity_animation(
        SetterValue::Value(Rc::new(1.0_f64)),
        SetterValue::Binding(Rc::new(to.clone())),
    );
    animation.set_fill_mode(FillMode::None);
    let (_root, target) = create_root(&animation, Selectors::of_type::<Border>());

    use crate::reactive::IObserver;
    to.on_next(Rc::new(0.5_f64));
    clock.pulse(TimeSpan::ZERO);
    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 0.75);

    to.on_next(Rc::new(0.0_f64));
    clock.pulse(seconds(0.75));
    assert_eq!(target.opacity(), 0.25);
}

#[test]
fn animation_of_style_with_activator_runs_while_the_style_is_active() {
    let clock = start();
    let animation =
        create_opacity_animation(SetterValue::Value(Rc::new(1.0_f64)), SetterValue::Value(Rc::new(0.0_f64)));
    animation.set_fill_mode(FillMode::None);
    let (_root, target) = create_root(&animation, Selectors::of_type::<Border>().class("foo"));

    clock.pulse(TimeSpan::ZERO);
    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 1.0);
    assert!(!target.is_animating(Visual::opacity_property()));

    target.classes().add("foo");
    clock.pulse(seconds(1.0));
    clock.pulse(seconds(1.5));
    assert_eq!(target.opacity(), 0.5);

    target.classes().remove("foo");
    assert_eq!(target.opacity(), 1.0);
    assert!(!target.is_animating(Visual::opacity_property()));

    clock.pulse(seconds(1.75));
    assert_eq!(target.opacity(), 1.0);
}

#[test]
fn animations_are_disposed_when_the_style_is_detached() {
    let clock = start();
    let animation =
        create_opacity_animation(SetterValue::Value(Rc::new(1.0_f64)), SetterValue::Value(Rc::new(0.0_f64)));
    animation.set_iteration_count(crate::animation::IterationCount::INFINITE);
    animation.set_fill_mode(FillMode::None);
    let (root, target) = create_root(&animation, Selectors::of_type::<Border>());

    clock.pulse(TimeSpan::ZERO);
    clock.pulse(seconds(0.5));
    assert_eq!(target.opacity(), 0.5);

    root.styles().clear();

    assert_eq!(target.opacity(), 1.0);
    clock.pulse(seconds(0.75));
    assert_eq!(target.opacity(), 1.0);
    assert!(!clock.has_observers());
}

// --- The animation tests of the upstream style tests (`Styling/StyleTests.cs`) ---
//
// They are here because they need the clocks of the animation tests. `Class1`
// is the class of the styling tests.

use crate::animation::Cue;
use crate::media::Brushes;
use crate::styling::test_support::Class1;
use crate::styling::testing::try_attach;

fn green() -> Rc<dyn IBrush> {
    Brushes::green()
}

fn yellow() -> Rc<dyn IBrush> {
    Brushes::yellow()
}

fn blue() -> Rc<dyn IBrush> {
    Brushes::blue()
}

fn double_animation() -> Ref<Animation> {
    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    let first = KeyFrame::new();
    first.setters().add(Setter::new(Class1::double_property(), 5.0) as _);
    animation.children().add(first);
    animation
        .children()
        .add(KeyFrame::with_cue(Cue::new(1.0), [Setter::new(Class1::double_property(), 10.0) as _]));
    animation
}

#[test]
fn animations_should_be_activated() {
    let style = Style::with_selector(Selectors::of_type::<Class1>());
    style.add_animation(&double_animation());

    let clock = TestClock::new();
    let target = Class1::new();
    target.set_clock(Some(clock.as_clock()));

    try_attach(&style, &target, None);

    assert_eq!(0.0, target.double());

    clock.step(TimeSpan::ZERO);
    assert_eq!(5.0, target.double());

    clock.step(seconds(0.5));
    assert_eq!(7.5, target.double());
}

#[test]
fn animations_with_trigger_should_be_activated_and_deactivated() {
    let style = Style::with_selector(Selectors::of_type::<Class1>().class("foo"));
    style.add_animation(&double_animation());

    let clock = TestClock::new();
    let target = Class1::new();
    target.set_clock(Some(clock.as_clock()));

    try_attach(&style, &target, None);

    assert_eq!(0.0, target.double());

    target.classes().add("foo");
    clock.step(TimeSpan::ZERO);
    assert_eq!(5.0, target.double());

    clock.step(seconds(0.5));
    assert_eq!(7.5, target.double());

    target.classes().remove("foo");
    assert_eq!(0.0, target.double());
}

#[test]
fn animations_with_activator_trigger_should_be_activated_and_deactivated() {
    let clock = TestClock::new();
    let border = Border::new();

    let root = TestRoot::new();
    root.set_clock(Some(clock.as_clock()));

    let animation = Animation::new();
    animation.set_duration(seconds(1.0));
    animation.children().add(KeyFrame::with_cue(
        Cue::new(0.0),
        [Setter::new(Border::background_property(), Some(green())) as _],
    ));
    animation.children().add(KeyFrame::with_cue(
        Cue::new(1.0),
        [Setter::new(Border::background_property(), Some(green())) as _],
    ));
    let style = Style::with_setters(
        Selectors::of_type::<Border>().not(Selectors::class(None, "foo")),
        [Setter::new(Border::background_property(), Some(yellow()))],
    );
    style.add_animation(&animation);
    root.styles().add(style);
    root.styles().add(Style::with_setters(
        Selectors::of_type::<Border>().class("foo"),
        [Setter::new(Border::background_property(), Some(blue()))],
    ));
    root.set_child(&border);

    root.measure(Size::INFINITY);

    assert!(border.background() == Some(yellow()));

    clock.step(seconds(0.5));
    assert!(border.background() == Some(green()));

    border.classes().add("foo");
    assert!(border.background() == Some(blue()));
}
