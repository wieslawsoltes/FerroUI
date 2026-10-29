//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/ActionTests.cs` of Xaml.Behaviors.

use super::support::{boxed, object, panics, start_app, text};
use crate::{Action, ActionCollection, ActionImpl, Interaction};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref};
use ferroui_controls::Button;
use std::cell::{Cell, RefCell};

/// An action that records its execution and returns a fixed value.
#[repr(C)]
struct TestAction {
    base: Action,
    return_value: Option<BoxedValue>,
    sender: RefCell<Option<BoxedValue>>,
    parameter: RefCell<Option<BoxedValue>>,
    execute_count: Cell<i32>,
}

ferro_class!(TestAction: Action);
ferroui_base::ferro_impl_classes!(TestAction: FerroObjectImpl);

impl ActionImpl for TestAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        this.execute_count.set(this.execute_count.get() + 1);
        *this.sender.borrow_mut() = sender.cloned();
        *this.parameter.borrow_mut() = parameter.cloned();
        this.return_value.clone()
    }
}

impl TestAction {
    fn new() -> Ref<Self> {
        Self::with_return_value(None)
    }

    fn with_return_value(return_value: Option<BoxedValue>) -> Ref<Self> {
        instantiate(Self {
            base: Action::construct(),
            return_value,
            sender: RefCell::new(None),
            parameter: RefCell::new(None),
            execute_count: Cell::new(0),
        })
    }
}

#[test]
fn is_enabled_default_true() {
    let _app = start_app();
    let action = TestAction::new();
    assert!(action.is_enabled());
}

#[test]
fn is_enabled_set_false() {
    let _app = start_app();
    let action = TestAction::new();
    action.set_is_enabled(false);
    assert!(!action.is_enabled());
}

#[test]
fn execute_sets_properties_and_returns_value() {
    let _app = start_app();
    let action = TestAction::with_return_value(Some(text("result")));
    let sender = boxed(&FerroObject::new());
    let parameter = boxed(&FerroObject::new());

    let result = action.execute(Some(&sender), Some(&parameter));

    assert_eq!(1, action.execute_count.get());
    assert_eq!(Some(sender), action.sender.borrow().clone());
    assert_eq!(Some(parameter), action.parameter.borrow().clone());
    assert_eq!(Some(text("result")), result);
}

#[test]
fn action_collection_add_non_action_throws() {
    let _app = start_app();
    let collection = ActionCollection::new();
    collection.add(object(&TestAction::new()));

    assert!(panics(|| collection.add(object(&Button::new()))));
}

#[test]
fn action_collection_replace_with_non_action_throws() {
    let _app = start_app();
    let collection = ActionCollection::new();
    collection.add(object(&TestAction::new()));

    assert!(panics(|| collection.set(0, object(&Button::new()))));
}

#[test]
fn execute_actions_with_plain_actions_all_executed() {
    let _app = start_app();
    let actions = ActionCollection::new();
    let items = [TestAction::new(), TestAction::new(), TestAction::new()];
    for item in &items {
        actions.add(object(item));
    }

    let sender = boxed(&FerroObject::new());
    let parameter = text("param");

    Interaction::execute_actions(Some(&sender), Some(&actions), Some(&parameter));

    for action in &items {
        assert_eq!(1, action.execute_count.get());
        assert_eq!(Some(sender.clone()), action.sender.borrow().clone());
        assert_eq!(Some(parameter.clone()), action.parameter.borrow().clone());
    }
}

#[test]
fn execute_actions_return_values_in_order() {
    let _app = start_app();
    let expected = ["A", "B", "C"];
    let actions = ActionCollection::new();

    for value in expected {
        actions.add(object(&TestAction::with_return_value(Some(text(value)))));
    }

    let results = Interaction::execute_actions(None, Some(&actions), None);

    assert_eq!(expected.len(), results.len());
    for (i, result) in results.iter().enumerate() {
        assert_eq!(&text(expected[i]), result);
    }
}
