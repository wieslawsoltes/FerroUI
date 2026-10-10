//! Port of `DelegateCommand.cs`.

use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// A command that runs an action; whether it can execute is asked of a function of the
/// parameter, which answers yes when none is given.
pub struct DelegateCommand {
    action: Box<dyn Fn()>,
    can_execute: Box<dyn Fn(Option<&BoxedValue>) -> bool>,
}

impl DelegateCommand {
    /// `new DelegateCommand(action)`.
    pub fn new(action: impl Fn() + 'static) -> Rc<DelegateCommand> {
        Self::with_can_execute(action, |_| true)
    }

    /// `new DelegateCommand(action, canExecute)`.
    pub fn with_can_execute(
        action: impl Fn() + 'static,
        can_execute: impl Fn(Option<&BoxedValue>) -> bool + 'static,
    ) -> Rc<DelegateCommand> {
        Rc::new(Self { action: Box::new(action), can_execute: Box::new(can_execute) })
    }

    /// The command as the contract controls take.
    pub fn as_command(self: &Rc<Self>) -> Rc<dyn ICommand> {
        self.clone()
    }
}

impl ICommand for DelegateCommand {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        (self.can_execute)(parameter)
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        (self.action)();
    }

    /// The event of the managed original keeps no handler (`add { } remove { }`): the command
    /// never says that what it can execute changed.
    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;
    use std::cell::Cell;

    #[test]
    fn the_command_runs_its_action_and_can_execute_by_default() {
        let runs = Rc::new(Cell::new(0));
        let command = DelegateCommand::new({
            let runs = runs.clone();
            move || runs.set(runs.get() + 1)
        });
        assert!(command.can_execute(None));
        command.execute(None);
        assert_eq!(1, runs.get());
    }

    #[test]
    fn the_command_asks_its_function_whether_it_can_execute() {
        let command = DelegateCommand::with_can_execute(|| {}, |parameter| parameter.is_some());
        assert!(!command.can_execute(None));
        let parameter: BoxedValue = Rc::new(1);
        assert!(command.can_execute(Some(&parameter)));
    }
}
