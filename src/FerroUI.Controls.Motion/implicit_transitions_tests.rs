use crate::{ImplicitTransitions, ScalarTransition, Vector3Transition, Vector3TransitionComponents};
use ferroui_base::animation::{BrushTransition, SignedDuration};
use ferroui_base::numerics::Vector3;
use ferroui_base::Vector3D;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, Control, Grid};
use std::time::Duration;

fn milliseconds(value: i64) -> SignedDuration {
    SignedDuration::from_ticks(value * SignedDuration::TICKS_PER_MILLISECOND)
}

#[test]
fn a_transition_reaches_the_facade_of_the_element_and_leaves_it() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let element = Border::new();
    let scalar = ScalarTransition::new();
    assert_eq!(scalar.duration(), milliseconds(300));
    ImplicitTransitions::set_opacity_transition(&element, Some(scalar.clone()));
    ImplicitTransitions::set_rotation_transition(&element, Some(scalar.clone()));
    assert_eq!(element.opacity_transition().map(|value| value.duration()), Some(Duration::from_millis(300)));
    assert_eq!(element.rotation_transition().map(|value| value.duration()), Some(Duration::from_millis(300)));

    let vector = Vector3Transition::new();
    ImplicitTransitions::set_scale_transition(&element, Some(vector.clone()));
    ImplicitTransitions::set_translation_transition(&element, Some(vector.clone()));
    assert!(element.scale_transition().is_some() && element.translation_transition().is_some());

    ImplicitTransitions::set_opacity_transition(&element, None);
    ImplicitTransitions::set_scale_transition(&element, None);
    assert!(element.opacity_transition().is_none() && element.scale_transition().is_none());
    assert!(element.rotation_transition().is_some() && element.translation_transition().is_some());
}

#[test]
fn a_changed_duration_reaches_every_element_of_the_transition() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (first, second) = (Border::new(), Border::new());
    let transition = ScalarTransition::new();
    ImplicitTransitions::set_opacity_transition(&first, Some(transition.clone()));
    ImplicitTransitions::set_opacity_transition(&second, Some(transition.clone()));
    transition.set_duration(milliseconds(750));
    for element in [&first, &second] {
        assert_eq!(element.opacity_transition().map(|value| value.duration()), Some(Duration::from_millis(750)));
    }
    // An element that left the transition no longer follows it.
    ImplicitTransitions::set_opacity_transition(&second, None);
    transition.set_duration(milliseconds(100));
    assert_eq!(first.opacity_transition().map(|value| value.duration()), Some(Duration::from_millis(100)));
    assert!(second.opacity_transition().is_none());
}

#[test]
fn the_components_of_a_vector_transition_reach_the_facade() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let element = Border::new();
    let transition = Vector3Transition::new();
    assert_eq!(transition.components(), Vector3TransitionComponents::all());
    ImplicitTransitions::set_scale_transition(&element, Some(transition.clone()));
    transition.set_components(Vector3TransitionComponents::X | Vector3TransitionComponents::Z);
    let expected = ferroui_base::Vector3TransitionComponents::X | ferroui_base::Vector3TransitionComponents::Z;
    assert_eq!(element.scale_transition().map(|value| value.components()), Some(expected));
}

#[test]
fn rotation_and_scale_set_the_facades() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let element = Border::new();
    ImplicitTransitions::set_rotation(&element, 45.0);
    ImplicitTransitions::set_scale(&element, Vector3D::new(2.0, 3.0, 1.0));
    assert_eq!(element.rotation(), 45.0);
    assert_eq!(element.scale(), Vector3::new(2.0, 3.0, 1.0));
}

#[test]
fn a_brush_transition_runs_on_the_background_of_each_kind_of_element() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let border = Border::new();
    let grid = Grid::new();
    let presenter = ContentPresenter::new();
    let controls: [&Control; 3] = [&border, &grid, &presenter];
    for (control, background) in controls.into_iter().zip([
        Border::background_property().as_property(),
        Border::background_property().as_property(),
        ContentPresenter::background_property().as_property(),
    ]) {
        let transition = BrushTransition::new();
        ImplicitTransitions::set_background_transition(control, Some(transition.clone()));
        assert_eq!(transition.property(), Some(background));
        assert_eq!(transition.duration(), milliseconds(150), "the default of WinUI");
        assert_eq!(control.transitions().map(|list| list.count()), Some(1));
        ImplicitTransitions::set_background_transition(control, None);
        assert_eq!(control.transitions().map(|list| list.count()), Some(0));
    }
}
