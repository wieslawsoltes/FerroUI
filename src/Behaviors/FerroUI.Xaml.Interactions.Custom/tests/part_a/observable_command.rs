//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/ObservableCommand.cs` of
//! Xaml.Behaviors.

use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// A command that counts the subscriptions to its can-execute change and records its can-execute
/// calls.
pub(crate) struct ObservableCommand {
    this: Weak<ObservableCommand>,
    can_execute_changed: HandlerList<dyn Fn()>,
    pub(crate) can_execute_result: Cell<bool>,
    pub(crate) can_execute_callback: RefCell<Option<Box<dyn Fn(Option<&BoxedValue>) -> bool>>>,
    subscription_count: Cell<i32>,
    last_can_execute_parameter: RefCell<Option<BoxedValue>>,
    can_execute_call_count: Cell<i32>,
}

impl ObservableCommand {
    /// `new ObservableCommand()`.
    pub(crate) fn new() -> Rc<Self> {
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

    /// `new ObservableCommand { CanExecuteResult = can_execute_result }`.
    pub(crate) fn with_result(can_execute_result: bool) -> Rc<Self> {
        let command = Self::new();
        command.can_execute_result.set(can_execute_result);
        command
    }

    /// `new ObservableCommand { CanExecuteCallback = callback }`.
    pub(crate) fn with_callback(callback: impl Fn(Option<&BoxedValue>) -> bool + 'static) -> Rc<Self> {
        let command = Self::new();
        *command.can_execute_callback.borrow_mut() = Some(Box::new(callback));
        command
    }

    pub(crate) fn subscription_count(&self) -> i32 {
        self.subscription_count.get()
    }

    pub(crate) fn last_can_execute_parameter(&self) -> Option<BoxedValue> {
        self.last_can_execute_parameter.borrow().clone()
    }

    pub(crate) fn can_execute_call_count(&self) -> i32 {
        self.can_execute_call_count.get()
    }

    pub(crate) fn raise_can_execute_changed(&self) {
        for (_, handler) in self.can_execute_changed.snapshot().iter() {
            handler();
        }
    }

    /// The command as the contract.
    pub(crate) fn as_command(self: &Rc<Self>) -> Option<Rc<dyn ICommand>> {
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
pub(crate) fn is_text(value: Option<&BoxedValue>, text: &str) -> bool {
    value.and_then(|value| value.downcast_ref::<String>()).is_some_and(|value| value == text)
}
