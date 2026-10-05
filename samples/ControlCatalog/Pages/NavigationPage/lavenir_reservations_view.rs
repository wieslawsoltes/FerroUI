//! Port of `Pages/NavigationPage/LAvenirReservationsView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/LAvenirReservationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct LAvenirReservationsView {
    base: UserControl,
}

user_control_class!(LAvenirReservationsView);
ferro_class_info!(LAvenirReservationsView { new: LAvenirReservationsView::new });
xaml_class!(LAvenirReservationsView, "/Pages/NavigationPage/LAvenirReservationsView.xaml");

impl LAvenirReservationsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
