//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/InvokeCommandActionTests.cs` of
//! Xaml.Behaviors (with its windows `InvokeCommandAction001.xaml` to `InvokeCommandAction004.xaml`,
//! loaded with the run-time loader).
//!
//! Adapted: the code of the windows of the reference (a `TestCommand` property and the window as
//! its own data context) is a model set as the data context of the window by the test; the input
//! converter of `InvokeCommandAction003` (`{x:Static local:TestValueConverter.Instance}`) is set
//! by the test; the name of the type of the event args is the name of the routed event args the
//! command receives. The rendered frames the reference saves are not taken.

use super::command::Command;
use super::test_value_converter::TestValueConverter;
use crate::tests::support::{click, find, load, run_jobs, start_app};
use crate::{EventTriggerBehavior, InvokeCommandAction};
use ferroui_base::data::core::Maybe;
use ferroui_base::data::model::Model as FerroModel;
use ferroui_base::input::ICommand;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_model, BoxedValue, Ref};
use ferroui_controls::{Button, TextBox, Window};
use ferroui_xaml_interactivity::Interaction;
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The code of the windows of the tests: the `TestCommand` they bind to.
pub(crate) struct CommandHost {
    test_command: RefCell<Option<Rc<dyn ICommand>>>,
}

ferro_model!(CommandHost, |b| b
    .read_only::<Maybe<Rc<dyn ICommand>>>("TestCommand", |host| host.test_command.borrow().clone()));

/// Loads a window of the tests and sets its data context to a host of the command that `execute`
/// makes from the text box of the window.
fn load_window(
    name: &str,
    xaml: &str,
    execute: impl Fn(&Ref<TextBox>, Option<&BoxedValue>) + 'static,
) -> (Ref<Window>, Rc<CommandHost>) {
    let window = load::<Window>(name, xaml);
    let target_text_box = find::<TextBox>(&window, "TargetTextBox");
    let host = <CommandHost as FerroModel>::new_model(CommandHost { test_command: RefCell::new(None) });
    *host.test_command.borrow_mut() = Some(Command::new(move |parameter| execute(&target_text_box, parameter)));
    // A model is boxed as the shared object itself.
    window.set_data_context(Some(host.clone() as BoxedValue));
    (window, host)
}

/// The text of a parameter of the commands (`$"{parameter}"`).
fn parameter_text(parameter: Option<&BoxedValue>) -> String {
    parameter.and_then(|parameter| parameter.downcast_ref::<String>().cloned()).unwrap_or_default()
}

/// The name of the type of a parameter (`parameter?.GetType().Name`): the routed event args are
/// named by their type.
fn parameter_type_name(parameter: Option<&BoxedValue>) -> String {
    let Some(parameter) = parameter else {
        return String::new();
    };

    if let Some(args) = parameter.downcast_ref::<Rc<dyn IRoutedEventArgs>>() {
        let args: &dyn Any = &**args;
        if args.is::<RoutedEventArgs>() {
            return "RoutedEventArgs".to_string();
        }
    }

    parameter.type_name().rsplit("::").next().unwrap_or_default().to_string()
}

fn target_text(window: &Window) -> Option<String> {
    find::<TextBox>(window, "TargetTextBox").text()
}

#[test]
fn invoke_command_action_001() {
    let _app = start_app();
    let (window, _host) =
        load_window("InvokeCommandAction001.xaml", include_str!("InvokeCommandAction001.xaml"), |text_box, _| {
            text_box.set_text(Some("Command Text"))
        });

    window.show();
    run_jobs();

    assert_eq!(Some("Initial Text".to_string()), target_text(&window));

    // Click
    click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("Command Text".to_string()), target_text(&window));
}

#[test]
fn invoke_command_action_002() {
    let _app = start_app();
    let (window, _host) =
        load_window("InvokeCommandAction002.xaml", include_str!("InvokeCommandAction002.xaml"), |text_box, parameter| {
            text_box.set_text(Some(&format!("Command {}", parameter_text(parameter))))
        });

    window.show();
    run_jobs();

    assert_eq!(Some("Initial Text".to_string()), target_text(&window));

    // Click
    click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("Command Param".to_string()), target_text(&window));
}

#[test]
fn invoke_command_action_003() {
    let _app = start_app();
    let (window, _host) =
        load_window("InvokeCommandAction003.xaml", include_str!("InvokeCommandAction003.xaml"), |text_box, parameter| {
            text_box.set_text(Some(&format!("Command {}", parameter_text(parameter))))
        });
    let target_button = find::<Button>(&window, "TargetButton");
    let trigger = Interaction::get_behaviors(&target_button).get(0).cast::<EventTriggerBehavior>().expect("the trigger");
    let action = trigger.actions().expect("the actions").get(0).cast::<InvokeCommandAction>().expect("the action");
    action.set_input_converter(Some(TestValueConverter::instance()));

    window.show();
    run_jobs();

    assert_eq!(Some("Initial Text".to_string()), target_text(&window));

    // Click
    click(&target_button);

    assert_eq!(Some("Command Button".to_string()), target_text(&window));
}

#[test]
fn invoke_command_action_004() {
    let _app = start_app();
    let (window, _host) =
        load_window("InvokeCommandAction004.xaml", include_str!("InvokeCommandAction004.xaml"), |text_box, parameter| {
            text_box.set_text(Some(&parameter_type_name(parameter)))
        });

    window.show();
    run_jobs();

    assert_eq!(Some("Initial Text".to_string()), target_text(&window));

    // Click
    click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("RoutedEventArgs".to_string()), target_text(&window));
}
