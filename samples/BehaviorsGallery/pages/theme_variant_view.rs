//! Port of `Views/Pages/ThemeVariantView` of the reference sample: the class of `pages/ThemeVariantView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class_info, ferro_thread_local, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ThemeVariantView {
    base: UserControl,
}

user_control_class!(ThemeVariantView);
ferro_class_info!(ThemeVariantView { new: ThemeVariantView::new });
xaml_class!(ThemeVariantView, "/pages/ThemeVariantView.xaml");

ferro_thread_local! {
    /// The value of [`ThemeVariantView::pink`], created once.
    static PINK: ThemeVariant = ThemeVariant::new("Pink", Some(ThemeVariant::light()));
}

impl ThemeVariantView {
    /// `Pink`: a custom theme variant that inherits the light variant.
    pub fn pink() -> ThemeVariant {
        PINK.with(Clone::clone)
    }

    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ThemeVariantView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ThemeVariantView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ThemeVariantView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Theme Variant",
    reference: "Views/Pages/ThemeVariantView",
    create: Some(create),
    types: TYPES,
    classes: &[&ThemeVariantView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
