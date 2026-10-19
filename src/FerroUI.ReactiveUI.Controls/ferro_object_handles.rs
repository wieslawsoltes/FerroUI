//! The FerroUI object behind an untyped object of ReactiveUI.
//!
//! Not an upstream file. ReactiveUI hands the integration its objects untyped: the owner of a
//! property of a path (`PropertyOwner::object`), the control of a command binding and the target
//! of a binding's write are `Rc<dyn Any>`, and a view is a `dyn IActivatableView`. Upstream tests
//! them with `is FerroObject`; here [`FerroObjectHandles::object_of`] finds the object behind
//! one of the forms a FerroUI object takes in an untyped value:
//!
//! - a handle `Ref<T>` of any class (`Ref<FerroObject>`, `Ref<Button>`, ...), the form the paths of
//!   [`ferro_property`](crate::ferro_property) and the command bindings of controls give;
//! - a [`BoxedValue`] holding a handle, the form of the untyped values of the property system;
//! - a type registered with [`FerroObjectHandles::register`] (a view that wraps its control) or
//!   [`FerroObjectHandles::register_class`] (the class itself).

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Ref, TypeInfo, Upcast};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Finds the object behind a registered handle type.
type ObjectOf = Rc<dyn Fn(&dyn Any) -> Option<Ref<FerroObject>>>;

ferroui_base::ferro_thread_local! {
    /// The handle types registered on the thread, by their `TypeId`.
    static HANDLES: RefCell<HashMap<TypeId, ObjectOf>> = RefCell::new(HashMap::new());
}

/// The registry of the types that stand for a FerroUI object in the untyped values of ReactiveUI,
/// and the lookup of the object behind such a value.
///
/// Deviation (DEVIATIONS.md, ReactiveUI controls): upstream tests an untyped object with
/// `is FerroObject`; Rust cannot ask a `dyn Any` whether it is a class of the object model, so
/// the handles of the classes are recognised through the type model of the framework and other
/// types are registered.
pub struct FerroObjectHandles;

impl FerroObjectHandles {
    /// Registers `H` as a type that stands for a FerroUI object: `object_of` gives the object
    /// behind a value of it. A view that wraps its control (`struct MyView(Ref<UserControl>)`)
    /// registers itself so that the activation fetcher and the property observer see the control.
    pub fn register<H: 'static>(object_of: fn(&H) -> Ref<FerroObject>) {
        let entry: ObjectOf = Rc::new(move |value: &dyn Any| value.downcast_ref::<H>().map(object_of));
        HANDLES.with(|handles| {
            handles.borrow_mut().insert(TypeId::of::<H>(), entry);
        });
    }

    /// Registers the class `T` itself (its struct, as an `Rc<T>` that is the allocation of the
    /// object) as a type that stands for its object: a class that implements a trait of ReactiveUI
    /// on its struct (`IActivatableView`) is seen through it.
    pub fn register_class<T: ObjectType>() {
        fn object_of_class<T: ObjectType>(value: &dyn Any) -> Option<Ref<FerroObject>> {
            value.downcast_ref::<T>().map(|object| FerroObject::to_ref(Upcast::<FerroObject>::upcast(object)))
        }
        let entry: ObjectOf = Rc::new(object_of_class::<T>);
        HANDLES.with(|handles| {
            handles.borrow_mut().insert(TypeId::of::<T>(), entry);
        });
    }

    /// The FerroUI object `value` stands for, or `None` when it is not one (`value as FerroObject`).
    pub fn object_of(value: &dyn Any) -> Option<Ref<FerroObject>> {
        if let Some(object) = value.downcast_ref::<Ref<FerroObject>>() {
            return Some(object.clone());
        }
        if let Some(boxed) = value.downcast_ref::<BoxedValue>() {
            return ValueTypes::as_object(&**boxed);
        }
        let type_id = <dyn Any>::type_id(value);
        if let Some(object) = Self::handle_object(value, type_id) {
            return Some(object);
        }
        let registered = HANDLES.with(|handles| handles.borrow().get(&type_id).cloned());
        registered.and_then(|object_of| object_of(value))
    }

    /// The untyped form of `object` that the integration gives ReactiveUI: its handle as a
    /// `Ref<FerroObject>` in an `Rc<dyn Any>`.
    pub fn handle_of<T: ObjectType>(object: &Ref<T>) -> Rc<dyn Any> {
        let object: &FerroObject = Upcast::<FerroObject>::upcast(&**object);
        Rc::new(object.to_ref())
    }

    /// The object behind a handle `Ref<T>` of a class of the type model: the class is found by the
    /// type of the handle, and the handle is read through the untyped conversions the class
    /// registers when it is initialised (its nullable form, then the object of that form).
    fn handle_object(value: &dyn Any, type_id: TypeId) -> Option<Ref<FerroObject>> {
        let (class, nullable) = TypeInfo::find_by_handle(type_id)?;
        if nullable {
            return None;
        }
        let nullable_handle = class.nullable_handle()?;
        let boxed = ValueTypes::try_convert_for_property(value, nullable_handle)?;
        let handle = ValueTypes::normalize(boxed)?;
        ValueTypes::as_object(&*handle)
    }
}
