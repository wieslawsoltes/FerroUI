//! Port of `Pages/AnimationSpeedPage.xaml.cs`: the class of the document
//! `Pages/AnimationSpeedPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::view_models::AnimationSpeedPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct AnimationSpeedPage {
    base: UserControl,
}

user_control_class!(AnimationSpeedPage);
ferro_class_info!(AnimationSpeedPage { new: AnimationSpeedPage::new });
xaml_class!(AnimationSpeedPage, "/Pages/AnimationSpeedPage.xaml");

impl AnimationSpeedPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(AnimationSpeedPageViewModel::new() as BoxedValue));
        this
    }
}
