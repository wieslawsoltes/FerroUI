//! Port of `Pages/ComboBoxPage.xaml.cs`: the class of the document
//! `Pages/ComboBoxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::ComboBoxPageViewModel;
use ferroui_base::media::FontManager;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ComboBox, ContentPage, ItemsSource};

#[repr(C)]
pub struct ComboBoxPage {
    base: ContentPage,
}

content_page_class!(ComboBoxPage);
ferro_class_info!(ComboBoxPage { new: ComboBoxPage::new });
xaml_class!(ComboBoxPage, "/Pages/ComboBoxPage.xaml");

impl ComboBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let font_combo_box = this.font_combo_box();
        // The font families of the collection of the system fonts, as the list of the combo box.
        font_combo_box.set_items_source(Some(ItemsSource::from_values(FontManager::current().system_fonts().font_families())));
        font_combo_box.set_selected_index(0);
        this.set_data_context(Some(ComboBoxPageViewModel::new() as BoxedValue));
        this
    }

    fn font_combo_box(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("fontComboBox")
    }
}
