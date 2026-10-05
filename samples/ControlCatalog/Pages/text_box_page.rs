//! Port of `Pages/TextBoxPage.xaml.cs`: the class of the document
//! `Pages/TextBoxPage.xaml`.

use super::{
    TextBoxEditingPage, TextBoxFirstLookPage, TextBoxFontsPage, TextBoxInputPage, TextBoxMultilinePage,
    TextBoxPlaceholderPage, TextBoxSelectionPage, TextBoxTextLayoutPage, TextBoxValidationPage,
};
use crate::controls::{SampleGalleryPage, SampleGroups, SampleInfo};
use crate::markup::xaml_class;
use ferroui_base::collections::FerroList;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, PageImpl};
use std::rc::Rc;

#[repr(C)]
pub struct TextBoxPage {
    base: SampleGalleryPage,
}

ferro_class!(TextBoxPage: SampleGalleryPage);
ferro_impl_classes!(
    TextBoxPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);
ferro_class_info!(TextBoxPage { new: TextBoxPage::new });
xaml_class!(TextBoxPage, "/Pages/TextBoxPage.xaml");

impl TextBoxPage {
    /// The registry of the samples of the gallery (the static field `Demos`).
    pub(crate) fn demos() -> FerroList<Rc<SampleInfo>> {
        thread_local! {
            static DEMOS: FerroList<Rc<SampleInfo>> = FerroList::from_items([
                SampleInfo::new(
                    SampleGroups::OVERVIEW,
                    "First Look",
                    "A single-line TextBox with a placeholder and a live readout of its Text.",
                    || TextBoxFirstLookPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::APPEARANCE,
                    "Placeholder and Inner Content",
                    "PlaceholderText, the floating placeholder, a themed PlaceholderForeground, inner left and right content, and the clearButton and revealPasswordButton classes.",
                    || TextBoxPlaceholderPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::APPEARANCE,
                    "Text Layout",
                    "TextAlignment, TextWrapping, LineHeight and the MinLines and MaxLines height range, all on one box.",
                    || TextBoxTextLayoutPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::APPEARANCE,
                    "Fonts and Complex Scripts",
                    "Embedded font faces, font size and the input method, and CJK and right-to-left paragraphs.",
                    || TextBoxFontsPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::FEATURES,
                    "Multiline Editing",
                    "AcceptsReturn, AcceptsTab, the NewLine string and the Multiline soft-keyboard hint.",
                    || TextBoxMultilinePage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::FEATURES,
                    "Selection and Caret",
                    "Setting the selection from XAML and from code, and styling the selection and the caret.",
                    || TextBoxSelectionPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::FEATURES,
                    "Input Restrictions and Masks",
                    "IsReadOnly, MaxLength, password characters, soft-keyboard hints and MaskedTextBox.",
                    || TextBoxInputPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::FEATURES,
                    "Validation",
                    "Errors raised by a model through INotifyDataErrorInfo, and errors set directly through DataValidationErrors.",
                    || TextBoxValidationPage::new().upcast(),
                ),
                SampleInfo::new(
                    SampleGroups::EVENTS,
                    "Text, Clipboard and Undo",
                    "TextChanging and TextChanged, the three cancellable clipboard events, and the undo stack.",
                    || TextBoxEditingPage::new().upcast(),
                ),
            ]);
        }
        DEMOS.with(FerroList::clone)
    }

    pub fn construct() -> Self {
        Self { base: SampleGalleryPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_samples(Self::demos());
        this
    }
}
