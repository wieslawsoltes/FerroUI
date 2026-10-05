use crate::reactive::IDisposable;
use crate::BoxedValue;
use std::rc::Rc;

/// Defines a command.
///
/// Commands compare by reference.
pub trait ICommand {
    /// Defines the method that determines whether the command can execute
    /// in its current state. `parameter` is data used by the command; if
    /// the command does not require data, it can be `None`.
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool;

    /// Defines the method to be called when the command is invoked.
    fn execute(&self, parameter: Option<&BoxedValue>);

    /// Occurs when changes occur that affect whether or not the command
    /// should execute. Disposing the returned handle unsubscribes.
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

impl PartialEq for dyn ICommand {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const dyn ICommand, other as *const dyn ICommand)
    }
}
