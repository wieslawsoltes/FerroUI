//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/InvokeCommandBehaviorBaseTests.cs` of
//! Xaml.Behaviors.

use super::command_can_execute_observer_tests::{is_text, ObservableCommand};
use super::support::{start_app, text};
use crate::{InvokeCommandBehaviorBase, StyledElementBehaviorImpl};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElement;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_controls::Button;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// `TestInvokeCommandBehavior`.
#[repr(C)]
struct TestInvokeCommandBehavior {
    base: InvokeCommandBehaviorBase,
}

ferro_class!(TestInvokeCommandBehavior: InvokeCommandBehaviorBase);
ferroui_base::ferro_impl_classes!(TestInvokeCommandBehavior: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

impl TestInvokeCommandBehavior {
    fn new() -> Ref<Self> {
        instantiate(Self { base: InvokeCommandBehaviorBase::construct() })
    }

    fn resolve_parameter_for_test(&self, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        self.resolve_parameter(parameter)
    }
}

/// `BooleanObservable`: an observable of one boolean value with a single observer.
struct BooleanObservable {
    this: Weak<BooleanObservable>,
    observer: RefCell<Option<Rc<dyn IObserver<bool>>>>,
    value: Cell<bool>,
}

impl BooleanObservable {
    fn new(value: bool) -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), observer: RefCell::new(None), value: Cell::new(value) })
    }

    fn set_value(&self, value: bool) {
        if self.value.get() == value {
            return;
        }

        self.value.set(value);
        let observer = self.observer.borrow().clone();
        if let Some(observer) = observer {
            observer.on_next(value);
        }
    }

    fn unsubscribe(&self, observer: &Rc<dyn IObserver<bool>>) {
        let is_current = self.observer.borrow().as_ref().is_some_and(|current| Rc::ptr_eq(current, observer));
        if is_current {
            *self.observer.borrow_mut() = None;
        }
    }
}

impl IObservable<bool> for BooleanObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<bool>>) -> Rc<dyn IDisposable> {
        *self.observer.borrow_mut() = Some(observer.clone());
        observer.on_next(self.value.get());

        let owner = self.this.clone();
        let disposed = Cell::new(false);
        Disposable::create(move || {
            if disposed.replace(true) {
                return;
            }
            if let Some(owner) = owner.upgrade() {
                owner.unsubscribe(&observer);
            }
        })
    }
}

#[test]
fn can_execute_command_defaults_to_true_without_command() {
    let _app = start_app();
    let behavior = TestInvokeCommandBehavior::new();
    let button = Button::new();

    behavior.attach(&button);

    assert!(behavior.can_execute_command());

    behavior.detach();
}

#[test]
fn can_execute_command_updates_when_command_raises_can_execute_changed() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(behavior.can_execute_command());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_default_false_does_not_update_associated_object_is_enabled() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    let button = Button::new();

    behavior.attach(&button);

    assert!(!behavior.can_execute_command());
    assert!(button.is_enabled());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_updates_associated_object_is_enabled() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let button = Button::new();

    behavior.attach(&button);

    assert!(!button.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(button.is_enabled());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_preserves_existing_disabled_state() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let button = Button::new();
    button.set_is_enabled(false);

    behavior.attach(&button);

    assert!(!button.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(!button.is_enabled());

    button.set_is_enabled(true);

    assert!(button.is_enabled());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_composes_with_existing_is_enabled_binding() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let is_enabled = BooleanObservable::new(true);
    let button = Button::new();
    let _binding = button.bind(
        InputElement::is_enabled_property(),
        is_enabled.clone() as Rc<dyn IObservable<bool>>,
        BindingPriority::LocalValue,
    );

    behavior.attach(&button);

    assert!(!button.is_enabled());

    is_enabled.set_value(false);

    assert!(!button.is_enabled());

    is_enabled.set_value(true);

    assert!(!button.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(button.is_enabled());

    is_enabled.set_value(false);

    assert!(!button.is_enabled());

    command.can_execute_result.set(false);
    command.raise_can_execute_changed();

    assert!(!button.is_enabled());

    command.can_execute_result.set(true);
    command.raise_can_execute_changed();

    assert!(!button.is_enabled());

    behavior.detach();
}

#[test]
fn use_command_can_execute_for_is_enabled_property_change_attaches_and_detaches_binding() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    let button = Button::new();

    behavior.attach(&button);

    assert!(button.is_enabled());

    behavior.set_use_command_can_execute_for_is_enabled(true);

    assert!(!button.is_enabled());

    behavior.set_use_command_can_execute_for_is_enabled(false);

    assert!(button.is_enabled());

    behavior.detach();
}

#[test]
fn can_execute_command_uses_current_command_parameter() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_command_parameter(Some(text("disabled")));
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());

    behavior.set_command_parameter(Some(text("enabled")));

    assert!(behavior.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));

    behavior.detach();
}

#[test]
fn can_execute_command_uses_null_when_command_parameter_is_unset() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| parameter.is_none());
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    let button = Button::new();

    behavior.attach(&button);

    assert!(behavior.can_execute_command());
    assert!(command.last_can_execute_parameter().is_none());

    behavior.detach();
}

#[test]
fn can_execute_command_defers_when_event_parameter_is_unavailable() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|_| panic!("The event parameter is not available yet."));
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_pass_event_args_to_command(true);
    behavior.set_use_command_can_execute_for_is_enabled(true);
    let button = Button::new();

    behavior.attach(&button);

    assert!(behavior.can_execute_command());
    assert!(button.is_enabled());
    assert_eq!(0, command.can_execute_call_count());

    command.raise_can_execute_changed();

    assert!(behavior.can_execute_command());
    assert!(button.is_enabled());
    assert_eq!(0, command.can_execute_call_count());

    behavior.detach();
}

#[test]
fn can_execute_command_uses_separate_can_execute_command_parameter() {
    let _app = start_app();
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    behavior.set_pass_event_args_to_command(true);
    behavior.set_can_execute_command_parameter(Some(text("disabled")));
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "disabled"));

    behavior.set_can_execute_command_parameter(Some(text("enabled")));

    assert!(behavior.can_execute_command());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));
    assert!(is_text(behavior.resolve_parameter_for_test(Some(&text("event parameter"))).as_ref(), "event parameter"));

    behavior.detach();
}

#[test]
fn pass_event_args_to_command_updates_can_execute_parameter_availability() {
    let _app = start_app();
    let command = ObservableCommand::with_result(false);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(command.as_command());
    let button = Button::new();

    behavior.attach(&button);
    assert!(!behavior.can_execute_command());
    assert_eq!(1, command.can_execute_call_count());

    behavior.set_pass_event_args_to_command(true);

    assert!(behavior.can_execute_command());
    assert_eq!(1, command.can_execute_call_count());

    behavior.detach();
}

#[test]
fn can_execute_command_rewires_subscriptions_when_command_changes_and_detaches() {
    let _app = start_app();
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let behavior = TestInvokeCommandBehavior::new();
    behavior.set_command(first_command.as_command());
    let button = Button::new();

    behavior.attach(&button);

    assert_eq!(1, first_command.subscription_count());
    assert!(!behavior.can_execute_command());

    behavior.set_command(second_command.as_command());

    assert_eq!(0, first_command.subscription_count());
    assert_eq!(1, second_command.subscription_count());
    assert!(behavior.can_execute_command());

    behavior.detach();

    assert_eq!(0, second_command.subscription_count());
}
