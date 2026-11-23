use crate::test_support::test_scope;
use crate::{Slider, SliderSnapsTo};
use ferroui_base::layout::Orientation;

#[test]
fn default_orientation_should_be_horizontal() {
    let _scope = test_scope();
    let slider = Slider::new();
    assert_eq!(Orientation::Horizontal, slider.orientation());
}

#[test]
fn should_set_horizontal_class() {
    let _scope = test_scope();
    let slider = Slider::new();
    slider.set_orientation(Orientation::Horizontal);

    assert!(slider.classes().contains(":horizontal"));
}

#[test]
fn should_set_vertical_class() {
    let _scope = test_scope();
    let slider = Slider::new();
    slider.set_orientation(Orientation::Vertical);

    assert!(slider.classes().contains(":vertical"));
}

fn ranged(minimum: f64, maximum: f64) -> ferroui_base::Ref<Slider> {
    let slider = Slider::new();
    slider.set_minimum(minimum);
    slider.set_maximum(maximum);
    slider
}

#[test]
fn a_slider_has_no_steps_by_default() {
    let _scope = test_scope();
    let slider = ranged(0.0, 100.0);
    assert_eq!(0.0, slider.step_frequency());
    assert_eq!(SliderSnapsTo::StepValues, slider.snaps_to());
    assert_eq!(33.3, slider.snap_to_tick(33.3));
}

#[test]
fn step_values_count_from_the_minimum() {
    let _scope = test_scope();
    let slider = ranged(5.0, 100.0);
    slider.set_step_frequency(10.0);

    assert_eq!(15.0, slider.snap_to_tick(17.0));
    assert_eq!(25.0, slider.snap_to_tick(20.0));
    assert_eq!(5.0, slider.snap_to_tick(5.0));
}

#[test]
fn the_last_step_stops_at_the_maximum() {
    let _scope = test_scope();
    let slider = ranged(0.0, 95.0);
    slider.set_step_frequency(10.0);

    assert_eq!(95.0, slider.snap_to_tick(94.0));
    assert_eq!(90.0, slider.snap_to_tick(91.0));
}

#[test]
fn snapping_to_ticks_goes_before_the_steps() {
    let _scope = test_scope();
    let slider = ranged(0.0, 100.0);
    slider.set_step_frequency(1.0);
    slider.set_tick_frequency(20.0);

    assert_eq!(33.0, slider.snap_to_tick(33.2));
    slider.set_snaps_to(SliderSnapsTo::Ticks);
    assert_eq!(40.0, slider.snap_to_tick(33.2));
}

#[test]
fn an_invalid_step_is_no_step() {
    let _scope = test_scope();
    let slider = ranged(0.0, 100.0);
    for step in [f64::NAN, f64::INFINITY, -5.0] {
        slider.set_step_frequency(step);
        assert_eq!(33.3, slider.snap_to_tick(33.3));
    }
}
