//! The converters of the reference (`Converters/`, namespace `FerroUI.Xaml.Interactions.Custom.Converters`).

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{IServiceProvider, MarkupType, MarkupTyped};
use ferroui_base::TypeInfo;
use std::rc::Rc;

mod pointer_event_args_converter;

pub use pointer_event_args_converter::PointerEventArgsConverter;

/// The classes of the converters, registered by `register_types`. The converters are plain
/// values with markup metadata, not classes of the object model.
pub(crate) const TYPES: &[&TypeInfo] = &[];

ferro_markup_type!(class PointerEventArgsConverter {
    namespace: "FerroUI.Xaml.Interactions.Custom.Converters",
    handles: [Rc<PointerEventArgsConverter>, Option<Rc<PointerEventArgsConverter>>],
    this: Rc<PointerEventArgsConverter>,
    interfaces: [Rc<dyn IValueConverter>],
    constructors: [() => || Rc::new(PointerEventArgsConverter)],
    fields: [Instance: Rc<PointerEventArgsConverter> => PointerEventArgsConverter::instance],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<dyn IValueConverter> =>
            |this: &Rc<PointerEventArgsConverter>, service_provider: Rc<dyn IServiceProvider>| -> Rc<dyn IValueConverter> {
                this.provide_value(&service_provider)
            },
    ],
});

/// The markup types of the converters.
const MARKUP_TYPES: &[&MarkupType] = &[<PointerEventArgsConverter as MarkupTyped>::MARKUP];

/// The public Rust paths of the converters, by the shortest path another crate names them by.
mod rust_paths {
    ferroui_base::ferro_rust_paths! {
        classes: [
        ],
        types: [
            crate::converters::PointerEventArgsConverter,
        ],
        contracts: [
        ],
        generics: [
        ],
        generic_contracts: [
        ],
    }
}

/// Registers the Rust paths and the markup types of the converters. Called once by
/// `register_types`.
pub(crate) fn register() {
    rust_paths::register_rust_paths();
    MarkupType::register_all(MARKUP_TYPES);
    ValueTypes::register_global(register_value_types);
}

fn register_value_types() {
    pointer_event_args_converter::register_value_types();
}
