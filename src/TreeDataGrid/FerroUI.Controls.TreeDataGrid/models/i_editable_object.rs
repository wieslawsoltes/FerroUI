//! `System.ComponentModel.IEditableObject`: an object that commits or rolls back the changes of
//! an edit. The cells implement it (a text cell edits its text, a template cell forwards the edit
//! to its model), and a model type states it once with [`register_editable_model`], as the
//! framework's model types state their notification interfaces.
//!
//! Not a file of the reference: the runtime's `as IEditableObject` cast on an arbitrary model
//! becomes a lookup of the model's type.

use ferroui_base::AnyValue;
use std::any::TypeId;
use std::cell::RefCell;
use rustc_hash::FxHashMap;

/// An object that commits or discards the changes of an edit (`IEditableObject`).
pub trait IEditableObject {
    /// Begins an edit.
    fn begin_edit(&self);
    /// Discards the changes since the last `begin_edit`.
    fn cancel_edit(&self);
    /// Commits the changes since the last `begin_edit`.
    fn end_edit(&self);
}

type AsEditable = fn(&dyn AnyValue) -> Option<&dyn IEditableObject>;

ferroui_base::ferro_thread_local! {
    static EDITABLE_MODELS: RefCell<FxHashMap<TypeId, AsEditable>> = RefCell::new(FxHashMap::default());
}

fn as_editable_of<T: IEditableObject + 'static>(value: &dyn AnyValue) -> Option<&dyn IEditableObject> {
    value.downcast_ref::<T>().map(|typed| typed as &dyn IEditableObject)
}

/// States that the models of type `T` are editable objects, so that a template cell forwards its
/// edits to them.
pub fn register_editable_model<T: IEditableObject + PartialEq + 'static>() {
    EDITABLE_MODELS.with(|models| {
        models.borrow_mut().insert(TypeId::of::<T>(), as_editable_of::<T> as AsEditable);
    });
}

/// The value as an editable object (`value as IEditableObject`), if its type was registered.
pub fn as_editable_object(value: &dyn AnyValue) -> Option<&dyn IEditableObject> {
    let id = value.value_type_id();
    let cast = EDITABLE_MODELS.try_with(|models| models.borrow().get(&id).copied()).ok().flatten()?;
    cast(value)
}
