//! Port of `Pages/ThemePage.xaml.cs`: the class of the document
//! `Pages/ThemePage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ComboBox, ContentPage, ItemsSource, ThemeVariantScope};

thread_local! {
    static PINK: ThemeVariant = ThemeVariant::new("Pink", Some(ThemeVariant::light()));
}

#[repr(C)]
pub struct ThemePage {
    base: ContentPage,
}

content_page_class!(ThemePage);
ferro_class_info!(ThemePage {
    new: ThemePage::new,
    markup: {
        fields: [Pink: ThemeVariant => ThemePage::pink],
    },
});
xaml_class!(ThemePage, "/Pages/ThemePage.xaml");

impl ThemePage {
    pub fn pink() -> ThemeVariant {
        PINK.with(Clone::clone)
    }

    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let selector = this.selector();
        selector.set_items_source(Some(ItemsSource::from_values([
            ThemeVariant::default(),
            ThemeVariant::dark(),
            ThemeVariant::light(),
            Self::pink(),
        ])));
        selector.set_selected_index(0);

        let weak = this.downgrade();
        selector.selection_changed(move |_, _| {
            let Some(this) = weak.upgrade() else { return };
            if let Some(theme) = from_markup_value::<ThemeVariant>(&this.selector().selected_item()) {
                this.theme_variant_scope().set_requested_theme_variant(Some(theme));
            }
        });
        this
    }

    fn selector(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("Selector")
    }

    fn theme_variant_scope(&self) -> Ref<ThemeVariantScope> {
        self.get_control::<ThemeVariantScope>("ThemeVariantScope")
    }
}
