//! Port of `MainWindow.axaml.cs`: the class of the document `MainWindow.xaml`. The handlers of
//! the pages moved to the views of the pages (`views`), which the ControlCatalog shows too.

use crate::markup::xaml_class;
use crate::view_models::MainWindowViewModel;
use crate::views::update_realized_count;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, TabControl, TabItem, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use std::time::Duration;

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
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));

        let window = this.downgrade();
        DispatcherTimer::run(
            move || {
                let Some(window) = window.upgrade() else {
                    return false;
                };
                window.update_realized_count();
                true
            },
            Duration::from_millis(500),
            DispatcherPriority::BACKGROUND,
        );
        this
    }

    /// The tabs of the window.
    pub fn tabs(&self) -> Ref<TabControl> {
        self.get_control::<TabControl>("tabs")
    }

    fn update_realized_count(&self) {
        let tab_item = self.tabs().selected_item().and_then(|item| ferroui_base::metadata::from_markup_value::<Ref<TabItem>>(&Some(item)));
        let Some(tab_item) = tab_item else {
            return;
        };
        let Some(content) = ferroui_base::metadata::from_markup_value::<Ref<Control>>(&tab_item.content()) else {
            return;
        };
        update_realized_count(&content);
    }
}
