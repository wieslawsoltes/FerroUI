//! `InvokeCommandTests` (ReactiveUI.Binding.Tests, `Mixins/InvokeCommandTests.cs`).
//!
//! The overload naming a property is `invoke_command_path` with a property path, which observes
//! the property through the observation of ReactiveUI.Binding (`InvokeCommandUnsafe` upstream,
//! which reads the expression by reflection).

use crate::mocks::*;
use ferroui_reactive::subjects::{Subject, SubjectBase};
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObserver};
use std::rc::Rc;

const FIRST_VALUE: &str = "first";
const SECOND_VALUE: &str = "second";

#[test]
fn invoke_command_given_the_command_executes_with_each_value() {
    let command = RecordingCommand::new();
    let values = Subject::<String>::new();

    let invocation = values.as_observable().invoke_command(command.clone());

    values.on_next(FIRST_VALUE.to_owned());
    values.on_next(SECOND_VALUE.to_owned());

    assert_eq!(command.execute_count(), 2);
    assert_eq!(command.last_parameter().as_deref(), Some(SECOND_VALUE));
    invocation.dispose();
}

// Not ported: InvokeCommand_GivenNoCommand_Throws - argument validation

#[test]
fn invoke_command_naming_a_property_executes_through_the_runtime_engine() {
    let command = RecordingCommand::new();
    let view_model = DispatchStubViewModel::new();
    view_model.set_run(Some(command.clone()));
    let values = Subject::<String>::new();

    let invocation = values.as_observable().invoke_command_path(Some(view_model.clone()), prop!(DispatchStubViewModel => run));

    values.on_next(FIRST_VALUE.to_owned());

    assert_eq!(command.execute_count(), 1);
    assert_eq!(command.last_parameter().as_deref(), Some(FIRST_VALUE));
    invocation.dispose();
}

#[test]
fn invoke_command_naming_a_property_on_nothing_drops_the_values() {
    let values = Subject::<String>::new();

    let invocation = values
        .as_observable()
        .invoke_command_path::<DispatchStubViewModel, Rc<dyn ICommand>>(None, prop!(DispatchStubViewModel => run));

    values.on_next(FIRST_VALUE.to_owned());

    assert!(!values.has_observers());
    invocation.dispose();
}

// Not ported: InvokeCommand_NullSelector_Throws - argument validation
// Not ported: InvokeCommand_NullSource_Throws - argument validation
