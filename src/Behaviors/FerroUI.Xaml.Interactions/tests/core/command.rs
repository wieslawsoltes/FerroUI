//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/Command.cs` of Xaml.Behaviors.

use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// A command of the tests: runs `execute`, and can execute when `can_execute` says so (always
/// without it). It never raises its can-execute change.
pub(crate) struct Command {
    execute: Box<dyn Fn(Option<&BoxedValue>)>,
    can_execute: Option<Box<dyn Fn(Option<&BoxedValue>) -> bool>>,
}

impl Command {
    /// `new Command(execute)`.
    pub(crate) fn new(execute: impl Fn(Option<&BoxedValue>) + 'static) -> Rc<dyn ICommand> {
        Rc::new(Self { execute: Box::new(execute), can_execute: None })
    }

    /// `new Command(execute, canExecute)`.
    #[allow(dead_code)]
    pub(crate) fn with_can_execute(
        execute: impl Fn(Option<&BoxedValue>) + 'static,
        can_execute: impl Fn(Option<&BoxedValue>) -> bool + 'static,
    ) -> Rc<dyn ICommand> {
        Rc::new(Self { execute: Box::new(execute), can_execute: Some(Box::new(can_execute)) })
    }
}

impl ICommand for Command {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        self.can_execute.as_ref().is_none_or(|can_execute| can_execute(parameter))
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        (self.execute)(parameter);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
