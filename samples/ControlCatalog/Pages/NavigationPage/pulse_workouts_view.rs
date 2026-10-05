//! Port of `Pages/NavigationPage/PulseWorkoutsView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/PulseWorkoutsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PulseWorkoutsView {
    base: UserControl,
}

user_control_class!(PulseWorkoutsView);
ferro_class_info!(PulseWorkoutsView { new: PulseWorkoutsView::new });
xaml_class!(PulseWorkoutsView, "/Pages/NavigationPage/PulseWorkoutsView.xaml");

impl PulseWorkoutsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
