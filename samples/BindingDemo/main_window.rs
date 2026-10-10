//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`.

use crate::markup::xaml_class;
use crate::view_models::{MainWindowViewModel, TestItem};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl};

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
ferro_class_info!(MainWindow { new: MainWindow::new });
xaml_class!(MainWindow, "/MainWindow.xaml", create: MainWindow::new(), uninitialized: MainWindow::before_document());

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    /// The part of the constructor before `InitializeComponent()`: the instance with the
    /// resource the document refers to (`{StaticResource SharedItem}`), which a load of the
    /// document on its own populates too.
    fn before_document() -> Ref<Self> {
        let this = instantiate(Self::construct());
        let shared_item = TestItem::<String>::new();
        shared_item.set_value(Some(String::from("shared")));
        this.resources().set("SharedItem", Some(shared_item as BoxedValue));
        this
    }

    pub fn new() -> Ref<Self> {
        let this = Self::before_document();
        this.initialize_component();
        this.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));
        this
    }
}
