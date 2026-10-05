//! The registry of the classes that implement `ICommandSource` (declared in
//! the base crate's input module).
//!
//! A class cannot implement the trait itself (its methods would shadow the
//! inherent ones of the class): it provides a small adapter handle that
//! does, and makes it known with [`register_command_source`], normally from
//! its class initialization. [`as_command_source`] then views an object of
//! that class (or of a class derived from it) as a command source.

use ferroui_base::input::ICommandSource;
use ferroui_base::{FerroObject, ObjectType, Ref, TypeInfo};
use std::cell::RefCell;
use std::rc::Rc;

type Adapter = Rc<dyn Fn(&FerroObject) -> Option<Rc<dyn ICommandSource>>>;

thread_local! {
    static SOURCE_TYPES: RefCell<Vec<(&'static TypeInfo, Adapter)>> = const { RefCell::new(Vec::new()) };
}

/// Declares that class `T` (and the classes derived from it) implements
/// [`ICommandSource`] through the handle created by `adapter`.
pub fn register_command_source<T: ObjectType>(adapter: fn(Ref<T>) -> Rc<dyn ICommandSource>) {
    SOURCE_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if !types.iter().any(|(type_, _)| *type_ == T::TYPE) {
            let adapter: Adapter =
                Rc::new(move |object| object.downcast_ref::<T>().map(|target| adapter(FerroObject::ref_of(target))));
            types.push((T::TYPE, adapter));
        }
    });
}

/// The object viewed as a command source, if its class implements
/// [`ICommandSource`].
pub fn as_command_source(object: &FerroObject) -> Option<Rc<dyn ICommandSource>> {
    let adapter = SOURCE_TYPES.with(|types| {
        let types = types.borrow();
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            if let Some((_, adapter)) = types.iter().find(|(candidate, _)| *candidate == type_) {
                return Some(adapter.clone());
            }
            current = type_.base_type();
        }
        None
    });
    adapter.and_then(|adapter| adapter(object))
}
