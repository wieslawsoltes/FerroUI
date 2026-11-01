use super::*;
use crate::animation::transitions::BrushTransition;
use crate::animation::TransitionBaseImpl;
use crate::media::{LinearGradientBrush, SolidColorBrush};

fn solid(opacity: f64) -> Rc<dyn IBrush> {
    let brush = SolidColorBrush::new();
    brush.set_opacity(opacity);
    brush.into()
}

fn linear(opacity: f64) -> Rc<dyn IBrush> {
    let brush = LinearGradientBrush::new();
    brush.set_opacity(opacity);
    brush.into()
}

fn test(progress: f64, old_brush: Rc<dyn IBrush>, new_brush: Rc<dyn IBrush>) {
    let clock = TestClock::new();
    let border = Border::new();
    border.set_background(Some(old_brush.clone()));
    let sut = BrushTransition::new();
    sut.set_duration(seconds(1.0));
    sut.set_property(Some(Border::background_property()));

    let old_value: BoxedValue = Rc::new(Some(old_brush.clone()));
    let new_value: BoxedValue = Rc::new(Some(new_brush.clone()));
    <BrushTransition as TransitionBaseImpl>::apply(&sut, &border, clock.as_clock(), &old_value, &new_value);
    clock.pulse(TimeSpan::ZERO);
    clock.pulse(sut.duration() * progress);

    let background = border.background().expect("a background");
    assert_eq!(
        old_brush.opacity() + (new_brush.opacity() - old_brush.opacity()) * progress,
        background.opacity()
    );
}

#[test]
fn solid_color_brush_opacity_is_interoplated() {
    test(0.0, solid(0.0), solid(0.0));
    test(0.0, solid(0.0), solid(1.0));
    test(0.5, solid(0.0), solid(1.0));
    test(0.5, solid(0.5), solid(0.5));
    test(1.0, solid(1.0), solid(1.0));
}

#[test]
fn linear_gradient_brush_opacity_is_interoplated() {
    test(0.0, linear(0.0), linear(0.0));
    test(0.0, linear(0.0), linear(1.0));
    test(0.5, linear(0.0), linear(1.0));
    test(0.5, linear(0.5), linear(0.5));
    test(1.0, linear(1.0), linear(1.0));
}

// --- SolidColorBrushTransition: the rules of WinUI's BrushTransition (tests of this port) -------

use crate::animation::transitions::{DoubleTransition, SolidColorBrushTransition};
use crate::media::Color;

/// Applies a solid colour brush transition of one second (after `delay`) from `old_brush` to
/// `new_brush` and returns the border and the clock; the clock has had its first tick.
fn apply_solid(
    old_brush: Option<Rc<dyn IBrush>>,
    new_brush: Option<Rc<dyn IBrush>>,
    delay: TimeSpan,
) -> (Ref<Border>, Rc<TestClock>) {
    let clock = TestClock::new();
    let border = Border::new();
    border.set_background(old_brush.clone());
    let sut = SolidColorBrushTransition::new();
    sut.set_duration(seconds(1.0));
    sut.set_delay(delay);
    sut.set_property(Some(Border::background_property()));
    let old_value: BoxedValue = Rc::new(old_brush);
    let new_value: BoxedValue = Rc::new(new_brush);
    <SolidColorBrushTransition as TransitionBaseImpl>::apply(&sut, &border, clock.as_clock(), &old_value, &new_value);
    clock.pulse(TimeSpan::ZERO);
    (border, clock)
}

fn same(a: &Option<Rc<dyn IBrush>>, b: &Rc<dyn IBrush>) -> bool {
    a.as_ref().is_some_and(|a| Rc::ptr_eq(a, b))
}

#[test]
fn solid_color_brush_transition_interpolates_solid_color_brushes() {
    let (border, clock) = apply_solid(Some(solid(0.0)), Some(solid(1.0)), TimeSpan::ZERO);
    clock.pulse(seconds(0.5));
    assert_eq!(0.5, border.background().expect("a background").opacity());
}

#[test]
fn solid_color_brush_transition_fades_in_from_no_brush() {
    let brush = SolidColorBrush::new();
    brush.set_color(Color::from_argb(255, 10, 20, 30));
    let (border, clock) = apply_solid(None, Some(brush.into()), TimeSpan::ZERO);
    clock.pulse(seconds(0.5));
    let background = border.background().expect("a background");
    let color = background.as_solid_color_brush().expect("a solid colour brush").color();
    assert!((120..=135).contains(&color.a), "half-way alpha, got {}", color.a);
    assert_eq!((10, 20, 30), (color.r, color.g, color.b));
}

#[test]
fn solid_color_brush_transition_changes_other_brushes_at_once() {
    // To a brush that is not a solid colour brush (an acrylic or gradient background): at once.
    let gradient = linear(1.0);
    let (border, _clock) = apply_solid(Some(solid(0.0)), Some(gradient.clone()), TimeSpan::ZERO);
    assert!(same(&border.background(), &gradient), "the new brush on the first frame");

    // From one: at once as well.
    let target = solid(1.0);
    let (border, _clock) = apply_solid(Some(linear(1.0)), Some(target.clone()), TimeSpan::ZERO);
    assert!(same(&border.background(), &target));

    // To no brush: at once.
    let (border, _clock) = apply_solid(Some(solid(1.0)), None, TimeSpan::ZERO);
    assert!(border.background().is_none());
}

#[test]
fn solid_color_brush_transition_holds_the_old_brush_for_the_delay() {
    let old = linear(1.0);
    let (border, clock) = apply_solid(Some(old.clone()), Some(solid(0.0)), seconds(0.12));
    clock.pulse(seconds(0.1));
    assert!(same(&border.background(), &old), "the old brush during the delay");
    clock.pulse(seconds(0.05));
    assert!(!same(&border.background(), &old), "the new brush once the delay has passed");
}

#[test]
fn brush_transition_switches_incompatible_brushes_half_way() {
    // The upstream rule BrushTransition keeps (and SolidColorBrushTransition replaces): no brush
    // to a gradient brush switches half-way through the duration.
    let clock = TestClock::new();
    let border = Border::new();
    let new = linear(1.0);
    let sut = BrushTransition::new();
    sut.set_duration(seconds(1.0));
    sut.set_property(Some(Border::background_property()));
    let old_value: BoxedValue = Rc::new(None::<Rc<dyn IBrush>>);
    let new_value: BoxedValue = Rc::new(Some(new.clone()));
    <BrushTransition as TransitionBaseImpl>::apply(&sut, &border, clock.as_clock(), &old_value, &new_value);
    clock.pulse(TimeSpan::ZERO);
    clock.pulse(seconds(0.4));
    assert!(border.background().is_none());
    clock.pulse(seconds(0.2));
    assert!(same(&border.background(), &new));
}

#[test]
fn double_transition_does_not_interpolate_not_a_number() {
    // Width NaN (automatic) to 0, as the pane of a split view when its template is applied.
    for (old, new) in [(f64::NAN, 0.0), (0.0, f64::NAN)] {
        let clock = TestClock::new();
        let border = Border::new();
        let sut = DoubleTransition::new();
        sut.set_duration(seconds(1.0));
        sut.set_property(Some(Layoutable::width_property()));
        let old_value: BoxedValue = Rc::new(old);
        let new_value: BoxedValue = Rc::new(new);
        <DoubleTransition as TransitionBaseImpl>::apply(&sut, &border, clock.as_clock(), &old_value, &new_value);
        clock.pulse(TimeSpan::ZERO);
        clock.pulse(seconds(0.5));
        let value = border.width();
        assert!(value == new || (value.is_nan() && new.is_nan()), "{old} -> {new} gave {value}");
    }
}
