//! Port of `Views/ExpanderPageView.xaml.cs`: the class of the document `Views/ExpanderPageView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ExpanderPageView {
    base: UserControl,
}

user_control_class!(ExpanderPageView);
ferro_class_info!(ExpanderPageView { new: ExpanderPageView::new });
xaml_class!(ExpanderPageView, "/Views/ExpanderPageView.xaml");

impl ExpanderPageView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
