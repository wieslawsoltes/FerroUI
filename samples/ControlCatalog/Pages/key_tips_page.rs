//! Not from upstream: the page of the access keys and their Key Tips.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ContentPage, TextBlock};
use ferroui_controls_keytips::AccessKeyManager;

#[repr(C)]
pub struct KeyTipsPage {
    base: ContentPage,
}

content_page_class!(KeyTipsPage);
ferro_class_info!(KeyTipsPage { new: KeyTipsPage::new });
xaml_class!(KeyTipsPage, "/Pages/KeyTipsPage.xaml");

impl KeyTipsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let output = this.get_control::<TextBlock>("Output").downgrade();
        let _ = this.add_handler(&Button::click_event(), move |_, e: &RoutedEventArgs| {
            let button = e.source().and_then(|source| source.downcast::<Button>().ok());
            if let (Some(output), Some(button)) = (output.upgrade(), button) {
                let key = AccessKeyManager::get_access_key(&button).unwrap_or_default();
                output.set_text(Some(format!("Invoked the button of key {key}.").as_str()));
            }
        });
        this
    }
}
