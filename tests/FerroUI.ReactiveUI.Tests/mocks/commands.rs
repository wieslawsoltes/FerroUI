//! The objects of the tests of commands: the mocks of `ReactiveUI.Tests/Commands/Mocks`
//! (`CommandHolder`, `ReactiveCommandHolder`, `FakeCommand`), `Foo` and `FooViewModel`
//! (`ReactiveUI.Tests/Mocks`), the command and view model of the tests of ReactiveUI.Binding
//! (`TestModels/RecordingCommand.cs`, `TestModels/DispatchStubViewModel.cs`), and the helpers the
//! ported tests share in place of the asynchronous assertions of TUnit.

use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use ferroui_system::threading::tasks::{block_on, run, Task, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{prop, IDisposable, IObservable, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::Duration;

// ---- Errors --------------------------------------------------------------------------------

/// The `InvalidOperationException` the tests throw: an error with a message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOperationException {
    /// `Message`.
    pub message: String,
}

impl InvalidOperationException {
    /// An error with `message`.
    pub fn new(message: &str) -> Self {
        Self { message: message.to_owned() }
    }

    /// The error as the error of a sequence.
    pub fn error(message: &str) -> ObservableError {
        Rc::new(Self::new(message))
    }
}

impl fmt::Display for InvalidOperationException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for InvalidOperationException {}

/// Whether `error` is an [`InvalidOperationException`] (`IsTypeOf<InvalidOperationException>()`).
pub fn is_invalid_operation(error: &ObservableError) -> bool {
    error.downcast_ref::<InvalidOperationException>().is_some()
}

// ---- Schedulers and awaiting ----------------------------------------------------------------

/// The immediate scheduler (`Sequencer.Immediate`).
pub fn immediate() -> Rc<dyn IScheduler> {
    ImmediateScheduler::instance()
}

/// What `WithSchedulerExecutor` sets up: the main thread and background schedulers of
/// `RxSchedulers` are the immediate scheduler. The state is per thread, and each test runs on its
/// own thread.
pub fn with_immediate_schedulers() -> Rc<dyn IScheduler> {
    let scheduler = immediate();
    RxSchedulers::set_main_thread_scheduler(scheduler.clone());
    RxSchedulers::set_task_pool_scheduler(scheduler.clone());
    scheduler
}

/// What `WithVirtualTimeSchedulerExecutor` sets up: both schedulers of `RxSchedulers` are `scheduler`.
pub fn with_schedulers(scheduler: Rc<dyn IScheduler>) {
    RxSchedulers::set_main_thread_scheduler(scheduler.clone());
    RxSchedulers::set_task_pool_scheduler(scheduler);
}

/// The first value of `source` (`await source.FirstAsync()`), running the tasks of the thread
/// until it arrives.
pub fn first<T: Clone + 'static>(source: &Rc<dyn IObservable<T>>) -> T {
    block_on(source.first_async().to_task()).unwrap_or_else(|error| panic!("the sequence failed: {error}"))
}

/// The last value of `source` (`await source`), running the tasks of the thread until it ends.
pub fn last<T: Clone + 'static>(source: &Rc<dyn IObservable<T>>) -> T {
    block_on(source.to_task()).unwrap_or_else(|error| panic!("the sequence failed: {error}"))
}

/// Subscribes to `source` and ignores its values and its error (`Subscribe(_ => { }, _ => { })`).
pub fn subscribe_ignoring_errors<T: 'static>(source: &Rc<dyn IObservable<T>>) -> Rc<dyn IDisposable> {
    source.subscribe(ferroui_reactive::Observer::create_all(|_| {}, |_| {}, || {}))
}

/// The message of the panic of `action` (`Assert.Throws<T>(...)` and its message).
pub fn panic_message(action: impl FnOnce()) -> String {
    let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).expect_err("the action should panic");
    payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| (*text).to_owned()))
        .unwrap_or_default()
}

/// A future that ends after `duration`, or with a cancellation when `token` is cancelled first
/// (`Task.Delay(TimeSpan, CancellationToken)`). The time passes on a worker thread, and the task
/// ends on the thread that awaits it.
pub fn delay(duration: Duration, token: CancellationToken) -> Task<()> {
    let completion = TaskCompletionSource::<()>::new();
    let elapsed = completion.clone();
    let timer = run(move || {
        std::thread::sleep(duration);
        Ok(())
    });
    let _ = timer.continue_with(move |_| {
        elapsed.try_set_result(());
    });
    if token.can_be_canceled() {
        let canceled = completion.clone();
        let registration = token.register_local(move || {
            canceled.try_set_canceled();
        });
        let _ = completion.task().continue_with(move |_| registration.dispose());
    }
    completion.task()
}

/// A future that is pending once and wakes itself (`Task.Yield()`).
pub fn yield_now() -> impl Future<Output = Result<(), ObservableError>> {
    struct YieldNow {
        yielded: bool,
    }
    impl Future for YieldNow {
        type Output = Result<(), ObservableError>;

        fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
            if self.yielded {
                return Poll::Ready(Ok(()));
            }
            self.yielded = true;
            context.waker().wake_by_ref();
            Poll::Pending
        }
    }
    YieldNow { yielded: false }
}

// ---- Commands/Mocks ---------------------------------------------------------------------------

/// A command that records the parameters it was asked about and executed with (`FakeCommand`);
/// it can always execute. The parameters are read as `T`.
pub struct FakeCommand<T> {
    can_execute_parameter: RefCell<Option<T>>,
    execute_parameter: RefCell<Option<T>>,
    can_execute_changed: Event<()>,
}

impl<T: Clone + 'static> FakeCommand<T> {
    /// A command that recorded nothing.
    pub fn new() -> Rc<Self> {
        Rc::new(Self { can_execute_parameter: RefCell::new(None), execute_parameter: RefCell::new(None), can_execute_changed: Event::new() })
    }

    /// The parameter of the last `can_execute` (`CanExecuteParameter`).
    pub fn can_execute_parameter(&self) -> Option<T> {
        self.can_execute_parameter.borrow().clone()
    }

    /// The parameter of the last `execute` (`ExecuteParameter`).
    pub fn execute_parameter(&self) -> Option<T> {
        self.execute_parameter.borrow().clone()
    }

    /// Raises `CanExecuteChanged` (`NotifyCanExecuteChanged`).
    pub fn notify_can_execute_changed(&self) {
        self.can_execute_changed.raise(&());
    }
}

impl<T: Clone + 'static> ICommand for FakeCommand<T> {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        *self.can_execute_parameter.borrow_mut() = parameter.and_then(|parameter| parameter.downcast_ref::<T>().cloned());
        true
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        *self.execute_parameter.borrow_mut() = parameter.and_then(|parameter| parameter.downcast_ref::<T>().cloned());
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let _ = self.can_execute_changed.add(Rc::new(move |_: &()| handler()));
        Disposable::empty()
    }
}

reactive_properties! {
    /// A view model holding a command (`CommandHolder`).
    #[derive(Default)]
    pub struct CommandHolder {
        reactive: ReactiveObject,
        #[reactive]
        the_command: Option<Rc<dyn ICommand>>,
    }
}
reactive_object!(CommandHolder);

impl CommandHolder {
    /// A holder without a command.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

reactive_properties! {
    /// A view model holding a reactive command (`ReactiveCommandHolder`).
    #[derive(Default)]
    pub struct ReactiveCommandHolder {
        reactive: ReactiveObject,
        #[reactive]
        the_command: Option<ReactiveCommand<i32, ()>>,
    }
}
reactive_object!(ReactiveCommandHolder);

impl ReactiveCommandHolder {
    /// A holder without a command.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
}

// ---- Mocks/Foo, Mocks/FooViewModel ----------------------------------------------------------------

/// A model whose value is set asynchronously (`Foo`).
pub struct Foo {
    value: Cell<i32>,
}

impl Default for Foo {
    fn default() -> Self {
        Self { value: Cell::new(42) }
    }
}

impl Foo {
    /// A model whose value is 42.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// `Value`.
    pub fn value(&self) -> i32 {
        self.value.get()
    }

    /// Sets the value, and gives a completed task (`SetValueAsync`).
    pub fn set_value_async(&self, value: i32) -> Task<()> {
        self.value.set(value);
        Task::from_result(())
    }
}

reactive_properties! {
    /// A view model that sets the value of its [`Foo`] to its setpoint, 10 ms after each change, on
    /// the background scheduler (`FooViewModel`).
    pub struct FooViewModel {
        reactive: ReactiveObject,
        foo: Rc<Foo>,
        subscription: RefCell<Option<Rc<dyn IDisposable>>>,
        #[reactive]
        setpoint: i32,
    }
}
reactive_object!(FooViewModel);

impl FooViewModel {
    /// A view model of `foo`, which subscribes to its own setpoint.
    pub fn new(foo: Rc<Foo>) -> Rc<Self> {
        let view_model = Rc::new(Self {
            reactive: ReactiveObject::default(),
            foo: foo.clone(),
            subscription: RefCell::new(None),
            setpoint: RefCell::new(0),
        });
        let subscription = view_model
            .when_any_value(prop!(FooViewModel => setpoint))
            .select_many(move |value| {
                let foo = foo.clone();
                Observable::from_async(move || foo.set_value_async(value))
                    .delay_subscription_on(Duration::from_millis(10), RxSchedulers::task_pool_scheduler())
            })
            .subscribe_ignore();
        *view_model.subscription.borrow_mut() = Some(subscription);
        view_model
    }

    /// `Foo`.
    pub fn foo(&self) -> Rc<Foo> {
        self.foo.clone()
    }
}

// ---- ReactiveUI.Binding.Tests/TestModels -----------------------------------------------------

/// A command that records its executions (`RecordingCommand`); it asks `can_execute_result` per
/// value. The parameters are read as `String`.
pub struct RecordingCommand {
    /// `CanExecuteResult`.
    pub can_execute_result: Cell<bool>,
    execute_count: Cell<usize>,
    last_parameter: RefCell<Option<String>>,
    last_questioned_parameter: RefCell<Option<String>>,
}

fn as_string(parameter: Option<&dyn Any>) -> Option<String> {
    let parameter = parameter?;
    parameter.downcast_ref::<String>().cloned().or_else(|| parameter.downcast_ref::<&str>().map(|text| (*text).to_owned()))
}

impl RecordingCommand {
    /// A command that can execute and recorded nothing.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            can_execute_result: Cell::new(true),
            execute_count: Cell::new(0),
            last_parameter: RefCell::new(None),
            last_questioned_parameter: RefCell::new(None),
        })
    }

    /// `ExecuteCount`.
    pub fn execute_count(&self) -> usize {
        self.execute_count.get()
    }

    /// `LastParameter`.
    pub fn last_parameter(&self) -> Option<String> {
        self.last_parameter.borrow().clone()
    }

    /// `LastQuestionedParameter`.
    pub fn last_questioned_parameter(&self) -> Option<String> {
        self.last_questioned_parameter.borrow().clone()
    }
}

impl ICommand for RecordingCommand {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        *self.last_questioned_parameter.borrow_mut() = as_string(parameter);
        self.can_execute_result.get()
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        self.execute_count.set(self.execute_count.get() + 1);
        *self.last_parameter.borrow_mut() = as_string(parameter);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        // CanExecute is asked per emission rather than tracked from the event.
        Disposable::empty()
    }
}

/// A view model that raises `PropertyChanged` for its text properties and holds a command
/// (`DispatchStubViewModel`); like upstream's, `Run` does not notify its changes.
pub struct DispatchStubViewModel {
    property_changed: Event<str>,
    caption: RefCell<String>,
    parameter: RefCell<String>,
    run: RefCell<Option<Rc<dyn ICommand>>>,
}

impl DispatchStubViewModel {
    /// A view model whose texts are `"a"` and that has no command.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            property_changed: Event::new(),
            caption: RefCell::new("a".to_owned()),
            parameter: RefCell::new("a".to_owned()),
            run: RefCell::new(None),
        })
    }

    /// `Caption`.
    pub fn caption(&self) -> String {
        self.caption.borrow().clone()
    }

    /// Sets `Caption` and raises `PropertyChanged`.
    pub fn set_caption(&self, value: &str) {
        *self.caption.borrow_mut() = value.to_owned();
        self.property_changed.raise("Caption");
    }

    /// `Parameter`.
    pub fn parameter(&self) -> String {
        self.parameter.borrow().clone()
    }

    /// Sets `Parameter` and raises `PropertyChanged`.
    pub fn set_parameter(&self, value: &str) {
        *self.parameter.borrow_mut() = value.to_owned();
        self.property_changed.raise("Parameter");
    }

    /// `Run`.
    pub fn run(&self) -> Option<Rc<dyn ICommand>> {
        self.run.borrow().clone()
    }

    /// Sets `Run`, without notification.
    pub fn set_run(&self, value: Option<Rc<dyn ICommand>>) {
        *self.run.borrow_mut() = value;
    }
}

impl INotifyPropertyChanged for DispatchStubViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}
