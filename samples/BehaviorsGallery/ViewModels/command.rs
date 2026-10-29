//! Not from the reference: the commands of the view models of the gallery.
//!
//! The view models of the reference create their commands with `ReactiveCommand.Create(..)` and
//! hand them to the controls as `ICommand`, whose parameter is any object (`object?`): the picked
//! files of a picker action, the event arguments of a trigger, an item of a list.
//!
//! Deviation (DEVIATIONS.md, Behaviors gallery): the commands of ReactiveUI of the port take a
//! parameter of one Rust type and panic on another, so the commands of the view models are
//! commands of the controls over a closure that receives the untyped parameter; the can-execute
//! observable of `ReactiveCommand.Create(.., canExecute)` is a predicate the view model raises
//! [`Command::raise_can_execute_changed`] for when the value it reads changes.

use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type Handlers = Vec<(u64, Rc<dyn Fn()>)>;

/// A command that runs a closure with the untyped parameter it is executed with
/// (`ReactiveCommand.Create<object?>(..)`).
pub struct Command {
    this: Weak<Command>,
    execute: Box<dyn Fn(Option<&BoxedValue>)>,
    can_execute: Box<dyn Fn() -> bool>,
    can_execute_changed: RefCell<Handlers>,
    next_token: Cell<u64>,
}

impl Command {
    /// A command that runs `execute` without its parameter (`ReactiveCommand.Create(Action)`).
    pub fn create(execute: impl Fn() + 'static) -> Rc<dyn ICommand> {
        Self::new(move |_| execute(), || true)
    }

    /// A command that runs `execute` with its parameter (`ReactiveCommand.Create<T>(Action<T>)`).
    pub fn create_with(execute: impl Fn(Option<&BoxedValue>) + 'static) -> Rc<dyn ICommand> {
        Self::new(execute, || true)
    }

    /// A command that runs `execute` with its parameter while `can_execute` is true
    /// (`ReactiveCommand.Create<T>(Action<T>, IObservable<bool>)`).
    pub fn new(
        execute: impl Fn(Option<&BoxedValue>) + 'static,
        can_execute: impl Fn() -> bool + 'static,
    ) -> Rc<Command> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            execute: Box::new(execute),
            can_execute: Box::new(can_execute),
            can_execute_changed: RefCell::new(Vec::new()),
            next_token: Cell::new(0),
        })
    }

    /// Tells the controls that run the command to ask again whether it can execute.
    pub fn raise_can_execute_changed(&self) {
        let handlers: Vec<Rc<dyn Fn()>> = self
            .can_execute_changed
            .borrow()
            .iter()
            .map(|(_, handler)| handler.clone())
            .collect();
        for handler in handlers {
            handler();
        }
    }
}

impl ICommand for Command {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        (self.can_execute)()
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if (self.can_execute)() {
            (self.execute)(parameter)
        }
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.can_execute_changed.borrow_mut().push((token, handler));
        let weak = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.can_execute_changed
                    .borrow_mut()
                    .retain(|(t, _)| *t != token);
            }
        })
    }
}
