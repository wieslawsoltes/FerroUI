use ferroui_base::{FerroObject, ObjectType, TypeInfo};
use std::cell::RefCell;

/// Interface for objects that are selectable.
///
/// Controls such as the selecting items control check for this interface on
/// items that they display, to determine whether the item is selected.
///
/// A class implementing the trait is made known with
/// [`register_selectable`], normally from its class initialization;
/// [`as_selectable`] then views an object of the class (or of a class
/// derived from it) as the interface.
pub trait ISelectable {
    /// Gets whether the object is currently selected.
    fn is_selected(&self) -> bool;

    /// Sets whether the object is currently selected.
    fn set_is_selected(&self, value: bool);
}

type SelectableCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn ISelectable>;

thread_local! {
    static SELECTABLE_TYPES: RefCell<Vec<(&'static TypeInfo, SelectableCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_selectable<T: ObjectType + ISelectable>(object: &FerroObject) -> Option<&dyn ISelectable> {
    object.downcast_ref::<T>().map(|selectable| selectable as &dyn ISelectable)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`ISelectable`].
pub fn register_selectable<T: ObjectType + ISelectable>() {
    SELECTABLE_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_selectable::<T> as SelectableCast));
        }
    });
}

/// The object viewed as a selectable, if its class implements
/// [`ISelectable`].
pub fn as_selectable(object: &FerroObject) -> Option<&dyn ISelectable> {
    let cast = SELECTABLE_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, cast)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(*cast);
            }
            current = type_.base_type();
        }
        None
    });
    cast.and_then(|cast| cast(object))
}
