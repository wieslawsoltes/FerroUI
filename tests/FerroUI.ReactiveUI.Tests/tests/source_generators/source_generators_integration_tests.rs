//! `SourceGeneratorsIntegrationTests` (`ReactiveUI.Tests/SourceGenerators/SourceGeneratorsIntegrationTests.cs`).
//!
//! Upstream's view model is written by ReactiveUI.SourceGenerators from its attributes. The port
//! writes it with the macros and helpers that take their place: `[Reactive]` is
//! [`reactive_properties!`], `[ObservableAsProperty]` an [`ObservableAsPropertyHelper`] filled by
//! `to_property`, and `[ReactiveCommand]` a [`ReactiveCommand`] created in the constructor.

use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use ferroui_system::prop;
use ferroui_system::threading::tasks::{block_on, TaskCompletionSource};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IObservable, ObservableError};
use std::cell::{OnceCell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

/// `FirstName`.
const FIRST_NAME: &str = "Ada";
/// `LoadInput`.
const LOAD_INPUT: i32 = 20;

/// `Task.Yield()`: a future that is pending once.
struct YieldNow(bool);

impl Future for YieldNow {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        context.waker().wake_by_ref();
        Poll::Pending
    }
}

/// An error of the cancellation of the command (`OperationCanceledException`).
#[derive(Debug)]
struct OperationCanceled;

impl std::fmt::Display for OperationCanceled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The operation was canceled.")
    }
}

impl std::error::Error for OperationCanceled {}

reactive_properties! {
    /// The view model of the generators (`PersonViewModel`).
    pub struct PersonViewModel {
        reactive: ReactiveObject,
        #[reactive]
        /// `[Reactive] FirstName`.
        first_name: String,
        #[reactive]
        /// `[Reactive] LastName`.
        last_name: String,
        /// `_fullNameHelper`: `[ObservableAsProperty] FullName`.
        full_name_helper: OnceCell<ObservableAsPropertyHelper<String>>,
        /// `ClearCommand`: `[ReactiveCommand(CanExecute = nameof(_hasFirstName))] Clear`.
        clear_command: OnceCell<ReactiveCommand<(), ()>>,
        /// `LoadCommand`: `[ReactiveCommand] LoadAsync`.
        load_command: OnceCell<ReactiveCommand<i32, i32>>,
    }
}
reactive_object!(PersonViewModel);

impl PersonViewModel {
    fn new() -> Rc<Self> {
        let fixture = Rc::new(Self {
            reactive: ReactiveObject::new(),
            first_name: RefCell::new(FIRST_NAME.to_owned()),
            last_name: RefCell::new(String::new()),
            full_name_helper: OnceCell::new(),
            clear_command: OnceCell::new(),
            load_command: OnceCell::new(),
        });

        let helper = fixture
            .when_any_value((prop!(PersonViewModel => first_name), prop!(PersonViewModel => last_name)))
            .select(|(first, last): (String, String)| format!("{first} {last}"))
            .to_property(&fixture, prop!(PersonViewModel => full_name));
        let _ = fixture.full_name_helper.set(helper);

        // `_hasFirstName`, the gate of the command.
        let has_first_name: Rc<dyn IObservable<bool>> =
            fixture.when_any_value(prop!(PersonViewModel => first_name)).select(|name: String| !name.is_empty());

        let weak = Rc::downgrade(&fixture);
        let clear = ReactiveCommand::create_with(
            move || {
                if let Some(fixture) = weak.upgrade() {
                    fixture.clear();
                }
            },
            Some(has_first_name),
            None,
        );
        let _ = fixture.clear_command.set(clear);

        let load = ReactiveCommand::create_from_task_param_cancellable(|id: i32, token: CancellationToken| Self::load_async(id, token));
        let _ = fixture.load_command.set(load);

        fixture
    }

    /// `[ObservableAsProperty] FullName`.
    fn full_name(&self) -> String {
        self.full_name_helper.get().map(|helper| helper.value()).unwrap_or_default()
    }

    fn clear_command(&self) -> ReactiveCommand<(), ()> {
        self.clear_command.get().cloned().expect("the command is created with the view model")
    }

    fn load_command(&self) -> ReactiveCommand<i32, i32> {
        self.load_command.get().cloned().expect("the command is created with the view model")
    }

    /// `LoadAsync`.
    async fn load_async(id: i32, token: CancellationToken) -> Result<i32, ObservableError> {
        YieldNow(false).await;
        if token.is_cancellation_requested() {
            return Err(Rc::new(OperationCanceled));
        }
        Ok(id + 1)
    }

    /// `Clear`.
    fn clear(&self) {
        self.set_first_name(String::new());
    }
}

#[test]
fn reactive_property_raises_changing_then_changed() {
    let fixture = PersonViewModel::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let (changing, changed) = (events.clone(), events.clone());
    fixture.property_changing().add(Rc::new(move |name: &str| changing.borrow_mut().push(format!("changing:{name}"))));
    fixture.property_changed().add(Rc::new(move |name: &str| changed.borrow_mut().push(format!("changed:{name}"))));

    fixture.set_last_name("Lovelace".to_owned());

    let events = events.borrow();
    let changing_index = events.iter().position(|event| event == "changing:LastName");
    let changed_index = events.iter().position(|event| event == "changed:LastName");
    assert!(changing_index.is_some());
    assert!(changed_index.is_some());
    assert!(changing_index < changed_index);
}

#[test]
fn observable_as_property_follows_reactive_properties() {
    let fixture = PersonViewModel::new();
    fixture.set_last_name("Lovelace".to_owned());

    assert_eq!(fixture.full_name(), "Ada Lovelace");

    fixture.set_last_name("Byron".to_owned());

    assert_eq!(fixture.full_name(), "Ada Byron");
}

#[test]
fn reactive_command_runs_method_and_follows_can_execute() {
    let fixture = PersonViewModel::new();
    let can_execute = Rc::new(RefCell::new(Vec::new()));
    let sink = can_execute.clone();
    let can_execute_subscription = fixture.clear_command().can_execute().subscribe_next(move |value| sink.borrow_mut().push(value));

    let execution = fixture.clear_command().execute(()).subscribe_next(|_| {});
    assert_eq!(fixture.first_name(), "");
    execution.dispose();

    assert_eq!(*can_execute.borrow(), vec![true, false]);
    can_execute_subscription.dispose();
}

#[test]
fn reactive_command_from_async_method_returns_result() {
    let fixture = PersonViewModel::new();
    let result = TaskCompletionSource::<i32>::new();
    let (on_value, on_error) = (result.clone(), result.clone());

    let execution = fixture.load_command().execute(LOAD_INPUT).subscribe_next_error(
        move |value| {
            let _ = on_value.try_set_result(value);
        },
        move |error| {
            let _ = on_error.try_set_exception(error);
        },
    );

    // Upstream waits for the result for ten seconds at most; the task ends on this thread.
    assert_eq!(block_on(result.task()).expect("the command returns its result"), LOAD_INPUT + 1);
    execution.dispose();
}
