//! Port of `TestItemView.xaml.cs`: the class of the document `TestItemView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TestItemView {
    base: UserControl,
}

user_control_class!(TestItemView);
ferro_class_info!(TestItemView { new: TestItemView::new });
xaml_class!(TestItemView, "/TestItemView.xaml");

impl TestItemView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
