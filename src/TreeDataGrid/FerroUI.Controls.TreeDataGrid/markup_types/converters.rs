//! Markup metadata of the converters of this crate (`FerroUI.Controls.Converters`).

use crate::converters::IndentConverter;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use std::rc::Rc;

// A converter is a reference type: an instance equals itself.
impl PartialEq for IndentConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class IndentConverter {
    namespace: "FerroUI.Controls.Converters",
    handles: [Rc<IndentConverter>, Option<Rc<IndentConverter>>],
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(IndentConverter)],
    fields: [Instance: Rc<IndentConverter> => IndentConverter::instance],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[<IndentConverter as MarkupTyped>::MARKUP];

/// Registers the nullable handle of the converter and its cast to the converter contract.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<IndentConverter>>();
    ValueTypes::register_cast::<Rc<IndentConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
}
