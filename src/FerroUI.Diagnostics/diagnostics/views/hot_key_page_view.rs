//! Port of `Diagnostics/Views/HotKeyPageView.xaml.cs`: the class of the
//! document `HotKeyPageView.xaml`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, UserControl};

/// The view of the hot keys page: the hot keys of the developer tools and
/// what they do.
#[repr(C)]
pub struct HotKeyPageView {
    base: UserControl,
}

ferro_class!(HotKeyPageView: UserControl);
ferro_impl_classes!(
    HotKeyPageView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(HotKeyPageView { new: HotKeyPageView::new });

impl HotKeyPageView {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Diagnostics/diagnostics/views/HotKeyPageView.xaml";

    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// Creates the view and populates it from its document
    /// (`InitializeComponent()`: the compiled markup of the document, which
    /// the build of the crate generates).
    ///
    /// # Panics
    /// Panics when the document fails to load (an exception of the
    /// constructor in the original).
    pub fn new() -> Ref<Self> {
        crate::register_types();
        let this = instantiate(Self::construct());
        Self::initialize_component(&this);
        this
    }

    /// `InitializeComponent()`.
    fn initialize_component(this: &Ref<Self>) {
        if let Err(error) = crate::compiled_diagnostics_views_hot_key_page_view::populate(None, this) {
            panic!("{}: {error}", Self::DOCUMENT_URI);
        }
    }
}
