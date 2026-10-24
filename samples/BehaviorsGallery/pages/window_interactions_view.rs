//! Port of `Views/Pages/WindowInteractionsView` of the reference sample: the class of `pages/WindowInteractionsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::views::windows::sample_window::SampleWindow;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct WindowInteractionsView {
    base: UserControl,
}

user_control_class!(WindowInteractionsView);
ferro_class_info!(WindowInteractionsView { new: WindowInteractionsView::new });
xaml_class!(WindowInteractionsView, "/pages/WindowInteractionsView.xaml");

impl WindowInteractionsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `WindowInteractionsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    WindowInteractionsView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the window it opens.
pub(crate) const TYPES: &[&TypeInfo] = &[WindowInteractionsView::TYPE, SampleWindow::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Window Interactions",
    reference: "Views/Pages/WindowInteractionsView",
    create: Some(create),
    types: TYPES,
    classes: &[
        &WindowInteractionsView::XAML_CLASS,
        &SampleWindow::XAML_CLASS,
    ],
    markup_types: &[],
    register_value_types: no_value_types,
};
