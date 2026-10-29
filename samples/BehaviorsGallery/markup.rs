//! The documents of the gallery and the table of the classes the documents name.
//!
//! A class with a document is populated by the compiled markup the build of the crate generates
//! for the document (`build.rs`; docs/porting/xaml.md, 9.5.22), and the constructor of the class
//! calls it (`InitializeComponent()`): `initialize_component()`, which [`xaml_class!`] declares
//! over the table of the compiled documents the build writes ([`SAMPLE`]).

use crate::register_types::ASSEMBLY;
use sample_support::Sample;

pub(crate) use sample_support::xaml_class;
pub use sample_support::XamlClass;

/// Declares a class that derives directly from `UserControl`, without overrides (the class
/// declaration and the implementation traits of every level below): the class of a page.
macro_rules! user_control_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: UserControl);
        ::ferroui_base::ferro_impl_classes!(
            $class: ::ferroui_base::FerroObjectImpl,
            ::ferroui_base::StyledElementImpl,
            ::ferroui_base::VisualImpl,
            ::ferroui_base::layout::LayoutableImpl,
            ::ferroui_base::interactivity::InteractiveImpl,
            ::ferroui_base::input::InputElementImpl,
            ::ferroui_controls::ControlImpl,
            ::ferroui_controls::primitives::TemplatedControlImpl,
            ::ferroui_controls::ContentControlImpl
        );
    };
}
#[allow(unused_imports)]
pub(crate) use user_control_class;

// `ASSETS`, `DOCUMENT_ASSETS` (a test build), `DOCUMENTS` and `EXCLUDED` (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

// `COMPILED`: the documents the build compiled, each with the function that populates an
// instance of its class from its compiled markup (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/compiled_classes.rs"));

/// What the gallery states about its documents and assets.
pub static SAMPLE: Sample = Sample {
    assembly: &ASSEMBLY,
    register_types: crate::register_types::register_types,
    classes: &crate::register_types::CLASSES,
    compiled: COMPILED,
    assets: ASSETS,
    #[cfg(test)]
    document_assets: DOCUMENT_ASSETS,
    #[cfg(not(test))]
    document_assets: &[],
    documents: DOCUMENTS,
};

/// The element an untyped value holds, as `T` (`value as T`).
pub fn object_as<T: ferroui_base::ObjectType>(
    value: &ferroui_base::BoxedValue,
) -> Option<ferroui_base::Ref<T>> {
    ferroui_base::data::core::ValueTypes::as_object(&**value).and_then(|object| object.cast::<T>())
}

/// The content of a content control, as `T` (`Content as T`).
pub fn content_of<T: ferroui_base::ObjectType>(
    content: Option<ferroui_base::BoxedValue>,
) -> Option<ferroui_base::Ref<T>> {
    content.as_ref().and_then(object_as::<T>)
}
