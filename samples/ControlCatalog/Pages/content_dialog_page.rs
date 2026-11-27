//! The class of the document `Pages/ContentDialogPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, ContentDialog, ContentDialogButton, ContentDialogResult, ContentPage, TextBlock};
use std::rc::Rc;

#[repr(C)]
pub struct ContentDialogPage {
    base: ContentPage,
}

content_page_class!(ContentDialogPage);
ferro_class_info!(ContentDialogPage { new: ContentDialogPage::new });
xaml_class!(ContentDialogPage, "/Pages/ContentDialogPage.xaml");

impl ContentDialogPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let button = this.get_control::<Button>("ShowDialog");
        let output = this.get_control::<TextBlock>("DialogResult");
        let weak_button = button.downgrade();
        let weak_output = output.downgrade();
        button.click(move |_, _| {
            let (Some(button), Some(output)) = (weak_button.upgrade(), weak_output.upgrade()) else {
                return;
            };
            let dialog = ContentDialog::new();
            dialog.set_title(Some(Rc::new("Save your work?".to_string()) as BoxedValue));
            dialog.set_content(Some(Rc::new("The changes are lost if you do not save them.".to_string()) as BoxedValue));
            dialog.set_primary_button_text(Some("Save"));
            dialog.set_secondary_button_text(Some("Don't Save"));
            dialog.set_close_button_text(Some("Cancel"));
            dialog.set_default_button(ContentDialogButton::Primary);
            let weak_output = output.downgrade();
            dialog
                .closed(move |args| {
                    if let Some(output) = weak_output.upgrade() {
                        output.set_text(Some(match args.result() {
                            ContentDialogResult::Primary => "Saved",
                            ContentDialogResult::Secondary => "Not saved",
                            ContentDialogResult::None => "Cancelled",
                        }));
                    }
                })
                .detach();
            // A second click while a dialog is open does nothing.
            drop(dialog.show_async(&button));
        });
        this
    }
}
