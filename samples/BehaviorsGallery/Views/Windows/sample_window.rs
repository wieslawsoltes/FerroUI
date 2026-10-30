//! Port of `Views/Windows/SampleWindow.xaml.cs` of the reference sample (the class of
//! `Views/Windows/SampleWindow.xaml`): the window the window interactions page opens.

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
    ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};

/// A window with a button that closes it.
#[repr(C)]
pub struct SampleWindow {
    base: Window,
}

ferro_class!(SampleWindow: Window);
ferro_class_info!(SampleWindow {
    new: SampleWindow::new
});
ferro_impl_classes!(
    SampleWindow: FerroObjectImpl,
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
xaml_class!(SampleWindow, "/Views/Windows/SampleWindow.xaml");

impl SampleWindow {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
        }
    }

    /// `SampleWindow()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
