//! Port of `Views/ChatPageView.xaml.cs`: the class of the document `Views/ChatPageView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ChatPageView {
    base: UserControl,
}

user_control_class!(ChatPageView);
ferro_class_info!(ChatPageView { new: ChatPageView::new });
xaml_class!(ChatPageView, "/Views/ChatPageView.xaml");

impl ChatPageView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
