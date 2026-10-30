//! Port of `Dialogs/SimpleDialog.xaml.cs` (the class of `Dialogs/SimpleDialog.xaml`).

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Button, ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};

/// The dialog of the dialog actions: a text and a button that closes it.
#[repr(C)]
pub struct SimpleDialog {
    base: Window,
}

ferro_class!(SimpleDialog: Window);
ferro_class_info!(SimpleDialog {
    new: SimpleDialog::new
});
ferro_impl_classes!(
    SimpleDialog: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
xaml_class!(SimpleDialog, "/Dialogs/SimpleDialog.xaml");

impl SimpleDialog {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
        }
    }

    /// `SimpleDialog()`: the close button closes the dialog.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        if let Some(close_button) = this.find_control::<Button>("CloseButton") {
            let dialog = this.downgrade();
            close_button.click(move |_, _| {
                if let Some(dialog) = dialog.upgrade() {
                    dialog.close();
                }
            });
        }
        this
    }
}
