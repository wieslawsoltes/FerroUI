//! Port of `Pages/DrawerPage/EcoTrackerCommunityView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/EcoTrackerCommunityView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct EcoTrackerCommunityView {
    base: UserControl,
}

user_control_class!(EcoTrackerCommunityView);
ferro_class_info!(EcoTrackerCommunityView { new: EcoTrackerCommunityView::new });
xaml_class!(EcoTrackerCommunityView, "/Pages/DrawerPage/EcoTrackerCommunityView.xaml");

impl EcoTrackerCommunityView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
