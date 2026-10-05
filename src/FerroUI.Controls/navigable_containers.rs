//! Views a panel as the navigable container interface it implements.
//!
//! A class implementing [`INavigableContainer`] is made known with
//! [`register_navigable_container`], normally from its class
//! initialization; [`as_navigable_container`] then views an object of the
//! class (or of a class derived from it) as the interface. The panels of
//! this crate are registered when the first items control is created.

use ferroui_base::input::INavigableContainer;
use ferroui_base::{FerroObject, ObjectType, TypeInfo};
use std::cell::RefCell;

type ContainerCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn INavigableContainer>;

thread_local! {
    static CONTAINER_TYPES: RefCell<Vec<(&'static TypeInfo, ContainerCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_container<T: ObjectType + INavigableContainer>(object: &FerroObject) -> Option<&dyn INavigableContainer> {
    object.downcast_ref::<T>().map(|container| container as &dyn INavigableContainer)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`INavigableContainer`].
pub fn register_navigable_container<T: ObjectType + INavigableContainer>() {
    CONTAINER_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_container::<T> as ContainerCast));
        }
    });
}

/// The object viewed as a navigable container, if its class implements
/// [`INavigableContainer`].
pub fn as_navigable_container(object: &FerroObject) -> Option<&dyn INavigableContainer> {
    let cast = CONTAINER_TYPES.with(|types| {
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
