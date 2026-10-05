//! Port of `Pages/SectionPage.xaml.cs`: the class of the document
//! `Pages/SectionPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::models::HomeSection;
use crate::view_models::SectionViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;
use std::rc::Rc;

#[repr(C)]
pub struct SectionPage {
    base: ContentPage,
}

content_page_class!(SectionPage);
ferro_class_info!(SectionPage { new: SectionPage::new });
xaml_class!(SectionPage, "/Pages/SectionPage.xaml");

impl SectionPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    /// `new SectionPage(homeSection)`.
    pub fn with_home_section(home_section: Rc<HomeSection>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.set_data_context(Some(Rc::new(SectionViewModel::new(home_section)) as BoxedValue));
        this
    }

    /// `new SectionPage()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
