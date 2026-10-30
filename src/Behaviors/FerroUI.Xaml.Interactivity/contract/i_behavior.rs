//! Port of `Contract/IBehavior.cs`.

use super::InterfaceTable;
use ferroui_base::{FerroObject, ObjectType, Ref};

/// Interface implemented by all custom behaviors.
pub trait IBehavior {
    /// Gets the object to which the behavior is attached.
    fn associated_object(&self) -> Option<Ref<FerroObject>>;

    /// Attaches to the specified object.
    fn attach(&self, associated_object: Option<Ref<FerroObject>>);

    /// Detaches this instance from its associated object.
    fn detach(&self);
}

static TABLE: InterfaceTable<dyn IBehavior> = InterfaceTable::new();

fn cast<'a, T: ObjectType + IBehavior>(object: &'a FerroObject) -> Option<&'a (dyn IBehavior + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn IBehavior + 'static))
}

impl dyn IBehavior {
    /// States that the objects of class `T` (and of the classes deriving from it) are behaviors.
    /// Called from the static constructor of the class.
    pub fn register<T: ObjectType + IBehavior>() {
        TABLE.register(T::TYPE, cast::<T>);
    }

    /// The object as a behavior (`object as IBehavior`), if its class is one.
    pub fn of(object: &FerroObject) -> Option<&dyn IBehavior> {
        TABLE.find(object)
    }
}
