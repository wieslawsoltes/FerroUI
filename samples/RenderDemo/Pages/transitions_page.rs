//! Port of `Pages/TransitionsPage.xaml.cs`: the class of the document
//! `Pages/TransitionsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::AnimationsPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TransitionsPage {
    base: UserControl,
}

user_control_class!(TransitionsPage);
ferro_class_info!(TransitionsPage { new: TransitionsPage::new });
xaml_class!(TransitionsPage, "/Pages/TransitionsPage.xaml");

impl TransitionsPage {
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
