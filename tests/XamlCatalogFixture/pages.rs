//! The page classes of the fixture (`ControlCatalog.Pages`): the source files of the
//! sample, compiled here as they are, each populated by the compiled markup of its
//! document.

use ferroui_base::{Ref, TypeInfo};
use ferroui_markup_xaml::XamlLoadException;

use crate::markup::CompiledMarkup;

#[path = "../../samples/ControlCatalog/Pages/button_spinner_page.rs"]
mod button_spinner_page;
#[path = "../../samples/ControlCatalog/Pages/check_box_page.rs"]
mod check_box_page;
#[path = "../../samples/ControlCatalog/Pages/image_page.rs"]
mod image_page;
#[path = "../../samples/ControlCatalog/Pages/progress_bar_page.rs"]
mod progress_bar_page;
#[path = "../../samples/ControlCatalog/Pages/wrap_panel_page.rs"]
mod wrap_panel_page;

pub use button_spinner_page::ButtonSpinnerPage;
pub use check_box_page::CheckBoxPage;
pub use image_page::ImagePage;
pub use progress_bar_page::ProgressBarPage;
pub use wrap_panel_page::WrapPanelPage;

impl CompiledMarkup for ButtonSpinnerPage {
    fn populate(this: &Ref<Self>) -> Result<(), XamlLoadException> {
        crate::compiled_button_spinner_page::populate(None, this)
    }
}

impl CompiledMarkup for CheckBoxPage {
    fn populate(this: &Ref<Self>) -> Result<(), XamlLoadException> {
        crate::compiled_check_box_page::populate(None, this)
    }
}

impl CompiledMarkup for ProgressBarPage {
    fn populate(this: &Ref<Self>) -> Result<(), XamlLoadException> {
        crate::compiled_progress_bar_page::populate(None, this)
    }
}

impl CompiledMarkup for ImagePage {
    fn populate(this: &Ref<Self>) -> Result<(), XamlLoadException> {
        crate::compiled_image_page::populate(None, this)
    }
}

impl CompiledMarkup for WrapPanelPage {
    fn populate(this: &Ref<Self>) -> Result<(), XamlLoadException> {
        crate::compiled_wrap_panel_page::populate(None, this)
    }
}

/// The classes of this namespace.
pub(crate) const TYPES: &[&TypeInfo] =
    &[ButtonSpinnerPage::TYPE, CheckBoxPage::TYPE, ImagePage::TYPE, ProgressBarPage::TYPE, WrapPanelPage::TYPE];
