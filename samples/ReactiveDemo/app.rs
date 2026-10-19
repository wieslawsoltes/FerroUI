//! Port of `App.xaml.cs`: the class of the document `App.xaml`, which registers the views of the
//! sample for their view models.

use crate::markup::xaml_class;
use crate::{MainWindow, NugetDetailsView, NugetDetailsViewFor, NugetDetailsViewModel};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use ferroui_reactiveui_controls::{ViewAssembly, ViewRegistrar};
use ferroui_splat::AppLocator;
use std::rc::Rc;

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
    fn initialize(this: &Self) {
        crate::register_types();
        this.initialize_component();

        // A helper method that will register all classes that derive off IViewFor into our
        // dependency injection container. ReactiveUI uses Splat for its dependency injection by
        // default, but you can override this if you like.
        register_views();
    }

    // Deviation (DEVIATIONS.md, samples): `StartupUri="MainWindow.xaml"` of the reference's
    // document; the main window is the one of the desktop lifetime.
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            desktop.set_main_window(Some(MainWindow::new().upcast()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    pub fn construct() -> Self {
        Self { base: Application::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// The views of the sample, each with the type of its view model (`Assembly.GetCallingAssembly()`
/// as `RegisterViewsForViewModels` scans it).
///
/// Deviation (DEVIATIONS.md, samples): upstream finds the classes that implement
/// `IViewFor<TViewModel>` by reflection; Rust has none, so the sample lists its one view, as
/// `ViewRegistrar` of the integration takes it. `MainWindow` is not located: the reference does
/// not resolve it either (it is the startup window).
pub fn view_assembly() -> ViewAssembly {
    ViewAssembly::new("ReactiveDemo").view::<NugetDetailsViewModel, NugetDetailsViewFor>(None, || NugetDetailsView::new().view())
}

/// Registers the views of the sample with the service locator
/// (`Locator.CurrentMutable.RegisterViewsForViewModels(Assembly.GetCallingAssembly())`): the
/// view host of each item of the list then finds the view of its view model.
pub fn register_views() {
    let resolver = AppLocator::current_mutable();
    let assemblies = [Rc::new(view_assembly())];
    ViewRegistrar::register_views(Some(&*resolver), Some(&assemblies[..]));
}
