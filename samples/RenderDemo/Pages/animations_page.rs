//! Port of `Pages/AnimationsPage.xaml.cs`: the class of the document
//! `Pages/AnimationsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::AnimationsPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct AnimationsPage {
    base: UserControl,
}

user_control_class!(AnimationsPage);
ferro_class_info!(AnimationsPage { new: AnimationsPage::new });
xaml_class!(AnimationsPage, "/Pages/AnimationsPage.xaml");

impl AnimationsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(AnimationsPageViewModel::new() as BoxedValue));
        this
    }
}
