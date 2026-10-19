//! The two forms of a view model: the untyped value of the property system (`BoxedValue`, what a
//! `DataContext` and a `ViewModel` property hold) and the untyped view model of ReactiveUI
//! (`Rc<dyn Any>`, what `IViewFor::view_model` gives and takes).
//!
//! Not an upstream file: upstream's `object` is both.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI controls): a `BoxedValue` is an `Rc<dyn AnyValue>`, and
//! `AnyValue` needs `PartialEq`; an `Rc<dyn Any>` cannot be turned into one without its type. The
//! types of view models are therefore registered per thread ([`register_view_model`]; the typed
//! views and [`ControlViewFor`](crate::ControlViewFor) register theirs), and a view model of a type
//! that was never registered is boxed in an [`UntypedViewModel`], which compares by identity and is
//! unwrapped again on the way back. Bindings of markup see the view model itself only for
//! registered types.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::BoxedValue;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// Boxes a view model of a registered type as the value of a property.
type BoxViewModel = fn(Rc<dyn Any>) -> Option<BoxedValue>;

ferroui_base::ferro_thread_local! {
    /// The registered view model types, by type.
    static VIEW_MODEL_TYPES: RefCell<HashMap<TypeId, BoxViewModel>> = RefCell::new(HashMap::new());
}

fn box_view_model<VM: PartialEq + 'static>(view_model: Rc<dyn Any>) -> Option<BoxedValue> {
    view_model.downcast::<VM>().ok().map(|view_model| view_model as BoxedValue)
}

/// Registers the view model type `VM` on this thread: a view model of the type is then the value
/// of a `DataContext` itself (`Rc<VM>` as the box), and a reference type of the untyped value
/// conversions (`ValueTypes::register_reference`). Idempotent.
pub fn register_view_model<VM: PartialEq + 'static>() {
    let added = VIEW_MODEL_TYPES.with(|types| {
        let mut types = types.borrow_mut();
        if types.contains_key(&TypeId::of::<VM>()) {
            return false;
        }
        types.insert(TypeId::of::<VM>(), box_view_model::<VM>);
        true
    });
    if added {
        ValueTypes::register_reference::<VM>();
    }
}

/// A view model of a type that was not registered, as the value of a property: the view model
/// itself, compared by identity.
#[derive(Clone)]
pub struct UntypedViewModel(pub Rc<dyn Any>);

impl PartialEq for UntypedViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }
}

impl fmt::Debug for UntypedViewModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("UntypedViewModel").field(&(Rc::as_ptr(&self.0) as *const ())).finish()
    }
}

/// The view model held by the value of a property (`object` read as a view model): the object
/// form of a registered reference handle, the view model of an [`UntypedViewModel`], or the value
/// itself.
pub fn view_model_of_value(value: &BoxedValue) -> Rc<dyn Any> {
    if let Some(untyped) = value.downcast_ref::<UntypedViewModel>() {
        return untyped.0.clone();
    }
    let value: BoxedValue = ValueTypes::reference_object(value).unwrap_or_else(|| value.clone());
    let object: Rc<dyn Any> = value;
    object
}

/// [`view_model_of_value`] of an optional value.
pub fn view_model_of_optional(value: Option<&BoxedValue>) -> Option<Rc<dyn Any>> {
    value.map(view_model_of_value)
}

/// The value of a property that holds `view_model` (`object` written to a property): the view
/// model itself when its type is registered, else an [`UntypedViewModel`].
pub fn value_of_view_model(view_model: Rc<dyn Any>) -> BoxedValue {
    let type_id = (*view_model).type_id();
    let boxer = VIEW_MODEL_TYPES.with(|types| types.borrow().get(&type_id).copied());
    if let Some(boxed) = boxer.and_then(|boxer| boxer(view_model.clone())) {
        return boxed;
    }
    Rc::new(UntypedViewModel(view_model))
}

/// [`value_of_view_model`] of an optional view model.
pub fn value_of_optional_view_model(view_model: Option<Rc<dyn Any>>) -> Option<BoxedValue> {
    view_model.map(value_of_view_model)
}

/// Whether two values of properties hold the same view model (`ReferenceEquals` of the objects):
/// both null, or the same object behind any of their forms.
pub fn same_view_model(first: Option<&BoxedValue>, second: Option<&BoxedValue>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(first), Some(second)) => {
            let (first, second) = (view_model_of_value(first), view_model_of_value(second));
            std::ptr::addr_eq(Rc::as_ptr(&first), Rc::as_ptr(&second))
        }
        _ => false,
    }
}

/// Whether two view models are the same object (`ReferenceEquals`).
pub fn same_view_model_object(first: Option<&Rc<dyn Any>>, second: Option<&Rc<dyn Any>>) -> bool {
    match (first, second) {
        (None, None) => true,
        (Some(first), Some(second)) => std::ptr::addr_eq(Rc::as_ptr(first), Rc::as_ptr(second)),
        _ => false,
    }
}

/// Whether the value of a property holds a view model of type `VM` (`value is TViewModel`).
pub fn is_view_model_of<VM: 'static>(value: &BoxedValue) -> bool {
    view_model_of_value(value).is::<VM>()
}
