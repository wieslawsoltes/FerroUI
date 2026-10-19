//! Property paths through the properties of FerroUI objects.
//!
//! Not an upstream file. Upstream writes `x => x.NameBox.Text` and observes the step `Text` of the
//! `TextBox` through the property system, because the observer of `FerroObject`s has the
//! highest affinity for it. A FerroUI object is held as a handle `Ref<T>`, not as an `Rc` of a
//! type that raises `PropertyChanged`, so its steps are written with [`ferro_property`] (or the
//! macro [`ferro_prop!`](crate::ferro_prop)) and joined to the path of a view with
//! [`PropertyPath::then`]:
//!
//! ```ignore
//! let text = prop!(MyView => name_box).then(ferro_prop!(TextBox => text));
//! ```
//!
//! The step gives the object as its owner ([`ferro_property_owner`]), so that
//! [`FerroObjectObservableForProperty`](crate::FerroObjectObservableForProperty) observes it, and
//! the path observes the registered property of that name itself.

use crate::FerroObjectHandles;
use ferroui_base::{FerroObject, FerroPropertyChangedEventArgs, FerroPropertyRegistry, ObjectType, Ref, Upcast};
use ferroui_system::component_model::{PropertyOwner, PropertyPath};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The owner of the properties of `object`, as a step of a path gives it to the property observers
/// chosen at run time: the object as its untyped handle, without a `PropertyChanged` notification.
pub fn ferro_property_owner<T: ObjectType>(object: &Ref<T>) -> PropertyOwner {
    PropertyOwner { object: Some(FerroObjectHandles::handle_of(object)), notify: None }
}

/// A path of the property `name` of a FerroUI object of class `T`, read with `get`: its step gives
/// the object as its owner ([`ferro_property_owner`]), and the path observes the change of the
/// registered property of that name.
pub fn ferro_property<T: ObjectType, V: 'static>(name: impl Into<String>, get: impl Fn(&Ref<T>) -> V + 'static) -> PropertyPath<Ref<T>, V> {
    let name: String = name.into();
    let watched = name.clone();
    PropertyPath::owned_property(name, get, ferro_property_owner::<T>, move |source: &Ref<T>, changed: Rc<dyn Fn()>| {
        observe_ferro_property(Upcast::<FerroObject>::upcast(&**source), &watched, changed)
    })
}

/// Runs `changed` each time the registered property `name` of `object` changes, until the result
/// is disposed; a property the class does not register is never observed.
pub fn observe_ferro_property(object: &FerroObject, name: &str, changed: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
    let property = if name.contains('.') { None } else { FerroPropertyRegistry::instance().find_registered_for(object, name) };
    match property {
        Some(property) => object.property_changed(move |args: &FerroPropertyChangedEventArgs<'_>| {
            if args.property() == property {
                changed();
            }
        }),
        None => ferroui_reactive::disposables::Disposable::empty(),
    }
}

/// A [`PropertyPath`] of one property of a FerroUI object (`x => x.Text` on a control).
///
/// `ferro_prop!(TextBox => text)` is the path of the getter `text` of a `Ref<TextBox>`; its
/// property name is the getter's in PascalCase, or the one given with `as "Name"`.
#[macro_export]
macro_rules! ferro_prop {
    ($type_:ty => $getter:ident $(as $name:literal)?) => {
        $crate::ferro_property::<$type_, _>(
            $crate::ferro_prop!(@name $getter $($name)?),
            |source: &$crate::__private::Ref<$type_>| source.$getter(),
        )
    };
    (@name $getter:ident $name:literal) => { $name };
    (@name $getter:ident) => { $crate::__private::property_name_of(::std::stringify!($getter)) };
}
