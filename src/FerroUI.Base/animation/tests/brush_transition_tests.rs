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
