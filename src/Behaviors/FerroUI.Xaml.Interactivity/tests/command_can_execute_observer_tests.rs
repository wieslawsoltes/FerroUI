//! Ported from `tests/Xaml.Behaviors.Interactivity.UnitTests/CommandCanExecuteObserverTests.cs` of
//! Xaml.Behaviors.
//!
//! Adapted: the collection of the observer and of its callback target by the garbage collector is
//! the release of their last strong reference.

use crate::CommandCanExecuteObserver;
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A command that counts the subscriptions to its notification and the calls of `can_execute`
/// (`ObservableCommand`, declared by the three command test files of the reference).
pub(super) struct ObservableCommand {
    this: Weak<ObservableCommand>,
    can_execute_changed: HandlerList<dyn Fn()>,
    pub(super) can_execute_result: Cell<bool>,
    pub(super) can_execute_callback: RefCell<Option<Box<dyn Fn(Option<&BoxedValue>) -> bool>>>,
    subscription_count: Cell<i32>,
    last_can_execute_parameter: RefCell<Option<BoxedValue>>,
    can_execute_call_count: Cell<i32>,
}

impl ObservableCommand {
    pub(super) fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            can_execute_changed: HandlerList::new(),
            can_execute_result: Cell::new(true),
            can_execute_callback: RefCell::new(None),
            subscription_count: Cell::new(0),
            last_can_execute_parameter: RefCell::new(None),
            can_execute_call_count: Cell::new(0),
        })
    }

    pub(super) fn with_result(can_execute_result: bool) -> Rc<Self> {
        let command = Self::new();
        command.can_execute_result.set(can_execute_result);
        command
    }

    pub(super) fn with_callback(callback: impl Fn(Option<&BoxedValue>) -> bool + 'static) -> Rc<Self> {
        let command = Self::new();
        *command.can_execute_callback.borrow_mut() = Some(Box::new(callback));
        command
    }

    pub(super) fn subscription_count(&self) -> i32 {
        self.subscription_count.get()
    }

    pub(super) fn last_can_execute_parameter(&self) -> Option<BoxedValue> {
        self.last_can_execute_parameter.borrow().clone()
    }

    pub(super) fn can_execute_call_count(&self) -> i32 {
        self.can_execute_call_count.get()
    }

    pub(super) fn raise_can_execute_changed(&self) {
        for (_, handler) in self.can_execute_changed.snapshot().iter() {
            handler();
        }
    }

    /// The notification without a sender: the handlers of the port have no sender, so this is
    /// the notification itself.
    pub(super) fn raise_can_execute_changed_with_null_sender(&self) {
        self.raise_can_execute_changed();
    }

    /// The command as the contract.
    pub(super) fn as_command(self: &Rc<Self>) -> Option<Rc<dyn ICommand>> {
        Some(self.clone())
    }
}

impl ICommand for ObservableCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        self.can_execute_call_count.set(self.can_execute_call_count.get() + 1);
        *self.last_can_execute_parameter.borrow_mut() = parameter.cloned();
        match &*self.can_execute_callback.borrow() {
            Some(callback) => callback(parameter),
            None => self.can_execute_result.get(),
        }
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {}

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.can_execute_changed.add(handler);
        self.subscription_count.set(self.subscription_count.get() + 1);
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.can_execute_changed.remove(token);
                this.subscription_count.set(this.subscription_count.get() - 1);
            }
        })
    }
}

/// The text an untyped value holds.
pub(super) fn is_text(value: Option<&BoxedValue>, text: &str) -> bool {
    value.and_then(|value| value.downcast_ref::<String>()).is_some_and(|value| value == text)
}

fn text(value: &str) -> Option<BoxedValue> {
    Some(Rc::new(value.to_string()))
}

// Not ported: Constructor_ThrowsForNullCallback - the callback is a closure, which cannot be null.

#[test]
fn start_reports_true_when_command_is_null() {
    let can_execute = Rc::new(Cell::new(false));
    let target = can_execute.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set(value));

    observer.start(None, None);

    assert!(can_execute.get());
}

#[test]
fn update_uses_current_command_parameter() {
    let can_execute = Rc::new(Cell::new(false));
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let target = can_execute.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set(value));

    observer.start(command.as_command(), text("disabled"));
    assert!(!can_execute.get());

    observer.update(command.as_command(), text("enabled"));

    assert!(can_execute.get());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));
}

#[test]
fn start_with_unknown_parameter_reports_true_without_calling_can_execute() {
    let can_execute = Rc::new(Cell::new(false));
    let command = ObservableCommand::with_callback(|_| panic!("The parameter is not available yet."));
    let target = can_execute.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set(value));

    observer.start_with(command.as_command(), None, false);

    assert!(can_execute.get());
    assert_eq!(0, command.can_execute_call_count());
}

#[test]
fn can_execute_changed_with_unknown_parameter_does_not_call_can_execute() {
    let can_execute = Rc::new(Cell::new(false));
    let command = ObservableCommand::with_callback(|_| panic!("The parameter is not available yet."));
    let target = can_execute.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set(value));

    observer.start_with(command.as_command(), None, false);
    command.raise_can_execute_changed();

    assert!(can_execute.get());
    assert_eq!(0, command.can_execute_call_count());
}

#[test]
fn update_from_unknown_to_known_parameter_probes_current_parameter() {
    let can_execute = Rc::new(Cell::new(false));
    let command = ObservableCommand::with_callback(|parameter| is_text(parameter, "enabled"));
    let target = can_execute.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set(value));

    observer.start_with(command.as_command(), None, false);
    observer.update_with(command.as_command(), text("enabled"), true);

    assert!(can_execute.get());
    assert!(is_text(command.last_can_execute_parameter().as_ref(), "enabled"));
    assert_eq!(1, command.can_execute_call_count());
}

#[test]
fn update_rewires_command_subscriptions() {
    let can_execute = Rc::new(Cell::new(false));
    let first_command = ObservableCommand::with_result(false);
    let second_command = ObservableCommand::with_result(true);
    let target = can_execute.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set(value));

    observer.start(first_command.as_command(), None);

    assert_eq!(1, first_command.subscription_count());
    assert!(!can_execute.get());

    observer.update(second_command.as_command(), None);

    assert_eq!(0, first_command.subscription_count());
    assert_eq!(1, second_command.subscription_count());
    assert!(can_execute.get());
}

#[test]
fn dispose_removes_command_subscription() {
    let command = ObservableCommand::new();
    let observer = CommandCanExecuteObserver::new(|_| {});

    observer.start(command.as_command(), None);
    assert_eq!(1, command.subscription_count());

    observer.dispose();

    assert_eq!(0, command.subscription_count());
}

// Not ported: CanExecuteChanged_FromBackgroundThread_UpdatesCallbackOnUiThread - commands and
// observers of the port belong to the thread that created them (they are not `Send`), so a
// command cannot raise its notification from another thread.

/// `CallbackTarget`.
struct CallbackTarget {
    can_execute: Cell<bool>,
}

impl CallbackTarget {
    fn set_can_execute(&self, value: bool) {
        self.can_execute.set(value);
    }
}

/// Starts an observer of `command` whose callback is a method of a target, and releases both.
fn create_and_release_observer(command: &Rc<ObservableCommand>) -> (Weak<impl Sized>, Weak<CallbackTarget>) {
    let callback_target = Rc::new(CallbackTarget { can_execute: Cell::new(false) });
    let target = callback_target.clone();
    let observer = CommandCanExecuteObserver::new(move |value| target.set_can_execute(value));

    observer.start(command.as_command(), None);

    (Rc::downgrade(&observer.inner), Rc::downgrade(&callback_target))
}

#[test]
fn command_can_execute_changed_subscription_does_not_retain_observer_or_callback_target() {
    let command = ObservableCommand::new();

    let (observer, callback_target) = create_and_release_observer(&command);

    assert_eq!(1, command.subscription_count());
    assert!(observer.upgrade().is_none());
    assert!(callback_target.upgrade().is_none());

    command.raise_can_execute_changed_with_null_sender();

    assert_eq!(0, command.subscription_count());
}
