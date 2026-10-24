use super::*;
use crate::{
    easing_functions::EasingFunctions,
    motion::{Animatable, Animation, PropertyError, PropertyValue},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn state_with(
    name: &str,
    setters: impl IntoIterator<Item = DrawnPropertySetter>,
) -> VisualStatesDictionary {
    let mut states = VisualStatesDictionary::new();
    states.insert(name, DrawnPropertiesDictionary::new(setters));
    states
}

#[derive(Default)]
struct RecordingBehavior {
    applied: Cell<usize>,
    removed: Cell<usize>,
    last_identity: RefCell<Option<Rc<()>>>,
}
impl RecordingBehavior {
    fn record(&self, target: &Animatable) {
        *self.last_identity.borrow_mut() = target
            .target_value("Identity")
            .unwrap()
            .downcast_ref::<Rc<()>>()
            .cloned();
    }
}
impl IStateBehavior for RecordingBehavior {
    fn on_state_applied(&self, target: &mut Animatable, _now_ms: f64) {
        self.applied.set(self.applied.get() + 1);
        self.record(target);
    }
    fn on_state_removed(&self, target: &mut Animatable, _now_ms: f64) {
        self.removed.set(self.removed.get() + 1);
        self.record(target);
    }
}
fn identified_target() -> (Animatable, Rc<()>) {
    let mut target = Animatable::new();
    let identity = Rc::new(());
    target.register_state("Identity", identity.clone()).unwrap();
    (target, identity)
}
fn states_with_behavior(behavior: Rc<dyn IStateBehavior>) -> VisualStatesDictionary {
    let mut states = VisualStatesDictionary::new();
    states.insert(
        "Hover",
        DrawnPropertiesDictionary::default().with_behavior(behavior),
    );
    states
}

// Direct fixtures from CoreObjectsTests/MotionlessStatePropertyTests.cs.
#[test]
fn source_set_state_applies_and_clear_state_restores_motionless_property() {
    let states = state_with("Hover", [DrawnPropertySetter::text("Label", "hovered")]);
    let mut target = Animatable::new();
    let label = target
        .register_state("Label", "default".to_owned())
        .unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), "hovered");
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), "default");
}
#[test]
fn source_clear_states_restores_motionless_property() {
    let states = state_with("Hover", [DrawnPropertySetter::text("Label", "hovered")]);
    let mut target = Animatable::new();
    let label = target
        .register_state("Label", "default".to_owned())
        .unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    states.clear_states(&mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), "default");
}
#[test]
fn source_layered_motionless_states_restore_in_order() {
    let mut states = state_with("Hover", [DrawnPropertySetter::text("Label", "hover")]);
    states.insert(
        "Selected",
        DrawnPropertiesDictionary::new([DrawnPropertySetter::text("Label", "selected")]),
    );
    let mut target = Animatable::new();
    let label = target
        .register_state("Label", "default".to_owned())
        .unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    states.set_state("Selected", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), "selected");
    states.clear_state("Selected", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), "hover");
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), "default");
}

// Direct fixtures from CoreObjectsTests/StateBehaviorTests.cs.
#[test]
fn source_behavior_fires_once_on_apply_and_once_on_remove() {
    let behavior = Rc::new(RecordingBehavior::default());
    let states = states_with_behavior(behavior.clone());
    let (mut target, identity) = identified_target();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(behavior.applied.get(), 1);
    assert_eq!(behavior.removed.get(), 0);
    assert!(Rc::ptr_eq(
        behavior.last_identity.borrow().as_ref().unwrap(),
        &identity
    ));
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(behavior.applied.get(), 1);
    assert_eq!(behavior.removed.get(), 1);
}
#[test]
fn source_behavior_not_removed_when_state_never_applied() {
    let behavior = Rc::new(RecordingBehavior::default());
    let states = states_with_behavior(behavior.clone());
    let (mut target, _) = identified_target();
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(behavior.applied.get(), 0);
    assert_eq!(behavior.removed.get(), 0);
}
#[test]
fn source_state_behavior_lambda_adapter_invokes_delegates() {
    let applied = Rc::new(Cell::new(0));
    let removed = Rc::new(Cell::new(0));
    let (mut target, identity) = identified_target();
    let expected = identity.clone();
    let applied_count = applied.clone();
    let removed_count = removed.clone();
    let behavior = StateBehavior::default()
        .applied(move |target, now_ms| {
            assert_eq!(now_ms, 25.0);
            assert!(Rc::ptr_eq(
                target
                    .target_value("Identity")
                    .unwrap()
                    .downcast_ref::<Rc<()>>()
                    .unwrap(),
                &expected,
            ));
            applied_count.set(applied_count.get() + 1);
        })
        .removed(move |_, now_ms| {
            assert_eq!(now_ms, 50.0);
            removed_count.set(removed_count.get() + 1);
        });
    let states = states_with_behavior(Rc::new(behavior));
    states.set_state("Hover", &mut target, 25.0).unwrap();
    assert_eq!(applied.get(), 1);
    states.clear_state("Hover", &mut target, 50.0).unwrap();
    assert_eq!(removed.get(), 1);
}
#[test]
fn source_state_behavior_null_delegates_are_no_ops() {
    let states = states_with_behavior(Rc::new(StateBehavior::default()));
    let mut target = Animatable::new();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert!(!target.is_state_active("Hover"));
}
#[test]
fn source_behavior_runs_alongside_value_setters() {
    let behavior = Rc::new(RecordingBehavior::default());
    let (mut target, _) = identified_target();
    target.register_motion("StrokeThickness", 2.0_f32).unwrap();
    let mut states = VisualStatesDictionary::new();
    states.insert(
        "Hover",
        DrawnPropertiesDictionary::new([DrawnPropertySetter::new("StrokeThickness", 10.0_f32)])
            .with_behavior(behavior.clone()),
    );
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(behavior.applied.get(), 1);
    assert_eq!(
        *target
            .target_value("StrokeThickness")
            .unwrap()
            .downcast_ref::<f32>()
            .unwrap(),
        10.0
    );
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(behavior.removed.get(), 1);
    assert_eq!(
        *target
            .target_value("StrokeThickness")
            .unwrap()
            .downcast_ref::<f32>()
            .unwrap(),
        2.0
    );
}
#[test]
fn source_clear_states_reverses_every_active_behavior() {
    let hover = Rc::new(RecordingBehavior::default());
    let selected = Rc::new(RecordingBehavior::default());
    let (mut target, _) = identified_target();
    let mut states = states_with_behavior(hover.clone());
    states.insert(
        "Selected",
        DrawnPropertiesDictionary::default().with_behavior(selected.clone()),
    );
    states.set_state("Hover", &mut target, 0.0).unwrap();
    states.set_state("Selected", &mut target, 0.0).unwrap();
    states.clear_states(&mut target, 0.0).unwrap();
    assert_eq!(hover.applied.get(), 1);
    assert_eq!(hover.removed.get(), 1);
    assert_eq!(selected.applied.get(), 1);
    assert_eq!(selected.removed.get(), 1);
}

#[test]
fn state_restores_motion_destination_while_retargeting_from_visible_value() {
    let states = state_with("Hover", [DrawnPropertySetter::new("X", 200.0_f64)]);
    let mut target = Animatable::new();
    let x = target.register_motion("X", 0.0_f64).unwrap();
    target
        .set_transition(Some(Animation::new(100.0, EasingFunctions::lineal)), &[])
        .unwrap();
    target.set(&x, 100.0, 0.0).unwrap();
    states.set_state("Hover", &mut target, 50.0).unwrap();
    assert_eq!(target.get(&x, 50.0).unwrap(), 50.0);
    assert_eq!(target.get(&x, 100.0).unwrap(), 125.0);
    states.clear_state("Hover", &mut target, 100.0).unwrap();
    assert_eq!(
        *target
            .target_value("X")
            .unwrap()
            .downcast_ref::<f64>()
            .unwrap(),
        100.0
    );
    assert_eq!(target.get(&x, 150.0).unwrap(), 112.5);
    assert_eq!(target.get(&x, 200.0).unwrap(), 100.0);
}

#[test]
fn reapplying_state_keeps_activation_order_and_each_property_restores_independently() {
    let mut states = state_with(
        "Hover",
        [
            DrawnPropertySetter::new("X", 10.0_f64),
            DrawnPropertySetter::new("Y", 20.0_f64),
        ],
    );
    states.insert(
        "Selected",
        DrawnPropertiesDictionary::new([DrawnPropertySetter::new("X", 30.0_f64)]),
    );
    let mut target = Animatable::new();
    let x = target.register_motion("X", 1.0_f64).unwrap();
    let y = target.register_motion("Y", 2.0_f64).unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    states.set_state("Selected", &mut target, 0.0).unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&x, 0.0).unwrap(), 10.0);
    assert_eq!(
        target.active_states().collect::<Vec<_>>(),
        ["Hover", "Selected"]
    );
    states.clear_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&x, 0.0).unwrap(), 30.0);
    assert_eq!(target.get(&y, 0.0).unwrap(), 2.0);
    states.clear_state("Selected", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&x, 0.0).unwrap(), 1.0);
}

#[test]
fn shared_definitions_keep_target_snapshots_and_reference_identity_independent() {
    let old1 = Rc::new(Cell::new(10_u8));
    let old2 = Rc::new(Cell::new(20_u8));
    let hover = Rc::new(Cell::new(30_u8));
    let states = state_with(
        "Hover",
        [DrawnPropertySetter::new("Fill", Some(hover.clone()))],
    );
    let mut first = Animatable::new();
    let mut second = Animatable::new();
    let fill1 = first.register_state("Fill", Some(old1.clone())).unwrap();
    let fill2 = second.register_state("Fill", Some(old2.clone())).unwrap();
    states.set_state("Hover", &mut first, 0.0).unwrap();
    states.set_state("Hover", &mut second, 0.0).unwrap();
    states.clear_state("Hover", &mut first, 0.0).unwrap();
    assert!(Rc::ptr_eq(&first.get(&fill1, 0.0).unwrap().unwrap(), &old1));
    assert!(Rc::ptr_eq(
        &second.get(&fill2, 0.0).unwrap().unwrap(),
        &hover
    ));
    states.clear_states(&mut second, 0.0).unwrap();
    assert!(Rc::ptr_eq(
        &second.get(&fill2, 0.0).unwrap().unwrap(),
        &old2
    ));
    assert_eq!(old1.get(), 10);
    assert_eq!(old2.get(), 20);
}

#[test]
fn theme_definitions_cannot_override_user_definitions() {
    let mut states = VisualStatesDictionary::new();
    let state = |value, origin| {
        DrawnPropertiesDictionary::new([DrawnPropertySetter::new("X", value)]).with_origin(origin)
    };
    assert!(states.insert("Hover", state(1.0_f64, StateOrigin::Theme)));
    assert!(states.insert("Hover", state(2.0_f64, StateOrigin::Theme)));
    assert!(states.insert("Hover", state(3.0_f64, StateOrigin::User)));
    assert!(!states.insert("Hover", state(4.0_f64, StateOrigin::Theme)));
    assert!(states.insert("Hover", state(5.0_f64, StateOrigin::User)));
    let mut target = Animatable::new();
    let x = target.register_motion("X", 0.0_f64).unwrap();
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&x, 0.0).unwrap(), 5.0);
}

#[test]
fn unknown_properties_are_ignored_and_invalid_known_types_are_atomic() {
    let mut target = Animatable::new();
    let x = target.register_motion("X", 2.0_f64).unwrap();
    let states = state_with(
        "Hover",
        [
            DrawnPropertySetter::text("Unknown", "ignored"),
            DrawnPropertySetter::text("X", "12.5"),
        ],
    );
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&x, 0.0).unwrap(), 12.5);
    states.clear_states(&mut target, 0.0).unwrap();
    assert_eq!(target.get(&x, 0.0).unwrap(), 2.0);
    target.register_state("Width", 1.0_f64).unwrap();
    let invalid = state_with(
        "Hover",
        [
            DrawnPropertySetter::new("X", 99.0_f64),
            DrawnPropertySetter::new("Width", false),
        ],
    );
    assert!(matches!(
        invalid.set_state("Hover", &mut target, 0.0),
        Err(StateError::Property(PropertyError::TypeMismatch { .. }))
    ));
    assert_eq!(target.get(&x, 0.0).unwrap(), 2.0);
    assert!(!target.is_state_active("Hover"));
    assert!(target.states.original_values.is_empty());
}

#[test]
fn optional_values_restore_none_and_custom_parsers_are_target_local() {
    let mut target = Animatable::new();
    let label = target.register_state("Label", None::<String>).unwrap();
    let states = state_with(
        "Hover",
        [DrawnPropertySetter::new("Label", Some("hover".to_owned()))],
    );
    states.set_state("Hover", &mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap().as_deref(), Some("hover"));
    states.clear_states(&mut target, 0.0).unwrap();
    assert_eq!(target.get(&label, 0.0).unwrap(), None);
    target.register_parser(|s| Some(Some(format!("parsed:{s}"))));
    target
        .set_value("Label", PropertyValue::from("text"), 0.0)
        .unwrap();
    assert_eq!(
        target.get(&label, 0.0).unwrap().as_deref(),
        Some("parsed:text")
    );
}
