//! What the untyped value conversions of markup and bindings must know about the types of this
//! crate: the handler contracts as nullable values of the `Handler` properties, the element
//! references of the properties that name a control, and the storage item handles.

use crate::values::StorageItemValue;
use crate::{IDragHandler, IDropHandler};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_controls::{Control, ItemsControl};
use std::rc::Rc;

// The handler contracts, so that a document can assign a handler declared as a resource to the
// `Handler` property of a behavior (`Handler="{StaticResource ...}"`).
ferroui_base::ferro_markup_type!(interface dyn IDropHandler as "IDropHandler" {
    handles: [Rc<dyn IDropHandler>, Option<Rc<dyn IDropHandler>>],
});

ferroui_base::ferro_markup_type!(interface dyn IDragHandler as "IDragHandler" {
    handles: [Rc<dyn IDragHandler>, Option<Rc<dyn IDragHandler>>],
});

/// The markup types of the contracts of this crate.
const MARKUP_TYPES: &[&MarkupType] = &[<dyn IDropHandler as MarkupTyped>::MARKUP, <dyn IDragHandler as MarkupTyped>::MARKUP];

/// Registers the markup types and the value types of this crate. Called once by
/// `register_types()`.
pub(crate) fn register() {
    MarkupType::register_all(MARKUP_TYPES);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    ValueTypes::register_nullable::<Rc<dyn IDragHandler>>();
    ValueTypes::register_nullable::<Rc<dyn IDropHandler>>();
    ValueTypes::register_nullable::<StorageItemValue>();
    ValueTypes::register_element_ref::<Control>();
    ValueTypes::register_element_ref::<ItemsControl>();
}
