//! The documents of the example and the table of the classes the documents name.
//!
//! A class with a document is populated by the compiled markup the build of the crate generates
//! for the document (`build.rs`; docs/porting/xaml.md, 9.5.22), and the constructor of the class
//! calls it (`InitializeComponent()`): `initialize_component()`, which [`xaml_class!`] declares
//! over the table of the compiled documents the build writes ([`SAMPLE`]).

use crate::register_types::ASSEMBLY;
use sample_support::Sample;

pub use sample_support::XamlClass;
pub(crate) use sample_support::xaml_class;

/// Declares a view class that derives directly from the reactive user control of the integration
/// (`ReactiveUserControlBase`, the class markup creates for `ReactiveUserControl<TViewModel>`),
/// without overrides: the class declaration and the implementation traits of every level below.
/// The module that invokes it imports `ReactiveUserControlBase` (`ferro_class!` takes the base as
/// a name).
macro_rules! reactive_user_control_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: ReactiveUserControlBase);
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
pub(crate) use reactive_user_control_class;

/// Declares a window class that derives directly from the reactive window of the integration
/// (`ReactiveWindowBase`, the class markup creates for `ReactiveWindow<TViewModel>`), without
/// overrides: the class declaration and the implementation traits of every level below. The module
/// that invokes it imports `ReactiveWindowBase`.
macro_rules! reactive_window_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: ReactiveWindowBase);
        ::ferroui_base::ferro_impl_classes!(
            $class: ::ferroui_base::FerroObjectImpl,
            ::ferroui_base::StyledElementImpl,
            ::ferroui_base::VisualImpl,
            ::ferroui_base::layout::LayoutableImpl,
            ::ferroui_base::interactivity::InteractiveImpl,
            ::ferroui_base::input::InputElementImpl,
            ::ferroui_controls::ControlImpl,
            ::ferroui_controls::primitives::TemplatedControlImpl,
            ::ferroui_controls::ContentControlImpl,
            ::ferroui_controls::TopLevelImpl,
            ::ferroui_controls::WindowBaseImpl,
            ::ferroui_controls::WindowImpl
        );
    };
}
pub(crate) use reactive_window_class;

// `ASSETS`, `DOCUMENT_ASSETS` (a test build), `DOCUMENTS` and `EXCLUDED` (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

// `COMPILED`: the documents the build compiled, each with the function that populates an
// instance of its class from its compiled markup (`build.rs`).
include!(concat!(env!("OUT_DIR"), "/compiled_classes.rs"));

/// What the example states about its documents and assets.
pub static SAMPLE: Sample = Sample {
    assembly: &ASSEMBLY,
    register_types: crate::register_types::register_types,
    classes: &[crate::register_types::ROOT_CLASSES, crate::models::CLASSES, crate::view_models::CLASSES, crate::views::CLASSES],
    compiled: COMPILED,
    assets: ASSETS,
    #[cfg(test)]
    document_assets: DOCUMENT_ASSETS,
    #[cfg(not(test))]
    document_assets: &[],
    documents: DOCUMENTS,
};
