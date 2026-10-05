//! Port of `Pages/TreeViewPage.xaml.cs`: the class of the document
//! `Pages/TreeViewPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::TreeViewPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct TreeViewPage {
    base: ContentPage,
}

content_page_class!(TreeViewPage);
ferro_class_info!(TreeViewPage { new: TreeViewPage::new });
xaml_class!(TreeViewPage, "/Pages/TreeViewPage.xaml");

impl TreeViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(TreeViewPageViewModel::new() as BoxedValue));
        this
    }
}
