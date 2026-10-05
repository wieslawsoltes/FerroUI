//! Port of `Pages/TabStripPage.xaml.cs`: the class of the document
//! `Pages/TabStripPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::{TabControlPageViewModel, TabControlPageViewModelItem};
use ferroui_base::data::model::BindableList;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct TabStripPage {
    base: ContentPage,
}

content_page_class!(TabStripPage);
ferro_class_info!(TabStripPage { new: TabStripPage::new });
xaml_class!(TabStripPage, "/Pages/TabStripPage.xaml");

impl TabStripPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let view_model = TabControlPageViewModel::new();
        view_model.set_tabs(Some(BindableList::new([
            TabControlPageViewModelItem::new().with_header("Item 1"),
            TabControlPageViewModelItem::new().with_header("Item 2"),
            TabControlPageViewModelItem::new().with_header("Disabled").with_is_enabled(false),
        ])));
        this.set_data_context(Some(view_model as BoxedValue));
        this
    }
}
