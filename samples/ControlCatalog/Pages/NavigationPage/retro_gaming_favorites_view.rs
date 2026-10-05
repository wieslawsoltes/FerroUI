//! Port of `Pages/NavigationPage/RetroGamingFavoritesView.xaml.cs`: the class of the document
//! `Pages/NavigationPage/RetroGamingFavoritesView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct RetroGamingFavoritesView {
    base: UserControl,
}

user_control_class!(RetroGamingFavoritesView);
ferro_class_info!(RetroGamingFavoritesView { new: RetroGamingFavoritesView::new });
xaml_class!(RetroGamingFavoritesView, "/Pages/NavigationPage/RetroGamingFavoritesView.xaml");

impl RetroGamingFavoritesView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
