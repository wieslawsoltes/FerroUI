//! Interactive Material search, snackbar queue and modal sheet examples.
use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ContentControlImpl, Control, ControlImpl, TextBlock, UserControl};
use ferroui_themes_material::{
    MaterialBottomSheetHost, MaterialSearchView, MaterialSnackbarHost, MaterialSnackbarMessage,
};

#[repr(C)]
pub struct MaterialInteractionsPage {
    base: UserControl,
}
ferro_class!(MaterialInteractionsPage: UserControl);
ferro_impl_classes!(MaterialInteractionsPage: FerroObjectImpl, StyledElementImpl, VisualImpl,
    LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);
ferro_class_info!(MaterialInteractionsPage {
    new: MaterialInteractionsPage::new
});
xaml_class!(
    MaterialInteractionsPage,
    "/Pages/MaterialInteractionsPage.xaml"
);

impl MaterialInteractionsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let weak = this.downgrade();
        this.get_control::<Button>("QueueMessages")
            .click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let host = this.get_control::<MaterialSnackbarHost>("SnackbarHost");
                    host.show("Your changes have been saved.");
                    host.show(
                        MaterialSnackbarMessage::new("A second message, shown after the first.")
                            .with_action("Undo")
                            .with_dismiss_action(true),
                    );
                }
            });
        for (name, expand) in [("OpenSheet", false), ("ExpandSheet", true)] {
            let weak = this.downgrade();
            this.get_control::<Button>(name).click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let sheet = this.get_control::<MaterialBottomSheetHost>("DemoSheet");
                    if expand {
                        sheet.expand();
                    } else {
                        sheet.show();
                    }
                }
            });
        }
        let weak = this.downgrade();
        this.get_control::<Button>("CloseSheet").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.get_control::<MaterialBottomSheetHost>("DemoSheet")
                    .hide();
            }
        });
        let weak = this.downgrade();
        this.get_control::<MaterialSearchView>("Search")
            .query_submitted(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    let query = this
                        .get_control::<MaterialSearchView>("Search")
                        .query()
                        .unwrap_or_default();
                    this.get_control::<TextBlock>("SearchStatus")
                        .set_text(Some(&format!("Submitted: {query}")));
                }
            });
        this
    }
}
pub fn create() -> Ref<Control> {
    MaterialInteractionsPage::new().upcast()
}
