//! Port of `ViewModels/CommandLabViewModel.cs`: demonstrates command can-execute, asynchronous
//! execution, failure handling, and interactions.

use super::page_view_model::page_view_model;
use super::PageViewModel;
use crate::services::{InvalidOperationError, SystemTimeProvider, TimeProvider};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand as FerroCommand;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{IReactiveObjectExt, IScreen, Interaction, ReactiveCommand, ReactiveUIBindingExtensions};
use ferroui_system::prop;
use ferroui_reactiveui_controls::ReactiveCommandAdapter;
use ferroui_system::{IDisposable, ObservableError};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The simulated command work delay (`WorkDelay`).
const WORK_DELAY: Duration = Duration::from_millis(250);

/// The simulated failure delay (`FailureDelay`).
const FAILURE_DELAY: Duration = Duration::from_millis(100);

/// The message of the failing command.
pub const FAILURE_MESSAGE: &str = "The sample command failed by design.";

/// Demonstrates command can-execute, asynchronous execution, failure handling, and interactions
/// (`CommandLabViewModel`).
///
/// Deviation (DEVIATIONS.md, samples): `Task.Delay` of the asynchronous work is a timer of the
/// dispatcher (`mini_mvvm::delay`), as the tasks of the port run on the thread that starts them.
pub struct CommandLabViewModel {
    page: PageViewModel,
    /// A weak handle to the view model, for the handlers it registers.
    this: Weak<CommandLabViewModel>,
    /// The time provider (`_timeProvider`).
    time_provider: Rc<dyn TimeProvider>,
    /// The command that runs asynchronous work (`RunWork`).
    run_work: OnceCell<ReactiveCommand<(), String>>,
    /// The command that intentionally fails (`FailWork`).
    fail_work: OnceCell<ReactiveCommand<(), ()>>,
    /// The commands of the controls of FerroUI that run the two commands, for the bindings of markup.
    run_work_command: OnceCell<Rc<dyn FerroCommand>>,
    fail_work_command: OnceCell<Rc<dyn FerroCommand>>,
    /// The interaction used by the view to present command errors (`ReportError`).
    report_error: Interaction<String, ()>,
    /// The command text input (`WorkItemText`).
    work_item_text: RefCell<String>,
    /// The last result text (`LastResult`).
    last_result: RefCell<String>,
    /// The last error text (`LastError`).
    last_error: RefCell<String>,
    /// The interaction acknowledgement text (`InteractionStatus`).
    interaction_status: RefCell<String>,
    /// The number of errors acknowledged by the view (`AcknowledgedErrors`).
    acknowledged_errors: Cell<i32>,
}
page_view_model!(CommandLabViewModel);

impl CommandLabViewModel {
    /// The command lab of `host_screen` on the clock of the system
    /// (`CommandLabViewModel(IScreen)`).
    pub fn new(host_screen: &Rc<dyn IScreen>) -> Rc<Self> {
        Self::with_time_provider(host_screen, Rc::new(SystemTimeProvider))
    }

    /// The command lab of `host_screen` on `time_provider`
    /// (`CommandLabViewModel(IScreen, TimeProvider)`).
    pub fn with_time_provider(host_screen: &Rc<dyn IScreen>, time_provider: Rc<dyn TimeProvider>) -> Rc<Self> {
        let view_model = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            page: PageViewModel::new(
                host_screen,
                "commands",
                "Command lab",
                "Exercises ReactiveCommand can-execute, async work, thrown exceptions, and interactions.",
            ),
            time_provider,
            run_work: OnceCell::new(),
            fail_work: OnceCell::new(),
            run_work_command: OnceCell::new(),
            fail_work_command: OnceCell::new(),
            report_error: Interaction::new(),
            work_item_text: RefCell::new(String::new()),
            last_result: RefCell::new("No command has run yet.".to_owned()),
            last_error: RefCell::new("No errors reported.".to_owned()),
            interaction_status: RefCell::new("The view has not handled an interaction yet.".to_owned()),
            acknowledged_errors: Cell::new(0),
        });
        let weak = Rc::downgrade(&view_model);
        let base = view_model.page.base();

        let can_run = view_model.when_any_value_select(prop!(CommandLabViewModel => work_item_text), |text: String| !text.trim().is_empty());
        let run_weak = weak.clone();
        let run_work = base.track(ReactiveCommand::create_from_task_with(
            move || Self::run_work_async(run_weak.clone()),
            Some(can_run),
            None,
        ));
        let fail_work = base.track(ReactiveCommand::create_from_task(Self::fail_work_async));

        let (on_result, on_result_error) = (weak.clone(), weak.clone());
        let _ = base.track_disposable(run_work.as_observable().subscribe_next_error(
            move |result: String| with_view_model(&on_result, |view_model| view_model.set_last_result(result)),
            move |error| with_view_model(&on_result_error, |view_model| view_model.handle_error(&error)),
        ));
        let (on_run_error, on_run_error_error) = (weak.clone(), weak.clone());
        let _ = base.track_disposable(run_work.thrown_exceptions().subscribe_next_error(
            move |error| with_view_model(&on_run_error, |view_model| view_model.handle_error(&error)),
            move |error| with_view_model(&on_run_error_error, |view_model| view_model.handle_error(&error)),
        ));
        let (on_fail_error, on_fail_error_error) = (weak.clone(), weak);
        let _ = base.track_disposable(fail_work.thrown_exceptions().subscribe_next_error(
            move |error| with_view_model(&on_fail_error, |view_model| view_model.handle_error(&error)),
            move |error| with_view_model(&on_fail_error_error, |view_model| view_model.handle_error(&error)),
        ));

        let _ = view_model.run_work_command.set(ReactiveCommandAdapter::command_of(Rc::new(run_work.clone())));
        let _ = view_model.fail_work_command.set(ReactiveCommandAdapter::command_of(Rc::new(fail_work.clone())));
        let _ = view_model.run_work.set(run_work);
        let _ = view_model.fail_work.set(fail_work);
        view_model
    }

    /// The command that runs asynchronous work (`RunWork`).
    pub fn run_work(&self) -> ReactiveCommand<(), String> {
        self.run_work.get().expect("the command is created with the view model").clone()
    }

    /// The command that intentionally fails (`FailWork`).
    pub fn fail_work(&self) -> ReactiveCommand<(), ()> {
        self.fail_work.get().expect("the command is created with the view model").clone()
    }

    /// The interaction used by the view to present command errors (`ReportError`).
    pub fn report_error(&self) -> Interaction<String, ()> {
        self.report_error.clone()
    }

    /// The command text input (`WorkItemText { get; }`).
    pub fn work_item_text(&self) -> String {
        self.work_item_text.borrow().clone()
    }

    /// Sets the command text input (`WorkItemText { set; }`).
    pub fn set_work_item_text(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.work_item_text, value, "WorkItemText");
    }

    /// The last result text (`LastResult { get; }`).
    pub fn last_result(&self) -> String {
        self.last_result.borrow().clone()
    }

    /// Sets the last result text (`LastResult { set; }`).
    pub fn set_last_result(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.last_result, value, "LastResult");
    }

    /// The last error text (`LastError { get; }`).
    pub fn last_error(&self) -> String {
        self.last_error.borrow().clone()
    }

    /// Sets the last error text (`LastError { set; }`).
    pub fn set_last_error(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.last_error, value, "LastError");
    }

    /// The interaction acknowledgement text (`InteractionStatus { get; }`).
    pub fn interaction_status(&self) -> String {
        self.interaction_status.borrow().clone()
    }

    /// Sets the interaction acknowledgement text (`InteractionStatus { set; }`).
    pub fn set_interaction_status(&self, value: String) {
        let _ = self.raise_and_set_if_changed(&self.interaction_status, value, "InteractionStatus");
    }

    /// The number of errors acknowledged by the view (`AcknowledgedErrors { get; }`).
    pub fn acknowledged_errors(&self) -> i32 {
        self.acknowledged_errors.get()
    }

    /// Sets the number of errors acknowledged by the view (`AcknowledgedErrors { set; }`).
    pub fn set_acknowledged_errors(&self, value: i32) {
        let _ = self.raise_and_set_if_changed(&self.acknowledged_errors, value, "AcknowledgedErrors");
    }

    /// Records that the view handled an error interaction (`AcknowledgeError(string)`).
    pub fn acknowledge_error(&self, message: &str) {
        self.set_acknowledged_errors(self.acknowledged_errors() + 1);
        self.set_interaction_status(format!("View acknowledged: {message}"));
    }

    /// Intentionally fails to demonstrate the error flow of a command (`FailWorkAsync()`): always
    /// an [`InvalidOperationError`], after the simulated delay.
    async fn fail_work_async() -> Result<(), ObservableError> {
        mini_mvvm::delay(FAILURE_DELAY).await;
        Err(InvalidOperationError::new(FAILURE_MESSAGE).into_error())
    }

    /// Runs the simulated asynchronous command work (`RunWorkAsync()`): the input is captured
    /// before the work is awaited.
    fn run_work_async(view_model: Weak<Self>) -> impl std::future::Future<Output = Result<String, ObservableError>> {
        let captured = view_model.upgrade().map(|view_model| (view_model.work_item_text().trim().to_owned(), view_model.time_provider.clone()));
        async move {
            let Some((text, time_provider)) = captured else {
                return Err(InvalidOperationError::new("The command lab was disposed.").into_error());
            };
            mini_mvvm::delay(WORK_DELAY).await;
            Ok(format!("Processed '{text}' at {}.", time_provider.get_local_now().to_string_with("T")))
        }
    }

    /// Handles command errors and notifies the interaction (`HandleError(Exception)`).
    fn handle_error(&self, error: &ObservableError) {
        let message = error.to_string();
        self.set_last_error(message.clone());
        let weak = self.this.clone();
        let _ = self.report_error.when_handled(message).subscribe_next_error(
            |_: ()| {},
            move |interaction_error| {
                with_view_model(&weak, |view_model| {
                    view_model.set_interaction_status(format!("No active view handled the interaction: {interaction_error}"))
                })
            },
        );
    }
}

impl IDisposable for CommandLabViewModel {
    /// `Dispose()`.
    fn dispose(&self) {
        self.page.base().dispose();
    }
}

/// Runs `action` on the view model while it is alive.
fn with_view_model(view_model: &Weak<CommandLabViewModel>, action: impl FnOnce(&CommandLabViewModel)) {
    if let Some(view_model) = view_model.upgrade() {
        action(&view_model);
    }
}

ferro_markup_type!(class CommandLabViewModel {
    this: Rc<CommandLabViewModel>,
    handles: [CommandLabViewModel, Rc<CommandLabViewModel>, Option<Rc<CommandLabViewModel>>],
    properties: [
        Title: String { get: |this: &Rc<CommandLabViewModel>| this.title() },
        Summary: String { get: |this: &Rc<CommandLabViewModel>| this.summary() },
        WorkItemText: String {
            get: |this: &Rc<CommandLabViewModel>| this.work_item_text(),
            set: |this: &Rc<CommandLabViewModel>, value: String| this.set_work_item_text(value)
        },
        LastResult: String { get: |this: &Rc<CommandLabViewModel>| this.last_result() },
        LastError: String { get: |this: &Rc<CommandLabViewModel>| this.last_error() },
        InteractionStatus: String { get: |this: &Rc<CommandLabViewModel>| this.interaction_status() },
        AcknowledgedErrors: i32 { get: |this: &Rc<CommandLabViewModel>| this.acknowledged_errors() },
        RunWork: Rc<dyn FerroCommand> {
            get: |this: &Rc<CommandLabViewModel>| this.run_work_command.get().expect("the command is created with the view model").clone()
        },
        FailWork: Rc<dyn FerroCommand> {
            get: |this: &Rc<CommandLabViewModel>| this.fail_work_command.get().expect("the command is created with the view model").clone()
        },
    ],
    notify_property_changed: CommandLabViewModel,
});
