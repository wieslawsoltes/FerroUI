//! Port of `Contract/IAction.cs`.

use super::InterfaceTable;
use ferroui_base::{BoxedValue, FerroObject, ObjectType};

/// Interface implemented by all custom actions.
pub trait IAction {
    /// Executes the action.
    ///
    /// `sender` is the object that is passed to the action by the behavior. Generally this is
    /// [`IBehavior::associated_object`](super::IBehavior::associated_object) or a target object.
    /// The value of `parameter` is determined by the caller. An example of parameter usage is
    /// EventTriggerBehavior, which passes the event args as a parameter to its actions.
    ///
    /// Returns the result of the action.
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue>;
}

static TABLE: InterfaceTable<dyn IAction> = InterfaceTable::new();

fn cast<'a, T: ObjectType + IAction>(object: &'a FerroObject) -> Option<&'a (dyn IAction + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn IAction + 'static))
}

impl dyn IAction {
    /// States that the objects of class `T` (and of the classes deriving from it) are actions.
    /// Called from the static constructor of the class.
    pub fn register<T: ObjectType + IAction>() {
        TABLE.register(T::TYPE, cast::<T>);
    }

    /// The object as an action (`object as IAction`), if its class is one.
    pub fn of(object: &FerroObject) -> Option<&dyn IAction> {
        TABLE.find(object)
    }
}
