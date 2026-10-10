//! Port of `Pages/Transform3DPage.xaml.cs`: the class of the document
//! `Pages/Transform3DPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::Transform3DPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct Transform3DPage {
    base: UserControl,
}

user_control_class!(Transform3DPage);
ferro_class_info!(Transform3DPage { new: Transform3DPage::new });
xaml_class!(Transform3DPage, "/Pages/Transform3DPage.xaml");

impl Transform3DPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(Transform3DPageViewModel::new() as BoxedValue));
        this
    }
}
