//! The helpers of the ported tests of the binding integration, the activation, the scheduler and
//! the adapter: `TestObject.cs`, `ValueSource.cs`, `EventSource.cs`, `ListObserver.cs` and
//! `RecordingCommand.cs` of the tests of ReactiveUI.Binding's integration, and the application of a
//! test.
//!
//! The reference's tests run their bodies on the UI thread of one headless session (the test
//! session of those tests); here each test starts the unit test application of the framework on
//! its own thread, whose dispatcher runs the posted work when the test runs its jobs.

#![allow(dead_code)]

use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledProperty,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::{CommandEventSubscription, CommandParameter, ICommand};
use ferroui_system::events::Event;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// ---------------------------------------------------------------------------------------------
// Application and dispatcher
// ---------------------------------------------------------------------------------------------

/// Starts the unit test application of a test, with the services of a styled window (windows, the
/// test theme, text).
pub fn start_app() -> UnitTestApplicationScope {
    UnitTestApplication::start(TestServices::styled_window())
}

/// `Dispatcher.UIThread.RunJobs()`.
pub fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// A command parameter of the text `value`.
pub fn text_parameter(value: &str) -> CommandParameter {
    Some(Rc::new(value.to_string()) as Rc<dyn Any>)
}

/// The text of a command parameter, if it is one.
pub fn parameter_text(parameter: &CommandParameter) -> Option<String> {
    parameter.as_ref().and_then(|value| value.downcast_ref::<String>().cloned())
}

/// An error of a parameter stream (`new FormatException()`).
#[derive(Debug)]
pub struct FormatError;

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FormatException: One of the identified items was in an invalid format.")
    }
}

impl std::error::Error for FormatError {}

// ---------------------------------------------------------------------------------------------
// `TestObject.cs`
// ---------------------------------------------------------------------------------------------

/// A FerroUI object with two registered properties and one plain property (`TestObject`).
#[repr(C)]
pub struct TestObject {
    base: FerroObject,
    plain: RefCell<Option<String>>,
}

ferro_class!(TestObject: FerroObject);
ferro_impl_classes!(TestObject: FerroObjectImpl);

ferroui_base::ferro_properties! { impl TestObject {
    ferro_property!(
        /// The registered property behind `text` (`TextProperty`).
        pub fn text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<TestObject, _>("Text", None)
        }
    );

    ferro_property!(
        /// The registered property behind `count` (`CountProperty`).
        pub fn count_property() -> StyledProperty<i32> {
            FerroProperty::register::<TestObject, _>("Count", 0)
        }
    );
} }

impl TestObject {
    /// `new TestObject()`.
    pub fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct(), plain: RefCell::new(None) })
    }

    /// A registered string property (`Text`).
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value);
    }

    /// A registered integer property (`Count`).
    pub fn count(&self) -> i32 {
        self.get_value(Self::count_property())
    }

    pub fn set_count(&self, value: i32) {
        self.set_value(Self::count_property(), value);
    }

    /// A plain property that registers no property of the property system (`Plain`).
    pub fn plain(&self) -> Option<String> {
        self.plain.borrow().clone()
    }

    pub fn set_plain(&self, value: Option<String>) {
        *self.plain.borrow_mut() = value;
    }
}

// ---------------------------------------------------------------------------------------------
// `ValueSource.cs`
// ---------------------------------------------------------------------------------------------

/// An observable the test pushes values and errors into, with one subscriber at a time
/// (`ValueSource`).
#[derive(Default)]
pub struct ValueSource {
    observer: Rc<RefCell<Option<Rc<dyn IObserver<CommandParameter>>>>>,
}

impl ValueSource {
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Whether a subscriber is attached (`HasObserver`).
    pub fn has_observer(&self) -> bool {
        self.observer.borrow().is_some()
    }

    /// Pushes a value to the subscriber (`Push`).
    pub fn push(&self, value: CommandParameter) {
        let observer = self.observer.borrow().clone();
        if let Some(observer) = observer {
            observer.on_next(value);
        }
    }

    /// Pushes an error to the subscriber (`Fail`).
    pub fn fail(&self, error: ObservableError) {
        let observer = self.observer.borrow().clone();
        if let Some(observer) = observer {
            observer.on_error(error);
        }
    }

    /// Completes the subscriber (`Complete`).
    pub fn complete(&self) {
        let observer = self.observer.borrow().clone();
        if let Some(observer) = observer {
            observer.on_completed();
        }
    }

    /// The source as the parameters of a binding.
    pub fn observable(self: &Rc<Self>) -> Rc<dyn IObservable<CommandParameter>> {
        self.clone()
    }
}

impl IObservable<CommandParameter> for ValueSource {
    fn subscribe(&self, observer: Rc<dyn IObserver<CommandParameter>>) -> Rc<dyn IDisposable> {
        *self.observer.borrow_mut() = Some(observer);
        let slot = self.observer.clone();
        Disposable::create(move || {
            let previous = slot.borrow_mut().take();
            drop(previous);
        })
    }
}

// ---------------------------------------------------------------------------------------------
// `EventSource.cs`
// ---------------------------------------------------------------------------------------------

/// An object with an event the test raises (`EventSource`).
pub struct EventSource {
    fired: Event<()>,
}

impl EventSource {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { fired: Event::new() })
    }

    /// Whether any handler is attached (`HasHandlers`).
    pub fn has_handlers(&self) -> bool {
        self.fired.has_handlers()
    }

    /// Raises the event (`Raise`).
    pub fn raise(&self) {
        self.fired.raise(&());
    }

    /// The subscription of a handler to the event (`h => source.Fired += h`, `h => source.Fired -= h`).
    pub fn subscription(self: &Rc<Self>) -> CommandEventSubscription {
        let source = self.clone();
        Rc::new(move |handler: Rc<dyn Fn()>| {
            let token = source.fired.add(Rc::new(move |_: &()| handler()));
            let source = source.clone();
            Disposable::create(move || {
                source.fired.remove(token);
            })
        })
    }
}

// ---------------------------------------------------------------------------------------------
// `ListObserver.cs`
// ---------------------------------------------------------------------------------------------

/// An observer that records the values it receives (`ListObserver<T>`).
pub struct ListObserver<T> {
    values: RefCell<Vec<T>>,
}

impl<T: Clone + 'static> ListObserver<T> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { values: RefCell::new(Vec::new()) })
    }

    /// The values received, in order (`Values`).
    pub fn values(&self) -> Vec<T> {
        self.values.borrow().clone()
    }
}

impl<T: 'static> IObserver<T> for ListObserver<T> {
    fn on_next(&self, value: T) {
        self.values.borrow_mut().push(value);
    }

    fn on_error(&self, _error: ObservableError) {}

    fn on_completed(&self) {}
}

// ---------------------------------------------------------------------------------------------
// `RecordingCommand.cs`
// ---------------------------------------------------------------------------------------------

/// A command that records each parameter it runs with and executes only when it is enabled
/// (`RecordingCommand`).
pub struct RecordingCommand {
    enabled: Cell<bool>,
    executed: RefCell<Vec<Option<String>>>,
}

impl RecordingCommand {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { enabled: Cell::new(true), executed: RefCell::new(Vec::new()) })
    }

    /// Whether the command can execute (`Enabled`).
    pub fn set_enabled(&self, value: bool) {
        self.enabled.set(value);
    }

    /// The texts of the parameters the command ran with, in order (`Executed`): `None` for a
    /// parameter that is not text.
    pub fn executed(&self) -> Vec<Option<String>> {
        self.executed.borrow().clone()
    }

    /// The command as a command of ReactiveUI.
    pub fn command(self: &Rc<Self>) -> Rc<dyn ICommand> {
        self.clone()
    }
}

impl ICommand for RecordingCommand {
    fn can_execute(&self, _parameter: Option<&dyn Any>) -> bool {
        self.enabled.get()
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        let text = parameter.and_then(|value| value.downcast_ref::<String>()).cloned();
        self.executed.borrow_mut().push(text);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
