//! Port of `App.xaml.cs` (the class of `App.xaml`): the main window with the classic desktop
//! lifetime, the main view with a single view, each with the view model of the main window.

use crate::markup::xaml_class;
use crate::view_models::MainWindowViewModel;
use crate::views::{MainView, MainWindow};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    Ref,
};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App: FerroObjectImpl);
xaml_class!(App, "/App.xaml");

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}

impl ApplicationImpl for App {
    /// `Initialize()`.
    fn initialize(this: &Self) {
        crate::register_types();
        this.initialize_component();
    }

    /// `OnFrameworkInitializationCompleted()`.
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let main_window = MainWindow::new();
            main_window.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));
            desktop.set_main_window(Some(main_window.upcast()));
        } else if let Some(single_view) = lifetime
            .as_ref()
            .and_then(|l| l.as_single_view_application_lifetime())
        {
            let main_view = MainView::new();
            main_view.set_data_context(Some(MainWindowViewModel::new() as BoxedValue));
            single_view.set_main_view(Some(main_view.upcast()));
        }

        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self {
            base: Application::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
