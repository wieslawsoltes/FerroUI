//! Not from the reference: tests of `InteractionTriggerBehavior.cs`.

use super::support::{boxed, object, start_app, window_with, RecordingAction};
use crate::{AnyInteraction, InteractionTriggerBehavior};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{BoxedValue, FerroObject, Ref};
use ferroui_controls::Button;
use ferroui_reactiveui::Interaction as ReactiveInteraction;
use ferroui_system::threading::tasks::block_on;
use ferroui_xaml_interactivity::Interaction;
use std::rc::Rc;

/// A trigger of `interaction` whose action is `action`.
fn trigger_with(interaction: Option<AnyInteraction>, action: &Ref<RecordingAction>) -> Ref<InteractionTriggerBehavior> {
    let trigger = InteractionTriggerBehavior::new();
    trigger.set_interaction(interaction);
    trigger.actions().expect("a trigger starts with a collection of actions").add(object(action));
    trigger
}

/// Whether running `f` panics (an exception of the reference).
fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}

#[test]
fn handle_executes_the_actions_with_the_input_as_the_parameter() {
    let _app = start_app();
    let interaction = ReactiveInteraction::<String, ()>::new();
    let action = RecordingAction::new();
    let trigger = trigger_with(Some(AnyInteraction::new(interaction.clone())), &action);
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    Interaction::get_behaviors(&button).add(object(&trigger));
    window.show();

    block_on(interaction.handle("question".to_owned())).expect("the trigger handles the question");

    assert_eq!(1, action.execute_count());
    assert_eq!(Some(boxed(&button)), action.sender());
    assert_eq!(Some(Rc::new("question".to_owned()) as BoxedValue), action.parameter());

    Interaction::set_behaviors(&button, None);
    window.close();
}

#[test]
fn handle_answers_with_the_default_output() {
    let _app = start_app();
    let interaction = ReactiveInteraction::<i32, i32>::new();
    let action = RecordingAction::new();
    let trigger = trigger_with(Some(AnyInteraction::new(interaction.clone())), &action);
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    Interaction::get_behaviors(&button).add(object(&trigger));
    window.show();

    let output = block_on(interaction.handle(42)).expect("the trigger handles the question");

    assert_eq!(0, output);
    assert_eq!(Some(Rc::new(42) as BoxedValue), action.parameter());

    Interaction::set_behaviors(&button, None);
    window.close();
}

#[test]
fn the_handler_is_registered_when_the_associated_object_is_attached_to_the_visual_tree() {
    let _app = start_app();
    let interaction = ReactiveInteraction::<(), ()>::new();
    let action = RecordingAction::new();
    let trigger = trigger_with(Some(AnyInteraction::new(interaction.clone())), &action);
    let button = Button::new();
    Interaction::get_behaviors(&button).add(object(&trigger));

    assert!(interaction.get_handlers().is_empty());

    let window = window_with(Some(button.clone().upcast()));
    window.show();

    assert_eq!(1, interaction.get_handlers().len());

    Interaction::set_behaviors(&button, None);
    window.close();
}

#[test]
fn detaching_from_the_visual_tree_unregisters_the_handler() {
    let _app = start_app();
    let interaction = ReactiveInteraction::<(), ()>::new();
    let action = RecordingAction::new();
    let trigger = trigger_with(Some(AnyInteraction::new(interaction.clone())), &action);
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    Interaction::get_behaviors(&button).add(object(&trigger));
    window.show();

    Interaction::set_behaviors(&button, None);

    assert!(interaction.get_handlers().is_empty());
    assert!(block_on(interaction.handle(())).is_err());
    assert_eq!(0, action.execute_count());

    window.close();
}

#[test]
fn without_an_interaction_the_trigger_registers_nothing() {
    let _app = start_app();
    let action = RecordingAction::new();
    let trigger = trigger_with(None, &action);
    let button = Button::new();
    let window = window_with(Some(button.clone().upcast()));
    Interaction::get_behaviors(&button).add(object(&trigger));
    window.show();

    assert!(trigger.interaction().is_none());
    assert_eq!(0, action.execute_count());

    Interaction::set_behaviors(&button, None);
    window.close();
}

#[test]
fn attach_to_an_object_that_is_not_a_visual_throws() {
    let _app = start_app();
    let action = RecordingAction::new();
    let trigger = trigger_with(None, &action);
    let not_a_visual = FerroObject::new();

    assert!(panics(|| trigger.attach(&not_a_visual)));
}

#[test]
fn interaction_of_gives_the_typed_interaction() {
    let _app = start_app();
    let interaction = ReactiveInteraction::<String, bool>::new();
    let trigger = InteractionTriggerBehavior::new();

    trigger.set_interaction_of(Some(interaction.clone()));

    assert_eq!(Some(interaction.clone()), trigger.interaction_of::<String, bool>());
    assert!(trigger.interaction_of::<String, i32>().is_none());
    assert_eq!(Some(AnyInteraction::new(interaction)), trigger.interaction());
}

#[test]
fn any_interactions_are_equal_when_they_wrap_the_same_interaction() {
    let first = ReactiveInteraction::<(), ()>::new();
    let second = ReactiveInteraction::<(), ()>::new();

    assert_eq!(AnyInteraction::new(first.clone()), AnyInteraction::new(first.clone()));
    assert_ne!(AnyInteraction::new(first), AnyInteraction::new(second));
}

#[test]
fn an_interaction_of_unit_converts_to_the_type_of_the_property() {
    let _app = start_app();
    let interaction = ReactiveInteraction::<(), ()>::new();
    let value: BoxedValue = Rc::new(interaction.clone());

    let converted = ValueTypes::try_cast(&value, ValueType::of::<Option<AnyInteraction>>())
        .expect("the crate registers the cast of an interaction of unit");

    assert_eq!(Some(&Some(AnyInteraction::new(interaction))), converted.downcast_ref::<Option<AnyInteraction>>());
}
