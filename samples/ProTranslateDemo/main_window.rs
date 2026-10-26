//! The compiled XAML window and its live culture/measurement interaction.

use crate::markup::xaml_class;
use crate::MainWindowViewModel;
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
use ferroui_protranslate::TranslationBindingSource;
use ferroui_protranslate_core::Culture;
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
    view_model: Rc<MainWindowViewModel>,
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
    new: MainWindow::new
});
xaml_class!(MainWindow, "/MainWindow.xaml", create: MainWindow::new(), uninitialized: MainWindow::before_document());

impl MainWindow {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
            view_model: MainWindowViewModel::new(TranslationBindingSource::default_source()),
        }
    }

    fn before_document() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_data_context(Some(this.view_model.clone()));
        this
    }

    pub fn view_model(&self) -> Rc<MainWindowViewModel> {
        self.view_model.clone()
    }

    pub fn new() -> Ref<Self> {
        let this = Self::before_document();
        this.initialize_component();
        let source = TranslationBindingSource::default_source();
        for (button, culture) in [
            ("English", "en-US"),
            ("Polish", "pl-PL"),
            ("Arabic", "ar-SA"),
        ] {
            let source = source.clone();
            this.find_control::<Button>(button)
                .expect("language button")
                .click(move |_, _| {
                    source.set_culture(Culture::parse(culture).expect("built-in culture"));
                });
        }
        this
    }
}
