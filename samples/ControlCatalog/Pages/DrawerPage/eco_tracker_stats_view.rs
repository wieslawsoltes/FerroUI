//! Port of `Pages/DrawerPage/EcoTrackerStatsView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/EcoTrackerStatsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct EcoTrackerStatsView {
    base: UserControl,
}

user_control_class!(EcoTrackerStatsView);
ferro_class_info!(EcoTrackerStatsView { new: EcoTrackerStatsView::new });
xaml_class!(EcoTrackerStatsView, "/Pages/DrawerPage/EcoTrackerStatsView.xaml");

impl EcoTrackerStatsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
