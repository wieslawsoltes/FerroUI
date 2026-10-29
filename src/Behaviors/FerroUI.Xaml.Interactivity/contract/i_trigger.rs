//! Port of `Contract/ITrigger.cs`.

use super::{IBehavior, InterfaceTable};
use crate::ActionCollection;
use ferroui_base::{FerroObject, ObjectType};

/// Interface implemented by all custom triggers.
pub trait ITrigger: IBehavior {
    /// Gets the collection of actions associated with the behavior.
    fn actions(&self) -> Option<ActionCollection>;
}

static TABLE: InterfaceTable<dyn ITrigger> = InterfaceTable::new();

fn cast<'a, T: ObjectType + ITrigger>(object: &'a FerroObject) -> Option<&'a (dyn ITrigger + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn ITrigger + 'static))
}

impl dyn ITrigger {
    /// States that the objects of class `T` (and of the classes deriving from it) are triggers.
    /// Called from the static constructor of the class, which also registers the class as an
    /// [`IBehavior`].
    pub fn register<T: ObjectType + ITrigger>() {
        TABLE.register(T::TYPE, cast::<T>);
    }

    /// The object as a trigger (`object as ITrigger`), if its class is one.
    pub fn of(object: &FerroObject) -> Option<&dyn ITrigger> {
        TABLE.find(object)
    }
}
