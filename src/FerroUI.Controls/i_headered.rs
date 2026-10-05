use ferroui_base::{BoxedValue, FerroObject, ObjectType, TypeInfo};
use std::cell::RefCell;

/// Defines an object that has a header.
///
/// A class implementing the trait is made known with [`register_headered`],
/// normally from its class initialization; [`as_headered`] then views an
/// object of the class (or of a class derived from it) as the interface.
pub trait IHeadered {
    /// The header content.
    fn header(&self) -> Option<BoxedValue>;

    /// Sets the header content.
    fn set_header(&self, value: Option<BoxedValue>);
}

type HeaderedCast = for<'a> fn(&'a FerroObject) -> Option<&'a dyn IHeadered>;

thread_local! {
    static HEADERED_TYPES: RefCell<Vec<(&'static TypeInfo, HeaderedCast)>> = const { RefCell::new(Vec::new()) };
}

fn cast_headered<T: ObjectType + IHeadered>(object: &FerroObject) -> Option<&dyn IHeadered> {
    object.downcast_ref::<T>().map(|headered| headered as &dyn IHeadered)
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`IHeadered`].
pub fn register_headered<T: ObjectType + IHeadered>() {
    HEADERED_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            types.push((T::TYPE, cast_headered::<T> as HeaderedCast));
        }
    });
}

/// The object viewed as a headered object, if its class implements
/// [`IHeadered`].
pub fn as_headered(object: &FerroObject) -> Option<&dyn IHeadered> {
    let cast = HEADERED_TYPES.with(|types| {
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
