//! Port of `Views/Windows/WindowDemo` of the reference sample: the class of
//! `Views/Windows/WindowDemo.xaml`, the window of the page `WindowDragMoveBehaviorView` (a window
//! without decorations, moved by its own title bar).

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

#[repr(C)]
pub struct WindowDemo {
    base: Window,
}

ferro_class!(WindowDemo: Window);
ferro_class_info!(WindowDemo {
    new: WindowDemo::new
});
ferro_impl_classes!(
    WindowDemo: FerroObjectImpl,
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
xaml_class!(WindowDemo, "/Views/Windows/WindowDemo.xaml");

impl WindowDemo {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
        }
    }

    /// `WindowDemo()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
