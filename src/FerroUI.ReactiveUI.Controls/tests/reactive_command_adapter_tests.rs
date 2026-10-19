//! The tests of [`ReactiveCommandAdapter`] and of the conversion of the command parameters: the
//! port's own (upstream has one `ICommand`).

use super::binding_support::{start_app, text_parameter};
use crate::{boxed_parameter, parameter_as_any, ReactiveCommandAdapter, ReactiveCommandParameter};
use ferroui_base::input::ICommand as FerroCommand;
use ferroui_base::BoxedValue;
use ferroui_controls::Button;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::ICommand;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A command that records the parameters it is asked about and executed with, answers
/// `can_execute` with `enabled`, and raises `CanExecuteChanged` when the test asks.
struct ProbeCommand {
    enabled: Cell<bool>,
    asked: RefCell<Vec<Option<String>>>,
    executed: RefCell<Vec<Option<String>>>,
    changed: Rc<Event<()>>,
}

impl ProbeCommand {
    fn new(enabled: bool) -> Rc<Self> {
        Rc::new(Self { enabled: Cell::new(enabled), asked: RefCell::new(Vec::new()), executed: RefCell::new(Vec::new()), changed: Rc::new(Event::new()) })
    }

    fn text(parameter: Option<&dyn Any>) -> Option<String> {
        parameter.and_then(|value| value.downcast_ref::<String>()).cloned()
    }
}

impl ICommand for ProbeCommand {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        self.asked.borrow_mut().push(Self::text(parameter));
        self.enabled.get()
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        self.executed.borrow_mut().push(Self::text(parameter));
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(move |_: &()| handler()));
        let changed = self.changed.clone();
        Disposable::create(move || {
            changed.remove(token);
        })
    }
}

/// A value that is none of the values a control's parameter compares.
struct Payload;

#[test]
fn can_execute_asks_the_command_with_the_parameter() {
    let command = ProbeCommand::new(false);
    let adapter = ReactiveCommandAdapter::new(command.clone());
    let parameter = boxed_parameter(&text_parameter("first"));

    assert!(!FerroCommand::can_execute(&*adapter, parameter.as_ref()));
    assert!(!FerroCommand::can_execute(&*adapter, None));

    assert_eq!(*command.asked.borrow(), vec![Some("first".to_string()), None]);
}

#[test]
fn execute_runs_the_command_with_the_parameter() {
    let command = ProbeCommand::new(true);
    let adapter = ReactiveCommandAdapter::new(command.clone());
    let parameter = boxed_parameter(&text_parameter("first"));

    FerroCommand::execute(&*adapter, parameter.as_ref());

    assert_eq!(*command.executed.borrow(), vec![Some("first".to_string())]);
}

#[test]
fn can_execute_changed_adds_the_handler_to_the_command() {
    let command = ProbeCommand::new(true);
    let adapter = ReactiveCommandAdapter::new(command.clone());
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();

    let subscription = FerroCommand::can_execute_changed(&*adapter, Rc::new(move || counted.set(counted.get() + 1)));
    command.changed.raise(&());
    subscription.dispose();
    command.changed.raise(&());

    assert_eq!(calls.get(), 1);
}

#[test]
fn the_adapter_wraps_its_command() {
    let command: Rc<dyn ICommand> = ProbeCommand::new(true);
    let other: Rc<dyn ICommand> = ProbeCommand::new(true);

    let adapter = ReactiveCommandAdapter::new(command.clone());

    assert!(adapter.wraps(&command));
    assert!(!adapter.wraps(&other));
    assert!(std::ptr::addr_eq(Rc::as_ptr(adapter.command()), Rc::as_ptr(&command)));
}

#[test]
fn a_text_parameter_is_boxed_as_its_value() {
    let boxed = boxed_parameter(&text_parameter("first")).expect("a parameter");

    assert_eq!(boxed.downcast_ref::<String>(), Some(&"first".to_string()));
    assert_eq!(parameter_as_any(&boxed).downcast_ref::<String>(), Some(&"first".to_string()));
    assert!(*boxed == *(Rc::new("first".to_string()) as BoxedValue));
}

#[test]
fn a_number_parameter_is_boxed_as_its_value() {
    let boxed = boxed_parameter(&Some(Rc::new(5_i32) as Rc<dyn Any>)).expect("a parameter");

    assert_eq!(boxed.downcast_ref::<i32>(), Some(&5));
}

#[test]
fn another_parameter_is_boxed_by_identity() {
    let payload: Rc<dyn Any> = Rc::new(Payload);

    let first = boxed_parameter(&Some(payload.clone())).expect("a parameter");
    let second = boxed_parameter(&Some(payload.clone())).expect("a parameter");
    let other = boxed_parameter(&Some(Rc::new(Payload) as Rc<dyn Any>)).expect("a parameter");

    assert!(first.downcast_ref::<ReactiveCommandParameter>().is_some());
    assert!(*first == *second);
    assert!(*first != *other);
    assert!(std::ptr::addr_eq(parameter_as_any(&first) as *const dyn Any, Rc::as_ptr(&payload)));
}

#[test]
fn a_boxed_parameter_is_kept() {
    let value: BoxedValue = Rc::new(3.5_f64);

    let boxed = boxed_parameter(&Some(Rc::new(value.clone()) as Rc<dyn Any>)).expect("a parameter");

    assert!(Rc::ptr_eq(&boxed, &value));
}

#[test]
fn no_parameter_is_none() {
    assert!(boxed_parameter(&None).is_none());
}

#[test]
fn the_adapter_is_the_command_of_a_button() {
    let _app = start_app();
    let command = ProbeCommand::new(true);
    let button = Button::new();

    button.set_command(Some(ReactiveCommandAdapter::command_of(command.clone())));
    button.set_command_parameter(boxed_parameter(&text_parameter("first")));
    let set = button.command().expect("the command of the button");
    FerroCommand::execute(&*set, button.command_parameter().as_ref());

    assert_eq!(*command.executed.borrow(), vec![Some("first".to_string())]);
}
