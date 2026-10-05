//! A command for the tests of this crate.

#![allow(dead_code)]

use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type Handlers = Vec<(u64, Rc<dyn Fn()>)>;

/// A command whose behaviour is given by closures or by an enabled flag,
/// and which counts the subscriptions to its "can execute changed" event.
pub struct TestCommand {
    this: Weak<TestCommand>,
    can_execute: Box<dyn Fn(Option<&BoxedValue>) -> bool>,
    execute: Box<dyn Fn(Option<&BoxedValue>)>,
    can_execute_changed: RefCell<Handlers>,
    next_token: Cell<u64>,
    enabled: Rc<Cell<bool>>,
    subscription_count: Cell<i32>,
}

impl TestCommand {
    /// Creates a command that can execute while it is enabled.
    pub fn new(enabled: bool) -> Rc<Self> {
        let enabled = Rc::new(Cell::new(enabled));
        let flag = enabled.clone();
        Self::create(enabled, Box::new(move |_| flag.get()), Box::new(|_| {}))
    }

    /// Creates a command from a "can execute" predicate.
    pub fn with_can_execute(can_execute: impl Fn(Option<&BoxedValue>) -> bool + 'static) -> Rc<Self> {
        Self::create(Rc::new(Cell::new(true)), Box::new(can_execute), Box::new(|_| {}))
    }

    /// Creates a command from a "can execute" predicate and an action.
    pub fn with_can_execute_and_execute(
        can_execute: impl Fn(Option<&BoxedValue>) -> bool + 'static,
        execute: impl Fn(Option<&BoxedValue>) + 'static,
    ) -> Rc<Self> {
        Self::create(Rc::new(Cell::new(true)), Box::new(can_execute), Box::new(execute))
    }

    fn create(
        enabled: Rc<Cell<bool>>,
        can_execute: Box<dyn Fn(Option<&BoxedValue>) -> bool>,
        execute: Box<dyn Fn(Option<&BoxedValue>)>,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            can_execute,
            execute,
            can_execute_changed: RefCell::new(Vec::new()),
            next_token: Cell::new(0),
            enabled,
            subscription_count: Cell::new(0),
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
    }

    pub fn set_is_enabled(&self, value: bool) {
        if self.enabled.get() != value {
            self.enabled.set(value);
            self.raise_can_execute_changed();
        }
    }

    /// The number of handlers currently subscribed to the "can execute
    /// changed" event.
    pub fn subscription_count(&self) -> i32 {
        self.subscription_count.get()
    }

    pub fn raise_can_execute_changed(&self) {
        let handlers: Vec<Rc<dyn Fn()>> = self.can_execute_changed.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler();
        }
    }

    /// The command as a value for a `Command` property.
    pub fn as_command(self: &Rc<Self>) -> Option<Rc<dyn ICommand>> {
        Some(self.clone())
    }
}

impl ICommand for TestCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        (self.can_execute)(parameter)
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        (self.execute)(parameter)
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.can_execute_changed.borrow_mut().push((token, handler));
        self.subscription_count.set(self.subscription_count.get() + 1);

        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_changed.borrow_mut().retain(|(t, _)| *t != token);
                this.subscription_count.set(this.subscription_count.get() - 1);
            }
        })
    }
}
