//! Not from the reference: the untyped values the behaviors pass to handlers, actions and
//! commands (the `object` arguments of the reference).
//!
//! An element is passed as a handle of the root class, event args as a shared handle to the live
//! args (as the event triggers of the behaviors pass them), so that the receiver views them with
//! [`object_of`] and [`drag_event_args_of`].

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::DragEventArgs;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Ref, Upcast};
use std::rc::Rc;

/// An element as an untyped value.
pub fn boxed_object<T: ObjectType + Upcast<FerroObject>>(object: &Ref<T>) -> BoxedValue {
    Rc::new(object.clone().upcast::<FerroObject>())
}

/// The object an untyped value holds (`value as SomeClass` of the reference, before the cast to
/// the class).
pub fn object_of(value: Option<&BoxedValue>) -> Option<Ref<FerroObject>> {
    value.and_then(|value| ValueTypes::as_object(&**value))
}

/// Event args as an untyped value: a shared handle to the live args.
pub fn boxed_event_args(e: &dyn IRoutedEventArgs) -> BoxedValue {
    Rc::new(e.share())
}

/// The drag event args an untyped value holds (`parameter is DragEventArgs e`).
pub fn drag_event_args_of(value: Option<&BoxedValue>) -> Option<DragEventArgs> {
    let value = value?;
    if let Some(args) = value.downcast_ref::<Rc<dyn IRoutedEventArgs>>() {
        return args.downcast_ref::<DragEventArgs>().cloned();
    }
    value.downcast_ref::<DragEventArgs>().cloned()
}

/// A storage item as an untyped value or an item of a list: a handle that compares by identity.
///
/// Deviation (DEVIATIONS.md, Behaviors): a storage item of the framework has no equality, which an
/// untyped value and an items source need; the files the behaviors add to a list are held by this
/// handle.
#[derive(Clone)]
pub struct StorageItemValue(pub Rc<dyn IStorageItem>);

impl PartialEq for StorageItemValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl std::ops::Deref for StorageItemValue {
    type Target = dyn IStorageItem;

    fn deref(&self) -> &(dyn IStorageItem + 'static) {
        &*self.0
    }
}
