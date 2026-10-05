//! Port of `Pages/DrawerPage/EcoTrackerHabitsView.xaml.cs`: the class of the document
//! `Pages/DrawerPage/EcoTrackerHabitsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct EcoTrackerHabitsView {
    base: UserControl,
}

user_control_class!(EcoTrackerHabitsView);
ferro_class_info!(EcoTrackerHabitsView { new: EcoTrackerHabitsView::new });
xaml_class!(EcoTrackerHabitsView, "/Pages/DrawerPage/EcoTrackerHabitsView.xaml");

impl EcoTrackerHabitsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
