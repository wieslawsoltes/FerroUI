//! The class of the document `Pages/MaterialComponentsPage.xaml`: Button, the accent button, RepeatButton, ToggleButton, HyperlinkButton, DropDownButton, SplitButton and ToggleSplitButton.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Button, ContentControlImpl, Control, ControlImpl, UserControl};
use ferroui_themes_material::MaterialChip;

#[repr(C)]
pub struct MaterialComponentsPage {
    base: UserControl,
}

ferro_class!(MaterialComponentsPage: UserControl);
ferro_impl_classes!(
    MaterialComponentsPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(MaterialComponentsPage {
    new: MaterialComponentsPage::new
});
xaml_class!(MaterialComponentsPage, "/Pages/MaterialComponentsPage.xaml");

impl MaterialComponentsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let weak = this.downgrade();
        MaterialChip::dismiss_requested(
            &this.get_control::<ToggleButton>("RecipientChip"),
            move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.get_control::<ToggleButton>("RecipientChip")
                        .set_is_visible(false);
                    this.get_control::<Button>("RestoreRecipient")
                        .set_is_visible(true);
                }
            },
        );
        let weak = this.downgrade();
        this.get_control::<Button>("RestoreRecipient")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.get_control::<ToggleButton>("RecipientChip")
                        .set_is_visible(true);
                    this.get_control::<Button>("RestoreRecipient")
                        .set_is_visible(false);
                }
            });
        this
    }
}

/// The page as a control (the entry of the page table).
pub fn create() -> Ref<Control> {
    MaterialComponentsPage::new().upcast()
}
