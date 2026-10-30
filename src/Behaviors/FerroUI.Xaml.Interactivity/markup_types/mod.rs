//! Markup metadata of the types of this crate that are not declared with their type: the
//! collections (`collections.rs`), the comparison kinds (`enums.rs`) and the accessors of the
//! attached property `Interaction.Behaviors` (`interaction.rs`). The classes state their
//! properties through their registered properties, as every class of the framework; the
//! templates declare theirs next to them.

use crate::{ActionCollectionTemplate, BehaviorCollectionTemplate, NotificationTemplate, ObjectTemplate};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::styling::ITemplate;
use ferroui_base::BoxedValue;
use std::any::Any;
use std::rc::Rc;

mod collections;
mod enums;
mod interaction;

/// The templates of the crate (declared next to them).
const TEMPLATES: &[&MarkupType] = &[
    <ActionCollectionTemplate as MarkupTyped>::MARKUP,
    <BehaviorCollectionTemplate as MarkupTyped>::MARKUP,
    <NotificationTemplate as MarkupTyped>::MARKUP,
    <ObjectTemplate as MarkupTyped>::MARKUP,
];

/// The lists of the types declared for markup.
const TYPE_LISTS: &[&[&MarkupType]] = &[collections::TYPES, enums::TYPES, interaction::TYPES, TEMPLATES];

/// Adds the types of this crate to the process-wide table of types with markup metadata. Called
/// once by `register_types()`.
pub(crate) fn register() {
    for types in TYPE_LISTS {
        MarkupType::register_all(types);
    }
    ValueTypes::register_global(register_value_types);
}

/// The shared object behind a value of a reference type `T`: its handle (`Rc<T>`), or the object
/// itself (a reference type is boxed as the object).
fn rc_of<T: 'static>(value: &BoxedValue) -> Option<Rc<T>> {
    if let Some(handle) = value.downcast_ref::<Rc<T>>() {
        return Some(handle.clone());
    }
    if let Some(handle) = value.downcast_ref::<Option<Rc<T>>>() {
        return handle.clone();
    }
    if !value.is::<T>() {
        return None;
    }
    let any: Rc<dyn Any> = value.clone();
    any.downcast::<T>().ok()
}

/// What the untyped value conversions must know about the types of this crate: the nullable
/// forms of the collections, and the templates as reference types assignable to the template
/// contract (what a style setter builds per element).
fn register_value_types() {
    collections::register_value_types();
    enums::register_value_types();

    macro_rules! templates {
        ($($template:ty),* $(,)?) => {$(
            ValueTypes::register_reference::<$template>();
            ValueTypes::register_cast::<Rc<$template>, Rc<dyn ITemplate>>(|value| {
                let handle: Rc<dyn ITemplate> = value.clone();
                handle
            });
            ValueTypes::register_boxed_cast::<$template, Rc<dyn ITemplate>>(|boxed| {
                rc_of::<$template>(boxed).map(|value| value as Rc<dyn ITemplate>)
            });
        )*};
    }
    templates![ActionCollectionTemplate, BehaviorCollectionTemplate, NotificationTemplate, ObjectTemplate];
    ValueTypes::register_nullable::<Rc<dyn ITemplate>>();
}
