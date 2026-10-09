//! What the page classes of the sample name from the module `markup` of their crate
//! (`content_page_class!`, `xaml_class!`), for a crate whose markup is compiled: where the
//! sample populates a class from its embedded document with the run-time loader, the
//! class is populated here by the compiled markup of its document.

use ferroui_base::{ObjectType, Ref};
use ferroui_markup_xaml::XamlLoadException;

/// Declares a class that derives directly from `ContentPage`, without overrides. The
/// macro of the sample, as the sample has it.
macro_rules! content_page_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: ContentPage);
        ::ferroui_base::ferro_impl_classes!(
            $class: ::ferroui_base::FerroObjectImpl,
            ::ferroui_base::StyledElementImpl,
            ::ferroui_base::VisualImpl,
            ::ferroui_base::layout::LayoutableImpl,
            ::ferroui_base::interactivity::InteractiveImpl,
            ::ferroui_base::input::InputElementImpl,
            ::ferroui_controls::ControlImpl,
            ::ferroui_controls::primitives::TemplatedControlImpl,
            ::ferroui_controls::PageImpl
        );
    };
}
pub(crate) use content_page_class;

/// The compiled markup of the document of a class: the function `populate` the build
/// writes into the module of the document (`crate::pages`).
pub trait CompiledMarkup: ObjectType {
    /// Populates `this` from the compiled document of its class.
    fn populate(this: &Ref<Self>) -> Result<(), XamlLoadException>;
}

/// Declares the document of a class: its path, and `initialize_component()`, which
/// populates the instance with the compiled markup of the document.
macro_rules! xaml_class {
    ($class:ident, $path:literal) => {
        impl $class {
            /// The rooted asset path of the document of the class.
            pub const DOCUMENT_PATH: &'static str = $path;

            /// `InitializeComponent()`: populates the instance from the document of the
            /// class.
            ///
            /// # Panics
            /// Panics if the document fails to load (an exception of the constructor in
            /// the managed original).
            #[allow(dead_code)]
            fn initialize_component(&self) {
                if let Err(error) = <$class as $crate::markup::CompiledMarkup>::populate(&self.to_ref()) {
                    panic!("{error}");
                }
            }
        }
    };
}
pub(crate) use xaml_class;
