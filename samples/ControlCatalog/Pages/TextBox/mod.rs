//! The samples of the `TextBox` gallery (directory `Pages/TextBox`): one
//! module per upstream code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod text_box_editing_page;
mod text_box_first_look_page;
mod text_box_fonts_page;
mod text_box_input_page;
mod text_box_multiline_page;
mod text_box_placeholder_page;
mod text_box_selection_page;
mod text_box_text_layout_page;
mod text_box_validation_page;

pub use text_box_editing_page::TextBoxEditingPage;
pub use text_box_first_look_page::TextBoxFirstLookPage;
pub use text_box_fonts_page::TextBoxFontsPage;
pub use text_box_input_page::TextBoxInputPage;
pub use text_box_multiline_page::TextBoxMultilinePage;
pub use text_box_placeholder_page::TextBoxPlaceholderPage;
pub use text_box_selection_page::TextBoxSelectionPage;
pub use text_box_text_layout_page::TextBoxTextLayoutPage;
pub use text_box_validation_page::TextBoxValidationPage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    TextBoxEditingPage::TYPE,
    TextBoxFirstLookPage::TYPE,
    TextBoxFontsPage::TYPE,
    TextBoxInputPage::TYPE,
    TextBoxMultilinePage::TYPE,
    TextBoxPlaceholderPage::TYPE,
    TextBoxSelectionPage::TYPE,
    TextBoxTextLayoutPage::TYPE,
    TextBoxValidationPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &TextBoxEditingPage::XAML_CLASS,
    &TextBoxFirstLookPage::XAML_CLASS,
    &TextBoxFontsPage::XAML_CLASS,
    &TextBoxInputPage::XAML_CLASS,
    &TextBoxMultilinePage::XAML_CLASS,
    &TextBoxPlaceholderPage::XAML_CLASS,
    &TextBoxSelectionPage::XAML_CLASS,
    &TextBoxTextLayoutPage::XAML_CLASS,
    &TextBoxValidationPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
