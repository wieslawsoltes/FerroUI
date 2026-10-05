//! Port of `Pages/TableViewPage.xaml.cs`: the class of the document
//! `Pages/TableViewPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::TableViewPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct TableViewPage {
    base: ContentPage,
}

content_page_class!(TableViewPage);
ferro_class_info!(TableViewPage { new: TableViewPage::new });
xaml_class!(TableViewPage, "/Pages/TableViewPage.xaml");

impl TableViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(TableViewPageViewModel::new() as BoxedValue));
        this
    }
}
