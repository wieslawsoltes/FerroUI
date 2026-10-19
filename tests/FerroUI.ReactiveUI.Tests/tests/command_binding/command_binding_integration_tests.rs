//! `CommandBindingIntegrationTests` (ReactiveUI.Tests, `CommandBinding/CommandBindingIntegrationTests.cs`).
//!
//! Upstream's generated code binds the literal event `"Click"` at compile time; here the event is a
//! `ToEvent::event` that reads it from the control. The `[Arguments]` rows are one test each.

use crate::mocks::immediate;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::events::Event;
use ferroui_system::{prop, IDisposable, IObservable};
use std::cell::Cell;
use std::rc::Rc;

/// The event of the control under test (`ClickEvent`).
const CLICK_EVENT: &str = "Click";

/// The sources of the command parameter (`ParameterSource`).
#[derive(Clone, Copy, Debug)]
enum ParameterSource {
    /// No command parameter.
    None,
    /// An observable command parameter.
    Observable,
    /// A view model property as the command parameter.
    Expression,
}

/// A control with a bindable event (`FakeControl`).
#[derive(Default)]
struct FakeControl {
    click: Event<()>,
}

impl FakeControl {
    /// Raises `Click` (`RaiseClick`).
    fn raise_click(&self) {
        self.click.raise(&());
    }
}

/// Controls compare by identity, as .NET compares references.
impl PartialEq for FakeControl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A notifying view model that owns the command (`FakeViewModel`).
struct FakeViewModel {
    reactive: ReactiveObject,
    command: ReactiveCommand<(), ()>,
}
reactive_object!(FakeViewModel);

impl FakeViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::default(), command: ReactiveCommand::create_with(|| {}, None, Some(immediate())) })
    }

    /// The command under test (`Command`).
    fn command(&self) -> ReactiveCommand<(), ()> {
        self.command.clone()
    }

    /// The parameter read from a property (`Parameter`).
    fn parameter(&self) {}
}

/// View models compare by identity, as .NET compares references.
impl PartialEq for FakeViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

reactive_properties! {
    /// A notifying view that owns the control (`FakeView`).
    #[derive(Default)]
    struct FakeView {
        reactive: ReactiveObject,
        control: Rc<FakeControl>,
        #[reactive]
        view_model: Option<Rc<FakeViewModel>>,
    }
}
reactive_object!(FakeView);
view_for!(FakeView: INotifyPropertyChanged, FakeViewModel);

impl FakeView {
    /// The bound control (`Control`).
    fn control(&self) -> Rc<FakeControl> {
        self.control.clone()
    }
}

/// The event `Click` of the control.
fn click() -> Option<ToEvent<FakeControl>> {
    Some(ToEvent::event(CLICK_EVENT, |control: &FakeControl| &control.click))
}

/// `BindCommand_ParameterSource_Executes`, for one row of its arguments.
fn bind_command_parameter_source_executes(parameter_source: ParameterSource) {
    let view_model = FakeViewModel::new();
    let view = Rc::new(FakeView::default());
    view.set_view_model(Some(view_model.clone()));
    let invocation_count = Rc::new(Cell::new(0));
    let counted = invocation_count.clone();
    let commands: Rc<dyn IObservable<()>> = Rc::new(view_model.command());
    let execution = commands.subscribe_next(move |_| counted.set(counted.get() + 1));

    let binding: Rc<dyn IDisposable> = match parameter_source {
        ParameterSource::None => view.bind_command(prop!(FakeViewModel => command), prop!(FakeView => control), click()),
        ParameterSource::Observable => {
            view.bind_command_with_parameter(prop!(FakeViewModel => command), prop!(FakeView => control), Observable::return_(()), click())
        }
        ParameterSource::Expression => view.bind_command_with_parameter_path(
            prop!(FakeViewModel => command),
            prop!(FakeView => control),
            prop!(FakeViewModel => parameter),
            click(),
        ),
    };

    view.control().raise_click();

    assert_eq!(invocation_count.get(), 1);
    binding.dispose();
    execution.dispose();
    view_model.command().dispose();
}

#[test]
fn bind_command_parameter_source_executes_none() {
    bind_command_parameter_source_executes(ParameterSource::None);
}

#[test]
fn bind_command_parameter_source_executes_observable() {
    bind_command_parameter_source_executes(ParameterSource::Observable);
}

#[test]
fn bind_command_parameter_source_executes_expression() {
    bind_command_parameter_source_executes(ParameterSource::Expression);
}
