use super::*;
use crate::animation::transitions::DoubleTransition;
use crate::animation::{TransitionInstance, Transitions};
use crate::reactive::AnonymousObserver;

fn opacity_transitions() -> Transitions {
    let transition = DoubleTransition::new();
    transition.set_duration(seconds(1.0));
    transition.set_property(Some(Visual::opacity_property()));
    Transitions::from_items([transition.into()])
}

#[test]
fn check_transitions_interpolation_negative_bounds_clamp() {
    let clock = TestClock::new();
    let border = Border::new();
    border.set_transitions(Some(opacity_transitions()));

    border.set_opacity(0.0);

    clock.pulse(seconds(0.0));
    clock.pulse(seconds(-0.5));

    assert_eq!(border.opacity(), 0.0);
}

#[test]
fn check_transitions_interpolation_positive_bounds_clamp() {
    let clock = TestClock::new();
    let border = Border::new();
    border.set_transitions(Some(opacity_transitions()));

    border.set_opacity(0.0);

    clock.pulse(seconds(0.0));
    clock.pulse(SignedDuration::from_milliseconds(1001.0));

    assert_eq!(border.opacity(), 0.0);
}

fn record(
    instance: &Rc<TransitionInstance>,
) -> (Rc<RefCell<Vec<f64>>>, Rc<Cell<bool>>) {
    let values = Rc::new(RefCell::new(Vec::new()));
    let completed = Rc::new(Cell::new(false));
    let (v, c) = (values.clone(), completed.clone());
    instance.subscribe(Rc::new(
        AnonymousObserver::new(move |value| v.borrow_mut().push(value)).with_completed(move || c.set(true)),
    ));
    (values, completed)
}

#[test]
fn transition_instance_with_zero_duration_is_completed_on_first_tick() {
    let clock = TestClock::new();
    let (values, completed) = record(&TransitionInstance::new(clock.as_clock(), SignedDuration::ZERO, SignedDuration::ZERO));

    clock.pulse(SignedDuration::from_milliseconds(10.0));

    assert_eq!(*values.borrow(), vec![0.0, 1.0]);
    assert!(completed.get());
}

#[test]
fn transition_instance_properly_calculates_delay_and_duration_values() {
    let clock = TestClock::new();
    let (values, completed) = record(&TransitionInstance::new(
        clock.as_clock(),
        SignedDuration::from_milliseconds(30.0),
        SignedDuration::from_milliseconds(70.0),
    ));

    for _ in 0..=10 {
        clock.pulse(SignedDuration::from_milliseconds(10.0));
    }

    let round = |v: f64| (v * 10_000.0).round() / 10_000.0;
    let values: Vec<f64> = values.borrow().iter().map(|v| round(*v)).collect();
    // The value published on subscription, then one per tick: the clock of
    // the transition starts at zero on the first tick.
    let mut expected = vec![0.0, 0.0, 0.0, 0.0, 0.0];
    expected.extend((1..=6).map(|i| round(i as f64 * 10.0 / 70.0)));
    expected.push(1.0);
    assert_eq!(values, expected);
    assert!(completed.get());
}

#[test]
fn transition_instance_with_delay_but_zero_duration_is_completed_after_delay() {
    let clock = TestClock::new();
    let (values, completed) =
        record(&TransitionInstance::new(clock.as_clock(), SignedDuration::from_milliseconds(30.0), SignedDuration::ZERO));

    for _ in 0..=4 {
        clock.pulse(SignedDuration::from_milliseconds(10.0));
    }

    // One iteration sooner than the test above, because the start of the
    // transition is also the end.
    assert_eq!(*values.borrow(), vec![0.0, 0.0, 0.0, 0.0, 1.0]);
    assert!(completed.get());
}

#[test]
fn transitions_reject_direct_properties() {
    let transition = MockTransition::new(Visual::bounds_property());
    let transitions = Transitions::new();
    assert_panics(|| transitions.add(transition.handle()));
}

#[test]
fn transition_rejects_property_of_another_type() {
    let transition = DoubleTransition::new();
    assert_panics(|| transition.set_property(Some(Visual::is_visible_property())));
}

#[test]
fn double_transition_animates_attached_control() {
    start();
    let clock = TestClock::new();
    let border = Border::new();
    border.set_transitions(Some(opacity_transitions()));
    let root = TestRoot::with_child(&border);
    root.set_clock(Some(clock.as_clock()));

    border.set_opacity(0.0);
    assert_eq!(border.opacity(), 1.0);

    clock.step(seconds(0.0));
    clock.step(seconds(0.25));
    assert_eq!(border.opacity(), 0.75);
    assert!(border.is_animating(Visual::opacity_property()));

    clock.step(seconds(1.0));
    assert_eq!(border.opacity(), 0.0);
    assert!(!border.is_animating(Visual::opacity_property()));
}

#[test]
fn only_an_object_with_transitions_keeps_data_for_them() {
    // Two flags and one word beside the object: what transitions need is behind the word.
    let words = std::mem::size_of::<usize>();
    assert!(std::mem::size_of::<crate::animation::Animatable>() <= std::mem::size_of::<FerroObject>() + 2 * words);

    let _clock = TestClock::new();
    let border = Border::new();

    // A property change, a detach and an attach, and a report outside a transition make no data
    // on an object without transitions.
    border.set_opacity(0.5);
    border.disable_transitions();
    border.enable_transitions();
    border.report_transition_interruption_state(Rc::new(1.0_f64));
    assert!(border.take_transition_interruption_state::<f64>().is_none());
    assert!(!border.has_transition_data());

    // The first transitions the object is given make it, and the transitions run with it.
    let transitions = opacity_transitions();
    let transition = transitions.snapshot()[0].clone();
    border.set_transitions(Some(transitions));
    assert!(border.has_transition_data());
    assert!(border.try_get_transition_instance(&transition).is_none());

    border.set_opacity(0.0);
    assert!(border.try_get_transition_instance(&transition).is_some());

    // Without its transitions the object keeps the data and runs none.
    border.set_transitions(None);
    assert!(border.try_get_transition_instance(&transition).is_none());
    border.set_opacity(1.0);
    assert_eq!(border.opacity(), 1.0);
}
