//! Port of `Pages/PipsPager/PipsPagerLargeCollectionPage.xaml.cs`: the class of the document
//! `Pages/PipsPager/PipsPagerLargeCollectionPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct PipsPagerLargeCollectionPage {
    base: UserControl,
}

user_control_class!(PipsPagerLargeCollectionPage);
ferro_class_info!(PipsPagerLargeCollectionPage { new: PipsPagerLargeCollectionPage::new });
xaml_class!(PipsPagerLargeCollectionPage, "/Pages/PipsPager/PipsPagerLargeCollectionPage.xaml");

impl PipsPagerLargeCollectionPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
