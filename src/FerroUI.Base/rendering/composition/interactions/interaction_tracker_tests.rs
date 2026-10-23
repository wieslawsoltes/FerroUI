//! Tests of the interaction tracker and of the visual interaction source.
//!
//! The tracker is driven by [`InteractionTracker::advance`] with fixed
//! steps, with its requests of animation frames turned off.

use super::interaction_tracker::natural_resting_value;
use super::*;
use crate::animation::easings::LinearEasing;
use crate::numerics::Vector3;
use crate::rendering::composition::animations::ICompositionAnimation;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::generated::ServerCompositionVisualProps;
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::CompositionGetValueStatus;
use crate::{Point, Vector3D};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// An owner that records its callbacks.
#[derive(Default)]
struct Recorder {
    states: RefCell<Vec<&'static str>>,
    ignored: RefCell<Vec<i32>>,
    inertia: RefCell<Vec<InteractionTrackerInertiaStateEnteredArgs>>,
    idle: RefCell<Vec<InteractionTrackerIdleStateEnteredArgs>>,
    values: RefCell<Vec<InteractionTrackerValuesChangedArgs>>,
}

impl IInteractionTrackerOwner for Recorder {
    fn custom_animation_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerCustomAnimationStateEnteredArgs,
    ) {
        self.states.borrow_mut().push("CustomAnimation");
    }

    fn idle_state_entered(&self, _sender: &Rc<InteractionTracker>, args: &InteractionTrackerIdleStateEnteredArgs) {
        self.states.borrow_mut().push("Idle");
        self.idle.borrow_mut().push(*args);
    }

    fn inertia_state_entered(&self, _sender: &Rc<InteractionTracker>, args: &InteractionTrackerInertiaStateEnteredArgs) {
        self.states.borrow_mut().push("Inertia");
        self.inertia.borrow_mut().push(*args);
    }

    fn interacting_state_entered(
        &self,
        _sender: &Rc<InteractionTracker>,
        _args: &InteractionTrackerInteractingStateEnteredArgs,
    ) {
        self.states.borrow_mut().push("Interacting");
    }

    fn request_ignored(&self, _sender: &Rc<InteractionTracker>, args: &InteractionTrackerRequestIgnoredArgs) {
        self.ignored.borrow_mut().push(args.request_id);
    }

    fn values_changed(&self, _sender: &Rc<InteractionTracker>, args: &InteractionTrackerValuesChangedArgs) {
        self.values.borrow_mut().push(*args);
    }
}

fn tracker_with_recorder(fixture: &TestCompositor) -> (Rc<InteractionTracker>, Rc<Recorder>) {
    let recorder = Rc::new(Recorder::default());
    let tracker = InteractionTracker::create_with_owner(&fixture.compositor, recorder.clone());
    tracker.set_frame_requests_enabled(false);
    (tracker, recorder)
}

/// Advances by frames of 1/60 s until the tracker is idle; panics after a
/// minute of frames.
fn run_until_idle(tracker: &InteractionTracker) {
    for _ in 0..3600 {
        if tracker.state() == InteractionTrackerState::Idle {
            return;
        }
        tracker.advance(Duration::from_secs_f64(1.0 / 60.0));
    }
    panic!("the tracker did not come to rest");
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!((actual - expected).abs() <= tolerance, "expected {expected}, got {actual}");
}

#[test]
fn try_update_position_clamps_and_reports_the_request_id() {
    let fixture = TestCompositor::new();
    let (tracker, recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(100.0, 200.0, 0.0));

    let first = tracker.try_update_position(Vector3::new(150.0, -10.0, 0.0));
    assert_eq!(tracker.position(), Vector3::new(100.0, 0.0, 0.0));
    assert_eq!(first, 1);
    let last = *recorder.values.borrow().last().expect("a values changed callback");
    assert_eq!(last.request_id, first);
    assert_eq!(last.position, Vector3::new(100.0, 0.0, 0.0));

    let second = tracker.try_update_position_with_option(
        Vector3::new(150.0, -10.0, 0.0),
        InteractionTrackerClampingOption::Disabled,
    );
    assert_eq!(second, 2);
    assert_eq!(tracker.position(), Vector3::new(150.0, -10.0, 0.0));

    let third = tracker.try_update_position_by(Vector3::new(-100.0, 50.0, 0.0));
    assert_eq!(third, 3);
    assert_eq!(tracker.position(), Vector3::new(50.0, 40.0, 0.0));

    // An idle tracker stays idle: no state callback for a jump.
    assert!(recorder.states.borrow().is_empty());
}

#[test]
fn requests_are_ignored_while_interacting() {
    let fixture = TestCompositor::new();
    let (tracker, recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 1000.0, 0.0));
    tracker.begin_interaction();
    let id = tracker.try_update_position(Vector3::new(0.0, 10.0, 0.0));
    assert_eq!(*recorder.ignored.borrow(), vec![id]);
    assert_eq!(tracker.position(), Vector3::ZERO);
}

#[test]
fn inertia_comes_to_rest_at_the_natural_resting_position() {
    let fixture = TestCompositor::new();
    let (tracker, recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 10_000.0, 0.0));

    let id = tracker.try_update_position_with_additional_velocity(Vector3::new(0.0, 1000.0, 0.0));
    assert_eq!(tracker.state(), InteractionTrackerState::Inertia);

    // The decay rate 0.95 keeps 5% of the velocity per second: the rest is
    // at v / ln(1 / 0.05) from the start.
    let expected = 1000.0 / (1.0f32 / 0.05).ln();
    assert_close(natural_resting_value(0.0, 1000.0, 0.05), expected, 0.001);
    assert_close(tracker.natural_resting_position().y, expected, 0.01);
    let args = recorder.inertia.borrow()[0];
    assert_eq!(args.request_id, id);
    assert!(args.is_inertia_from_impulse);
    assert_close(args.natural_resting_position.y, expected, 0.01);
    assert_close(args.modified_resting_position.expect("a modified resting position").y, expected, 0.01);

    // After a second, 5% of the velocity is left.
    tracker.advance(Duration::from_secs(1));
    assert_close(tracker.position_velocity_in_pixels_per_second().y, 50.0, 0.1);
    assert_close(tracker.position().y, expected * 0.95, 0.1);

    run_until_idle(&tracker);
    assert_close(tracker.position().y, expected, 0.001);
    assert_eq!(tracker.position_velocity_in_pixels_per_second(), Vector3::ZERO);
    assert!(tracker.is_position_rounding_suggested());
    assert_eq!(recorder.idle.borrow().last().map(|a| a.request_id), Some(id));
}

#[test]
fn inertia_is_clamped_to_the_boundaries() {
    let fixture = TestCompositor::new();
    let (tracker, _recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 100.0, 0.0));
    tracker.try_update_position_with_additional_velocity(Vector3::new(0.0, 1000.0, 0.0));
    run_until_idle(&tracker);
    assert_close(tracker.position().y, 100.0, 0.001);
}

#[test]
fn a_resting_value_modifier_snaps_to_a_multiple() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let (tracker, recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 10_000.0, 0.0));

    let modifier = InteractionTrackerInertiaRestingValue::create(compositor);
    modifier.set_condition(Some(compositor.create_expression_animation_with("this.Target.NaturalRestingPosition.Y > 0")));
    modifier.set_resting_value(Some(
        compositor.create_expression_animation_with("Round(this.Target.NaturalRestingPosition.Y / 100) * 100"),
    ));
    tracker.configure_position_y_inertia_modifiers(vec![InteractionTrackerInertiaModifier::from(modifier)]);

    tracker.try_update_position_with_additional_velocity(Vector3::new(0.0, 1000.0, 0.0));
    let args = recorder.inertia.borrow()[0];
    // The natural resting position (333.8) is snapped to 300.
    assert_close(args.natural_resting_position.y, 333.8, 0.1);
    assert_close(args.modified_resting_position.expect("a modified resting position").y, 300.0, 0.001);

    run_until_idle(&tracker);
    assert_close(tracker.position().y, 300.0, 0.001);
}

#[test]
fn the_property_set_has_the_values() {
    let fixture = TestCompositor::new();
    let (tracker, _recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(100.0, 100.0, 0.0));
    tracker.try_update_position(Vector3::new(10.0, 20.0, 0.0));

    let properties = tracker.properties();
    assert_eq!(
        properties.try_get_vector3("Position"),
        (CompositionGetValueStatus::Succeeded, Vector3::new(10.0, 20.0, 0.0))
    );
    assert_eq!(properties.try_get_scalar("Scale"), (CompositionGetValueStatus::Succeeded, 1.0));
    assert_eq!(
        properties.try_get_vector3("MaxPosition"),
        (CompositionGetValueStatus::Succeeded, Vector3::new(100.0, 100.0, 0.0))
    );
    assert_eq!(
        properties.try_get_boolean("IsPositionRoundingSuggested"),
        (CompositionGetValueStatus::Succeeded, true)
    );
}

#[test]
fn a_drag_goes_through_interacting_inertia_and_idle() {
    let fixture = TestCompositor::new();
    let (tracker, recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 10_000.0, 0.0));
    let visual = fixture.compositor.create_solid_color_visual();
    let source = VisualInteractionSource::create(&visual);
    source.set_position_y_source_mode(InteractionSourceMode::EnabledWithInertia);
    tracker.interaction_sources().add(source.clone());
    assert_eq!(tracker.interaction_sources().count(), 1);

    // Dragging the content up by 60 pixels in 30 ms.
    source.begin_manipulation(Point::new(0.0, 100.0), None, Vec::new());
    source.manipulation_moved(Point::new(0.0, 80.0), 10);
    source.manipulation_moved(Point::new(0.0, 60.0), 20);
    source.manipulation_moved(Point::new(0.0, 40.0), 30);
    assert_eq!(tracker.state(), InteractionTrackerState::Interacting);
    assert_close(tracker.position().y, 60.0, 0.001);
    assert_eq!(source.delta_position(), Vector3::new(0.0, 20.0, 0.0));

    source.manipulation_ended(Some(30));
    assert_eq!(tracker.state(), InteractionTrackerState::Inertia);
    let args = recorder.inertia.borrow()[0];
    assert_close(args.position_velocity_in_pixels_per_second.y, 2000.0, 0.5);
    assert!(!args.is_inertia_from_impulse);

    run_until_idle(&tracker);
    assert_eq!(*recorder.states.borrow(), vec!["Interacting", "Inertia", "Idle"]);
    assert_close(tracker.position().y, 60.0 + 2000.0 / (1.0f32 / 0.05).ln(), 0.01);
}

#[test]
fn a_drag_past_a_boundary_overpans_and_returns() {
    let fixture = TestCompositor::new();
    let (tracker, _recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 100.0, 0.0));
    let visual = fixture.compositor.create_solid_color_visual();
    let source = VisualInteractionSource::create(&visual);
    source.set_position_y_source_mode(InteractionSourceMode::EnabledWithoutInertia);
    tracker.interaction_sources().add(source.clone());

    source.begin_manipulation(Point::new(0.0, 300.0), None, Vec::new());
    source.manipulation_moved(Point::new(0.0, 100.0), 10);
    // 200 pixels pulled, 100 past the boundary: the rubber band shows less.
    let y = tracker.position().y;
    assert!(y > 100.0 && y < 200.0, "overpan {y}");

    source.manipulation_ended(Some(1000));
    run_until_idle(&tracker);
    assert_close(tracker.position().y, 100.0, 0.001);
}

#[test]
fn a_custom_animation_moves_the_position() {
    let fixture = TestCompositor::new();
    let (tracker, recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 1000.0, 0.0));
    let animation = fixture.compositor.create_vector3_key_frame_animation();
    animation.insert_key_frame_with_easing(1.0, Vector3::new(0.0, 100.0, 0.0), Rc::new(LinearEasing::new()));
    animation.set_duration(Duration::from_secs(1));

    let id = tracker.try_update_position_with_animation(animation.clone());
    assert_eq!(tracker.state(), InteractionTrackerState::CustomAnimation);
    tracker.advance(Duration::from_millis(500));
    assert_close(tracker.position().y, 50.0, 0.01);
    tracker.advance(Duration::from_millis(600));
    assert_eq!(tracker.state(), InteractionTrackerState::Idle);
    assert_close(tracker.position().y, 100.0, 0.001);
    assert_eq!(*recorder.states.borrow(), vec!["CustomAnimation", "Idle"]);
    assert_eq!(recorder.idle.borrow()[0].request_id, id);

    // A scalar animation is not an animation of the position.
    let scalar = fixture.compositor.create_scalar_key_frame_animation();
    let ignored = tracker.try_update_position_with_animation(scalar);
    assert_eq!(*recorder.ignored.borrow(), vec![ignored]);
}

#[test]
fn try_update_scale_keeps_the_center_in_place() {
    let fixture = TestCompositor::new();
    let (tracker, _recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_scale(4.0);
    tracker.set_max_position(Vector3::new(1000.0, 1000.0, 0.0));
    tracker.try_update_scale(2.0, Vector3::new(100.0, 50.0, 0.0));
    assert_eq!(tracker.scale(), 2.0);
    assert_eq!(tracker.position(), Vector3::new(100.0, 50.0, 0.0));
    // Clamped to MaxScale.
    tracker.try_update_scale(10.0, Vector3::ZERO);
    assert_eq!(tracker.scale(), 4.0);
}

#[test]
fn an_expression_animation_reads_the_tracker() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let (tracker, _recorder) = tracker_with_recorder(&fixture);
    tracker.set_max_position(Vector3::new(0.0, 100.0, 0.0));
    tracker.try_update_position(Vector3::new(0.0, 50.0, 0.0));
    let visual = compositor.create_solid_color_visual();
    fixture.run_jobs();

    let offset = compositor.create_expression_animation_with("-tracker.Position");
    offset.set_reference_parameter("tracker", tracker.clone());
    let instance = offset.create_instance(visual.server(), None).create();
    instance.resolve(compositor.server());
    instance.initialize(
        Duration::ZERO,
        ExpressionVariant::create(Vector3D::default()),
        ServerCompositionVisualProps::id_of_offset_property(),
    );
    assert_eq!(
        instance.evaluate(Duration::ZERO, ExpressionVariant::create(Vector3D::default())),
        ExpressionVariant::Vector3(Vector3::new(0.0, -50.0, 0.0))
    );

    let y = compositor.create_expression_animation_with("tracker.Position.Y * tracker.Scale");
    y.set_reference_parameter("tracker", tracker.clone());
    let instance = y.create_instance(visual.server(), None).create();
    instance.resolve(compositor.server());
    instance.initialize(
        Duration::ZERO,
        ExpressionVariant::create(0f32),
        ServerCompositionVisualProps::id_of_rotation_angle_property(),
    );
    assert_eq!(instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32)), ExpressionVariant::Double(50.0));
}
