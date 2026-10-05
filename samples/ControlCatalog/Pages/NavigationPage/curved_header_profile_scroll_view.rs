//! Port of `Pages/NavigationPage/CurvedHeaderProfileScrollView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/CurvedHeaderProfileScrollView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct CurvedHeaderProfileScrollView {
    base: UserControl,
}

user_control_class!(CurvedHeaderProfileScrollView);
ferro_class_info!(CurvedHeaderProfileScrollView { new: CurvedHeaderProfileScrollView::new });
xaml_class!(CurvedHeaderProfileScrollView, "/Pages/NavigationPage/CurvedHeaderProfileScrollView.xaml");

impl CurvedHeaderProfileScrollView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
