//! The converters of the gallery (namespace `BehaviorsGallery.Converters`, which the default XML
//! namespace of the framework includes): one module per reference file of `Converters/`.

use ferroui_base::data::converters::IMultiValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use std::rc::Rc;

pub mod classes_to_string_converter;

pub use classes_to_string_converter::ClassesToStringConverter;

/// The classes of this namespace (`X::TYPE`): none.
pub(crate) const TYPES: &[&TypeInfo] = &[];

/// The types of this namespace declared with `ferro_markup_type!` (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] =
    &[<ClassesToStringConverter as MarkupTyped>::MARKUP];

/// What the untyped value conversions must know about the types of this namespace.
pub(crate) fn register_value_types() {
    ValueTypes::register_reference::<ClassesToStringConverter>();
    ValueTypes::register_cast::<Rc<ClassesToStringConverter>, Rc<dyn IMultiValueConverter>>(
        |converter| converter.clone(),
    );
}
