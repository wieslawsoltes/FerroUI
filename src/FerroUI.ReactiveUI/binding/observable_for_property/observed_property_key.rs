//! `ObservedPropertyKey`.

use ferroui_system::component_model::PropertyOwner;
use std::any::TypeId;

/// One property of one type, for the records kept per property rather than per object
/// (`ObservedPropertyKey`): the warning of [`POCOObservableForProperty`] is written once per key.
///
/// [`POCOObservableForProperty`]: crate::binding::observable_for_property::POCOObservableForProperty
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObservedPropertyKey {
    /// The type of the object that owns the property, when the owner gives its object (`Type`).
    pub type_id: Option<TypeId>,
    /// The name of the property (`PropertyName`).
    pub property_name: String,
}

impl ObservedPropertyKey {
    /// The key of `property_name` on a type.
    pub fn new(type_id: Option<TypeId>, property_name: impl Into<String>) -> Self {
        Self { type_id, property_name: property_name.into() }
    }

    /// The key of `property_name` on the type of the object of `owner`.
    pub fn of(owner: &PropertyOwner, property_name: impl Into<String>) -> Self {
        // `type_id` of the object itself (through `dyn Any`), not of the `Rc` that holds it.
        Self::new(owner.object.as_ref().map(|object| (**object).type_id()), property_name)
    }
}

