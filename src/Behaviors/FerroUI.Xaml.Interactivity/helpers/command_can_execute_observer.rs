//! Port of `Helpers/CommandCanExecuteObserver.cs`.

use ferroui_base::input::ICommand;
use ferroui_base::reactive::IDisposable;
use ferroui_base::BoxedValue;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// Observes an [`ICommand`] and reports whether it can execute with a specified parameter.
///
/// Use this helper from custom command-backed behaviors to expose a bindable command state, such
/// as a direct property. Call [`start`](Self::start) when the behavior is attached,
/// [`update`](Self::update) when the command or command parameter changes, and
/// [`stop`](Self::stop) when the behavior is detached.
///
/// The command event subscription uses a weak reference to this observer so a long-lived command
/// does not keep the observer or owning behavior alive if cleanup is missed.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference posts the callback to the user-interface
/// thread when a command raises its notification on another thread. Commands and observers of
/// the port belong to the thread that created them (they are not `Send`), so the callback always
/// runs on that thread.
pub struct CommandCanExecuteObserver {
    pub(crate) inner: Rc<ObserverState>,
}

pub(crate) struct ObserverState {
    set_can_execute: Box<dyn Fn(bool)>,
    weak_can_execute_changed_handler: Rc<WeakCanExecuteChangedHandler>,
    command: RefCell<Option<Rc<dyn ICommand>>>,
    parameter: RefCell<Option<BoxedValue>>,
    is_parameter_known: Cell<bool>,
    is_observing: Cell<bool>,
}

impl CommandCanExecuteObserver {
    /// Initializes a new instance of the [`CommandCanExecuteObserver`] class; `set_can_execute`
    /// receives the current command execution state.
    pub fn new(set_can_execute: impl Fn(bool) + 'static) -> Self {
        let inner = Rc::new_cyclic(|this: &Weak<ObserverState>| ObserverState {
            set_can_execute: Box::new(set_can_execute),
            weak_can_execute_changed_handler: Rc::new(WeakCanExecuteChangedHandler::new(this.clone())),
            command: RefCell::new(None),
            parameter: RefCell::new(None),
            is_parameter_known: Cell::new(true),
            is_observing: Cell::new(false),
        });
        Self { inner }
    }

    /// Starts observing the specified command and reports its initial execution state; a null
    /// command reports that execution is allowed.
    pub fn start(&self, command: Option<Rc<dyn ICommand>>, parameter: Option<BoxedValue>) {
        self.start_with(command, parameter, true);
    }

    /// Starts observing the specified command and reports its initial execution state.
    ///
    /// `is_parameter_known` is true when `parameter` is the actual parameter to evaluate, false
    /// when the parameter will only be available at execution time: the observer then reports
    /// true without calling `can_execute`, which avoids probing commands with placeholder values
    /// such as null when event arguments or picker results will provide the real parameter later.
    pub fn start_with(&self, command: Option<Rc<dyn ICommand>>, parameter: Option<BoxedValue>, is_parameter_known: bool) {
        let state = &self.inner;
        if state.is_observing.get() {
            self.update_with(command, parameter, is_parameter_known);
            return;
        }

        state.is_observing.set(true);
        *state.command.borrow_mut() = command.clone();
        *state.parameter.borrow_mut() = parameter;
        state.is_parameter_known.set(is_parameter_known);

        if let Some(command) = command {
            state.weak_can_execute_changed_handler.subscribe(&command);
        }

        state.update_can_execute();
    }

    /// Updates the observed command and command parameter. Has no effect until
    /// [`start`](Self::start) has been called.
    pub fn update(&self, command: Option<Rc<dyn ICommand>>, parameter: Option<BoxedValue>) {
        self.update_with(command, parameter, true);
    }

    /// Updates the observed command and command parameter; see [`start_with`](Self::start_with)
    /// for `is_parameter_known`. Has no effect until [`start`](Self::start) has been called.
    pub fn update_with(&self, command: Option<Rc<dyn ICommand>>, parameter: Option<BoxedValue>, is_parameter_known: bool) {
        let state = &self.inner;
        if !state.is_observing.get() {
            return;
        }

        let same_command = match (&*state.command.borrow(), &command) {
            (Some(current), Some(command)) => Rc::ptr_eq(current, command),
            (None, None) => true,
            _ => false,
        };
        if !same_command {
            state.weak_can_execute_changed_handler.unsubscribe();

            *state.command.borrow_mut() = command.clone();

            if let Some(command) = command {
                state.weak_can_execute_changed_handler.subscribe(&command);
            }
        }

        *state.parameter.borrow_mut() = parameter;
        state.is_parameter_known.set(is_parameter_known);
        state.update_can_execute();
    }

    /// Stops observing the current command and clears the stored command parameter.
    pub fn stop(&self) {
        let state = &self.inner;
        if !state.is_observing.get() {
            return;
        }

        state.weak_can_execute_changed_handler.unsubscribe();

        *state.command.borrow_mut() = None;
        *state.parameter.borrow_mut() = None;
        state.is_parameter_known.set(true);
        state.is_observing.set(false);
    }
}

impl IDisposable for CommandCanExecuteObserver {
    /// Stops observing the current command.
    fn dispose(&self) {
        self.stop();
    }
}

impl ObserverState {
    fn update_can_execute(&self) {
        // The reference posts to the user-interface thread when called from another thread; see
        // the deviation on the type.
        self.update_can_execute_core();
    }

    fn update_can_execute_core(&self) {
        if !self.is_observing.get() {
            return;
        }

        let command = self.command.borrow().clone();
        let command = match command {
            Some(command) if self.is_parameter_known.get() => command,
            _ => {
                (self.set_can_execute)(true);
                return;
            }
        };

        let parameter = self.parameter.borrow().clone();
        (self.set_can_execute)(command.can_execute(parameter.as_ref()));
    }
}

/// The handler of the notification of a command: it refers to its observer weakly, and removes
/// itself from the command when the observer is gone.
struct WeakCanExecuteChangedHandler {
    observer: Weak<ObserverState>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl WeakCanExecuteChangedHandler {
    fn new(observer: Weak<ObserverState>) -> Self {
        Self { observer, subscription: RefCell::new(None) }
    }

    /// `command.CanExecuteChanged += handler` (and `SetCommand(command)`).
    fn subscribe(self: &Rc<Self>, command: &Rc<dyn ICommand>) {
        // The command holds the handler (the target of its delegate in the reference), and the
        // handler holds the observer weakly.
        let handler = self.clone();
        let subscription = command.can_execute_changed(Rc::new(move || handler.on_can_execute_changed()));
        let previous = self.subscription.borrow_mut().replace(subscription);
        if let Some(previous) = previous {
            previous.dispose();
        }
    }

    /// `command.CanExecuteChanged -= handler` (and `SetCommand(null)`).
    fn unsubscribe(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn on_can_execute_changed(&self) {
        if let Some(observer) = self.observer.upgrade() {
            observer.update_can_execute();
            return;
        }

        self.unsubscribe();
    }
}
