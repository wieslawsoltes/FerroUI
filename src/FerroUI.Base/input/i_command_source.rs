use super::ICommand;
use crate::BoxedValue;
use std::rc::Rc;

/// An interface for classes that know how to invoke a command.
pub trait ICommandSource {
    /// The command that will be executed when the class is "invoked."
    /// Classes that implement this interface should enable or disable based
    /// on what the command's `can_execute` returns. The property may be
    /// implemented as read-write if desired.
    fn command(&self) -> Option<Rc<dyn ICommand>>;

    /// The parameter that will be passed to the command when executing the
    /// command. The property may be implemented as read-write if desired.
    fn command_parameter(&self) -> Option<BoxedValue>;

    /// Called for the `can_execute_changed` event of the command when
    /// changes are detected.
    fn can_execute_changed(&self);

    /// Gets a value indicating whether this control and all its parents are
    /// enabled.
    fn is_effectively_enabled(&self) -> bool;
}
