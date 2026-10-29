//! Port of `Contract/IReversibleAction.cs`.

use super::{IAction, InterfaceTable};
use ferroui_base::{BoxedValue, FerroObject, ObjectType};

/// Interface for actions that can revert the effect of a previous
/// [`IAction::execute`] call.
pub trait IReversibleAction: IAction {
    /// Executes the action while capturing the state required by
    /// [`revert`](Self::revert).
    ///
    /// Returns the result of the reversible execution.
    fn execute_reversibly(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue>;

    /// Reverts the previously applied action.
    ///
    /// Returns the result of the revert operation.
    fn revert(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue>;
}

static TABLE: InterfaceTable<dyn IReversibleAction> = InterfaceTable::new();

fn cast<'a, T: ObjectType + IReversibleAction>(object: &'a FerroObject) -> Option<&'a (dyn IReversibleAction + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn IReversibleAction + 'static))
}

impl dyn IReversibleAction {
    /// States that the objects of class `T` (and of the classes deriving from it) are reversible
    /// actions. Called from the static constructor of the class, which also registers the class
    /// as an [`IAction`].
    pub fn register<T: ObjectType + IReversibleAction>() {
        TABLE.register(T::TYPE, cast::<T>);
    }

    /// The object as a reversible action (`object as IReversibleAction`), if its class is one.
    pub fn of(object: &FerroObject) -> Option<&dyn IReversibleAction> {
        TABLE.find(object)
    }
}
