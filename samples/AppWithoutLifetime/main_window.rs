//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`.

use crate::markup::xaml_class;
use crate::sub::Sub;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl};
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
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
ferro_class_info!(MainWindow {
    new: MainWindow::new,
    markup: {
        methods: [
            fn Open(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MainWindow>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.open(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    pub fn open(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        Sub::new().show_with_owner(self);
    }
}
