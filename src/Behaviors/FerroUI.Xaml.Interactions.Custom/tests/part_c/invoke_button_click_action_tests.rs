//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Custom/InvokeButtonClickActionTests.cs`
//! of Xaml.Behaviors.

use super::support::{as_bool, start_app, window_with, Command};
use crate::InvokeButtonClickAction;
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::BoxedValue;
use ferroui_controls::Button;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `new object()`: a value of its own identity.
#[derive(PartialEq)]
struct Parameter;

#[test]
fn execute_invokes_button_command_with_parameter() {
    let _app = start_app();
    let executed_parameter: Rc<RefCell<Option<BoxedValue>>> = Rc::new(RefCell::new(None));
    let click_count = Rc::new(Cell::new(0));
    let parameter: BoxedValue = Rc::new(Parameter);
    let button = Button::new();
    let recorded = executed_parameter.clone();
    button.set_command(Some(Command::new(move |value| *recorded.borrow_mut() = value.cloned())));
    button.set_command_parameter(Some(parameter.clone()));
    let clicks = click_count.clone();
    button.click(move |_sender: &Interactive, _e: &RoutedEventArgs| clicks.set(clicks.get() + 1));
    let window = window_with(&button);
    let action = InvokeButtonClickAction::new();
    action.set_target_button(&button);

    window.show();

    let result = action.execute(None, None);

    assert!(as_bool(result));
    assert_eq!(1, click_count.get());
    let executed_parameter = executed_parameter.borrow().clone().expect("the command ran with a parameter");
    assert!(Rc::ptr_eq(&parameter, &executed_parameter));
}

#[test]
fn execute_respects_handled_click_event() {
    let _app = start_app();
    let command_call_count = Rc::new(Cell::new(0));
    let button = Button::new();
    let calls = command_call_count.clone();
    button.set_command(Some(Command::new(move |_| calls.set(calls.get() + 1))));
    button.click(|_sender: &Interactive, args: &RoutedEventArgs| args.set_handled(true));
    let window = window_with(&button);
    let action = InvokeButtonClickAction::new();
    action.set_target_button(&button);

    window.show();

    let result = action.execute(None, None);

    assert!(as_bool(result));
    assert_eq!(0, command_call_count.get());
}

#[test]
fn execute_does_not_invoke_disabled_button() {
    let _app = start_app();
    let click_count = Rc::new(Cell::new(0));
    let command_call_count = Rc::new(Cell::new(0));
    let button = Button::new();
    button.set_is_enabled(false);
    let calls = command_call_count.clone();
    button.set_command(Some(Command::new(move |_| calls.set(calls.get() + 1))));
    let clicks = click_count.clone();
    button.click(move |_sender: &Interactive, _e: &RoutedEventArgs| clicks.set(clicks.get() + 1));
    let window = window_with(&button);
    let action = InvokeButtonClickAction::new();
    action.set_target_button(&button);

    window.show();

    let result = action.execute(None, None);

    assert!(!as_bool(result));
    assert_eq!(0, click_count.get());
    assert_eq!(0, command_call_count.get());
}
