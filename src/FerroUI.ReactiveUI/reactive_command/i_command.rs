//! `ICommand` (`System.Windows.Input.ICommand` of the base class library): a command that a view
//! executes, with a parameter of any type.
//!
//! The crate does not depend on the user interface, so this is its own trait. The `ICommand` of
//! `ferroui-base` takes the UI's `BoxedValue`; the integration crate bridges the two.

use ferroui_system::IDisposable;
use std::any::Any;
use std::rc::Rc;

/// A command, executed with a parameter of any type (`System.Windows.Input.ICommand`).
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the parameter `object?` is `Option<&dyn Any>`, and the
/// event `CanExecuteChanged` is [`can_execute_changed`](Self::can_execute_changed), which adds a
/// handler and returns the disposable that removes it.
pub trait ICommand {
    /// Whether the command can execute with `parameter` now (`CanExecute(object?)`).
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool;

    /// Executes the command with `parameter` (`Execute(object?)`).
    fn execute(&self, parameter: Option<&dyn Any>);

    /// Adds `handler`, which runs when what [`can_execute`](Self::can_execute) answers may have
    /// changed (`CanExecuteChanged`); disposing the result removes it.
    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable>;
}

/// Two commands are equal when they are the same object, as the default equality of .NET
/// compares references (`EqualityComparer<ICommand>.Default`), so that a property holding a
/// command can be set through `raise_and_set_if_changed`.
impl PartialEq for dyn ICommand {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

/// A shared command is a command.
impl<C: ICommand + ?Sized> ICommand for Rc<C> {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        (**self).can_execute(parameter)
    }

    fn execute(&self, parameter: Option<&dyn Any>) {
        (**self).execute(parameter)
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        (**self).can_execute_changed(handler)
    }
}
