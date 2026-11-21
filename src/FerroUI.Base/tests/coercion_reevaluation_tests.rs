//! A command-controlled property must not poll user code for unchanged frames.
use super::*;
use crate::{data::BindingPriority, ferro_property_metadata::PropertyMetadata, *};

test_class!(ScopeOwner: FerroObject);
test_class!(DerivedScopeOwner: ScopeOwner);

fn gated_property() -> (&'static StyledProperty<bool>, Rc<Cell<bool>>) {
    let blocked = Rc::new(Cell::new(true));
    let gate = blocked.clone();
    let property = StyledProperty::create(
        "GatedDefault",
        ScopeOwner::TYPE,
        ScopeOwner::TYPE,
        StyledPropertyMetadata::new(Some(true)).with_coerce(move |_, value| value && !gate.get()),
        false,
        None,
        None,
        false,
    );
    (property, blocked)
}

#[test]
fn coerced_default_replaces_retired_style_input() {
    let (property, blocked) = gated_property();
    let owner = ScopeOwner::new();
    let style = owner
        .set_value_with_priority(property, false, BindingPriority::Style)
        .unwrap();
    style.dispose();
    assert!(!owner.get_value(property));
    assert_eq!(owner.get_base_value(property), None);
    blocked.set(false);
    owner.coerce_value(property.as_property());
    assert!(owner.get_value(property));
    assert_eq!(owner.get_local_value(property), None);
}

#[test]
fn coerced_default_replaces_retired_binding_input() {
    let (property, blocked) = gated_property();
    let owner = ScopeOwner::new();
    let source = Subject::behavior(false);
    let binding = owner.bind(property, source.observable(), BindingPriority::Style);
    binding.dispose();
    assert!(!owner.get_value(property));
    assert_eq!(owner.get_base_value(property), None);
    blocked.set(false);
    owner.coerce_value(property.as_property());
    assert!(owner.get_value(property));
    source.on_next(false);
    assert!(owner.get_value(property));
}

#[test]
fn coerced_default_replaces_cleared_local_input() {
    let (property, blocked) = gated_property();
    let owner = ScopeOwner::new();
    owner.set_value(property, false);
    owner.clear_value(property);
    assert!(!owner.get_value(property));
    blocked.set(false);
    owner.coerce_value(property.as_property());
    assert!(owner.get_value(property));
    assert_eq!(owner.get_local_value(property), None);
}

fn property(reevaluate: bool) -> (&'static StyledProperty<bool>, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let property = StyledProperty::create(
        "Enabled",
        ScopeOwner::TYPE,
        ScopeOwner::TYPE,
        StyledPropertyMetadata::new(Some(true))
            .with_coerce(move |_, _| {
                count.set(count.get() + 1);
                false
            })
            .with_coerce_on_reevaluation(reevaluate),
        false,
        None,
        None,
        false,
    );
    (property, calls)
}

#[test]
fn explicit_policy_keeps_coerced_default_without_polling_unchanged_frames() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    owner.coerce_value(property.as_property());
    assert_eq!(calls.get(), 1);
    assert!(!owner.get_value(property));
    for _ in 0..3 {
        owner.values().begin_styling();
        owner.values().end_styling(&owner);
        assert_eq!(calls.get(), 1);
        assert!(!owner.get_value(property));
    }
    owner.coerce_value(property.as_property());
    assert_eq!(calls.get(), 2);
}

#[test]
fn explicit_policy_reuses_unchanged_frame_but_coerces_replacement_and_restores_default() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    let first = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    assert_eq!(calls.get(), 1);
    owner.values().begin_styling();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 1);
    assert!(!owner.get_value(property));
    owner.values().begin_styling();
    first.dispose();
    let second = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 2);
    assert!(!owner.get_value(property));
    second.dispose();
    assert_eq!(calls.get(), 2);
    assert!(owner.get_value(property));
}

#[test]
fn explicit_policy_still_coerces_explicit_local_writes_and_requests() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    owner.set_value(property, true);
    owner.set_value(property, true);
    assert_eq!(calls.get(), 2);
    owner.coerce_value(property.as_property());
    assert_eq!(calls.get(), 3);
    assert!(!owner.get_value(property));
}

#[test]
fn default_policy_preserves_automatic_default_recoercion() {
    let (property, calls) = property(true);
    let owner = ScopeOwner::new();
    owner.coerce_value(property.as_property());
    owner.values().begin_styling();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 2);
    assert!(!owner.get_value(property));
}

#[test]
fn metadata_inherits_clones_and_explicitly_overrides_reevaluation_policy() {
    let (property, _) = property(false);
    property.override_metadata::<DerivedScopeOwner>(StyledPropertyMetadata::new(None));
    let derived = property.get_metadata(DerivedScopeOwner::TYPE);
    assert!(!derived.coerce_on_reevaluation());
    assert!(!derived.as_ref().clone().coerce_on_reevaluation());
    assert!(
        !StyledPropertyMetadata::generate_type_safe_metadata(&derived).coerce_on_reevaluation()
    );
    let default = StyledPropertyMetadata::<bool>::new(Some(true));
    assert!(default.coerce_on_reevaluation());
    let mut explicit = StyledPropertyMetadata::new(None).with_coerce_on_reevaluation(true);
    explicit.merge(&derived);
    assert!(explicit.coerce_on_reevaluation());
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Choice {
    First,
    Second,
}
crate::styled_enum!(Choice { First, Second });

#[test]
fn explicit_policy_and_local_input_survive_the_storage_of_an_enumeration() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let property: &'static StyledProperty<Choice> = StyledProperty::create(
        "ExplicitChoice",
        ScopeOwner::TYPE,
        ScopeOwner::TYPE,
        StyledPropertyMetadata::new(Some(Choice::First))
            .with_coerce(move |_, _| {
                count.set(count.get() + 1);
                Choice::Second
            })
            .with_coerce_on_reevaluation(false),
        false,
        None,
        None,
        false,
    );
    let owner = ScopeOwner::new();
    let style = owner
        .set_value_with_priority(property, Choice::First, BindingPriority::Style)
        .unwrap();
    assert_eq!(calls.get(), 1);
    owner.values().begin_styling();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 1, "an unchanged frame is not coerced again");
    assert_eq!(owner.get_value(property), Choice::Second);
    style.dispose();
    owner.set_value(property, Choice::First);
    assert_eq!(owner.get_local_value(property), Some(Choice::First));
    assert_eq!(owner.get_value(property), Choice::Second);
}

#[test]
fn explicit_policy_still_polls_changed_binding_entries() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    let source = Subject::behavior(true);
    let binding = owner.bind(property, source.observable(), BindingPriority::Style);
    assert_eq!(calls.get(), 1);
    source.on_next(false);
    assert_eq!(calls.get(), 2);
    source.on_next(true);
    assert_eq!(calls.get(), 3);
    owner.values().begin_styling();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 3);
    binding.dispose();
    assert!(owner.get_value(property));
    assert_eq!(calls.get(), 3);
}

#[test]
fn explicit_policy_reuses_animation_and_base_inputs_and_recoerces_a_new_winner() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    let style = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    let animation = owner
        .set_value_with_priority(property, true, BindingPriority::Animation)
        .unwrap();
    assert_eq!(calls.get(), 2);
    owner.values().begin_styling();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 2);
    animation.dispose();
    assert_eq!(calls.get(), 3);
    assert!(!owner.get_value(property));
    style.dispose();
    assert_eq!(calls.get(), 3);
    assert!(owner.get_value(property));
}

#[test]
fn explicit_policy_does_not_cache_a_failed_coercion() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let property = StyledProperty::create(
        "Retry",
        ScopeOwner::TYPE,
        ScopeOwner::TYPE,
        StyledPropertyMetadata::new(Some(true))
            .with_coerce(move |_, _| {
                count.set(count.get() + 1);
                assert_ne!(count.get(), 1, "first coercion fails");
                false
            })
            .with_coerce_on_reevaluation(false),
        false,
        None,
        None,
        false,
    );
    let owner = ScopeOwner::new();
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        owner.set_value_with_priority(property, true, BindingPriority::Style);
    }))
    .is_err());
    owner.values().begin_styling();
    owner.values().end_styling(&owner);
    assert_eq!(calls.get(), 2);
    assert!(!owner.get_value(property));
}

#[test]
fn explicit_policy_clears_coerced_default_without_polling_the_callback() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    owner.coerce_value(property.as_property());
    assert!(!owner.get_value(property));
    assert_eq!(calls.get(), 1);
    owner.clear_value(property);
    assert!(owner.get_value(property));
    assert_eq!(calls.get(), 1);
}

#[test]
fn resetting_coercion_preserves_local_ownership_and_does_not_poll_user_code() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    owner.set_value(property, true);
    assert!(!owner.get_value(property));
    owner.reset_coercion(property.as_property());
    assert!(owner.get_value(property));
    assert_eq!(owner.get_local_value(property), Some(true));
    assert_eq!(calls.get(), 1);
    owner.coerce_value(property.as_property());
    assert!(!owner.get_value(property));
    assert_eq!(calls.get(), 2);
}

#[test]
fn resetting_coercion_retains_a_live_binding_and_its_notifications() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    let source = Subject::behavior(true);
    let binding = owner.bind(property, source.observable(), BindingPriority::Style);
    owner.reset_coercion(property.as_property());
    assert!(owner.get_value(property));
    assert_eq!(calls.get(), 1);
    source.on_next(false);
    assert!(!owner.get_value(property));
    assert_eq!(calls.get(), 2);
    binding.dispose();
    assert!(owner.get_value(property));
}

#[test]
fn clearing_local_input_reveals_a_style_without_a_coercion_poll() {
    let (property, calls) = property(false);
    let owner = ScopeOwner::new();
    let style = owner
        .set_value_with_priority(property, true, BindingPriority::Style)
        .unwrap();
    owner.set_value(property, false);
    assert!(!owner.get_value(property));
    assert_eq!(calls.get(), 2);
    owner.clear_value(property);
    assert!(owner.get_value(property));
    assert_eq!(owner.get_local_value(property), None);
    assert_eq!(calls.get(), 2);
    style.dispose();
}

#[test]
fn ended_animation_restores_a_local_value_under_a_coerced_default() {
    let property: &'static StyledProperty<i32> = StyledProperty::create(
        "AtLeastTen",
        ScopeOwner::TYPE,
        ScopeOwner::TYPE,
        StyledPropertyMetadata::new(Some(0)).with_coerce(|_, value: i32| value.max(10)),
        false,
        None,
        None,
        false,
    );
    let owner = ScopeOwner::new();
    owner.set_value(property, 50);
    let animation = owner
        .set_value_with_priority(property, 70, BindingPriority::Animation)
        .unwrap();
    assert_eq!(owner.get_value(property), 70);
    animation.dispose();
    assert_eq!(owner.get_value(property), 50);
    assert_eq!(owner.get_local_value(property), Some(50));
}

#[test]
fn clear_inside_a_styling_update_removes_the_local_value() {
    let (property, _) = property(false);
    let owner = ScopeOwner::new();
    owner.set_value(property, true);
    owner.values().begin_styling();
    owner.clear_value(property);
    owner.values().end_styling(&owner);
    assert_eq!(owner.get_local_value(property), None);
    assert!(owner.get_value(property));
}

#[test]
fn clear_keeps_a_local_value_hidden_by_an_animation() {
    let (property, _) = property(false);
    let owner = ScopeOwner::new();
    owner.set_value(property, false);
    let animation = owner
        .set_value_with_priority(property, true, BindingPriority::Animation)
        .unwrap();
    owner.clear_value(property);
    assert_eq!(owner.get_local_value(property), Some(false));
    animation.dispose();
    assert_eq!(owner.get_local_value(property), Some(false));
    assert!(!owner.get_value(property));
}
