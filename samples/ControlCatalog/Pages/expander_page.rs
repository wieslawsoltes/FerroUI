//! Port of `Pages/ExpanderPage.xaml.cs`: the class of the document
//! `Pages/ExpanderPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::ExpanderPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, Expander};

#[repr(C)]
pub struct ExpanderPage {
    base: ContentPage,
}

content_page_class!(ExpanderPage);
ferro_class_info!(ExpanderPage { new: ExpanderPage::new });
xaml_class!(ExpanderPage, "/Pages/ExpanderPage.xaml");

impl ExpanderPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ExpanderPageViewModel::new() as BoxedValue));

        this.collapsing_disabled_expander().collapsing(|_s, e| e.set_cancel(true));
        this.expanding_disabled_expander().expanding(|_s, e| e.set_cancel(true));
        this
    }

    fn collapsing_disabled_expander(&self) -> Ref<Expander> {
        self.get_control::<Expander>("CollapsingDisabledExpander")
    }

    fn expanding_disabled_expander(&self) -> Ref<Expander> {
        self.get_control::<Expander>("ExpandingDisabledExpander")
    }
}
