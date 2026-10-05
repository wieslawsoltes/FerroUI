//! Port of `Controls/SamplePage.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    Ref, StaticType, StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentPage, ControlImpl, PageImpl};

/// The standard catalog page: a scrolling page whose header is shown by the
/// shell navigation bar and whose `Description` is rendered above the
/// content. Pages stack `SampleSection`s inside it. A page whose content
/// scrolls itself sets `ScrollViewer.VerticalScrollBarVisibility="Disabled"`,
/// so the content gets the page height instead of growing to its full
/// extent.
#[repr(C)]
pub struct SamplePage {
    base: ContentPage,
}

ferro_class!(SamplePage: ContentPage);
ferro_class_info!(SamplePage { new: SamplePage::new });
ferro_impl_classes!(
    SamplePage: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);

impl StyledElementImpl for SamplePage {
    // Pages derive from this class, and a theme is looked up by the exact type.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <SamplePage as StaticType>::TYPE
    }
}

ferro_properties! {
    impl SamplePage {
        pub fn description_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SamplePage, _>("Description", None)
        }
    }
}

impl SamplePage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// One or two sentences saying what the control is for. Shown under the
    /// page title.
    pub fn description(&self) -> Option<String> {
        self.get_value(Self::description_property())
    }

    pub fn set_description(&self, value: Option<&str>) {
        self.set_value(Self::description_property(), value.map(str::to_string))
    }
}

/// Declares a class that derives directly from `SamplePage`, without
/// overrides (the class declaration and the implementation traits of every
/// level below). `SamplePage` must be imported in the file.
#[allow(unused_macros)]
macro_rules! sample_page_class {
    ($class:ident) => {
        ::ferroui_base::ferro_class!($class: SamplePage);
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
pub(crate) use sample_page_class;
