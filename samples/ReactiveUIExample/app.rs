//! Port of `App.xaml.cs`: the class of the document `App.xaml`, the example application.

use crate::markup::xaml_class;
use crate::services::{BrowserMachineMetricsService, ILocalMachineMetricsService, InvalidOperationError, LocalMachineMetricsService, ShowcaseSession};
use crate::view_models::{CommandLabViewModel, DynamicDataViewModel, FeatureViewModel, MainViewModel, MetricCardViewModel, MetricsViewModel, OverviewViewModel};
use crate::views::{
    CommandLabView, DynamicDataView, FeatureView, MainView, MainWindow, MemoryMetricCardView, MetricsView, OverviewView,
    PerformanceMetricCardView,
};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::application_lifetimes::{ControlledApplicationLifetimeExitEventArgs, IApplicationLifetime};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use ferroui_reactiveui::{RegisterViewForExt, ViewContract, ViewRegistrationOptions, ViewRegistrations};
use ferroui_splat::{AppLocator, IMutableDependencyResolver, IReadonlyDependencyResolver, MutableDependencyResolverExt, ReadonlyDependencyResolverExt};
use ferroui_system::{IDisposable, ObservableError};
use std::rc::Rc;

/// The view contract of the metric card of the processor (`"performance"`).
pub const PERFORMANCE_CONTRACT: &str = "performance";

/// The view contract of the metric cards of memory (`"memory"`).
pub const MEMORY_CONTRACT: &str = "memory";

/// The example application (`App`).
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
    /// `Initialize()`: the types of the crate, then the document of the application.
    fn initialize(this: &Self) {
        crate::register_types();
        this.initialize_component();
    }

    /// `OnFrameworkInitializationCompleted()`: the shell is given to the lifetime.
    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        Self::attach_shell(lifetime.as_deref());
        Self::parent_on_framework_initialization_completed(this);
    }
}

impl App {
    /// Field initialisation.
    pub fn construct() -> Self {
        Self { base: Application::construct() }
    }

    /// Creates the application.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Registers the view and service dependencies of the example (`RegisterViews`): the metrics
    /// service of the platform as a lazy singleton, and each view as `IViewFor<TViewModel>`, the
    /// metric cards under their contracts.
    ///
    /// Deviation (DEVIATIONS.md, samples): the hosts of the port resolve a view through the mappings
    /// of the view locator (`IViewLocator::resolve_view`), not through the service locator, so the
    /// views are also registered as the view dispatch of the locator ([`ViewRegistrations`]), the
    /// counterpart of the dispatch upstream's source generator writes.
    pub fn register_views(resolver: &dyn IMutableDependencyResolver) {
        resolver.register_lazy_singleton::<dyn ILocalMachineMetricsService>(|| Some(Self::create_metrics_service(cfg!(target_arch = "wasm32"))));
        resolver.register_view_for::<OverviewViewModel>(|| OverviewView::view_for_of(&OverviewView::new()), None);
        resolver.register_view_for::<MetricsViewModel>(|| MetricsView::view_for_of(&MetricsView::new()), None);
        resolver.register_view_for::<CommandLabViewModel>(|| CommandLabView::view_for_of(&CommandLabView::new()), None);
        resolver.register_view_for::<FeatureViewModel>(|| FeatureView::view_for_of(&FeatureView::new()), None);
        resolver.register_view_for::<MetricCardViewModel>(
            || PerformanceMetricCardView::view_for_of(&PerformanceMetricCardView::new()),
            Some(PERFORMANCE_CONTRACT),
        );
        resolver.register_view_for::<MetricCardViewModel>(|| MemoryMetricCardView::view_for_of(&MemoryMetricCardView::new()), Some(MEMORY_CONTRACT));
        // The DynamicData page is an addition of the port.
        resolver.register_view_for::<DynamicDataViewModel>(|| DynamicDataView::view_for_of(&DynamicDataView::new()), None);

        ViewRegistrations::new()
            .register_with::<OverviewViewModel, _>(|| Rc::new(OverviewView::typed(&OverviewView::new())), ViewRegistrationOptions::new())
            .register_with::<MetricsViewModel, _>(|| Rc::new(MetricsView::typed(&MetricsView::new())), ViewRegistrationOptions::new())
            .register_with::<CommandLabViewModel, _>(|| Rc::new(CommandLabView::typed(&CommandLabView::new())), ViewRegistrationOptions::new())
            .register_with::<FeatureViewModel, _>(|| Rc::new(FeatureView::typed(&FeatureView::new())), ViewRegistrationOptions::new())
            .register_with::<MetricCardViewModel, _>(
                || Rc::new(PerformanceMetricCardView::typed(&PerformanceMetricCardView::new())),
                ViewRegistrationOptions::new().with_contract(ViewContract::new(PERFORMANCE_CONTRACT)),
            )
            .register_with::<MetricCardViewModel, _>(
                || Rc::new(MemoryMetricCardView::typed(&MemoryMetricCardView::new())),
                ViewRegistrationOptions::new().with_contract(ViewContract::new(MEMORY_CONTRACT)),
            )
            .register_with::<DynamicDataViewModel, _>(|| Rc::new(DynamicDataView::typed(&DynamicDataView::new())), ViewRegistrationOptions::new())
            .apply();
    }

    /// Creates the metrics service for the platform (`CreateMetricsService`): the browser service in
    /// a browser, which cannot read the measurements of a process, otherwise the local service.
    pub fn create_metrics_service(is_browser: bool) -> Rc<dyn ILocalMachineMetricsService> {
        if is_browser {
            Rc::new(BrowserMachineMetricsService::new())
        } else {
            Rc::new(LocalMachineMetricsService::new())
        }
    }

    /// Creates the shell and gives it to the lifetime (`AttachShell`): a window on desktop, with the
    /// session of the example persisted; a single view elsewhere, without a session; nothing without
    /// a lifetime.
    ///
    /// # Panics
    /// Panics when the local metrics service has not been registered (`InvalidOperationException`).
    pub fn attach_shell(lifetime: Option<&dyn IApplicationLifetime>) {
        let Some(lifetime) = lifetime else {
            return;
        };

        if let Some(desktop) = lifetime.as_classic_desktop_style_application_lifetime() {
            let shell = Self::create_shell_or_panic();
            let session = ShowcaseSession::new(lifetime, shell.clone(), ShowcaseSession::default_path());
            let on_exit = session.clone();
            let _ = desktop.exit(Rc::new(move |_: &ControlledApplicationLifetimeExitEventArgs| on_exit.dispose()));
            session.start();
            let window = MainWindow::new();
            MainWindow::typed(&window).set_view_model(Some(shell));
            desktop.set_main_window(Some(window.upcast()));
            return;
        }

        if let Some(single_view) = lifetime.as_single_view_application_lifetime() {
            let shell = Self::create_shell_or_panic();

            // A single-view lifetime has no exit event, so the session is not persisted.
            shell.set_session_status(ShowcaseSession::UNAVAILABLE_STATUS.to_owned());
            let view = MainView::new();
            MainView::typed(&view).set_view_model(Some(shell));
            single_view.set_main_view(Some(view.upcast()));
        }
    }

    /// Creates the shell view model from the registered metrics service (`CreateShell`); an
    /// [`InvalidOperationError`] when the service has not been registered.
    pub fn create_shell(resolver: &dyn IReadonlyDependencyResolver) -> Result<Rc<MainViewModel>, ObservableError> {
        let metrics_service = resolver
            .get_service::<dyn ILocalMachineMetricsService>()
            .ok_or_else(|| InvalidOperationError::new("The local metrics service has not been registered.").into_error())?;
        Ok(MainViewModel::create(metrics_service))
    }

    /// [`create_shell`](Self::create_shell) from the current locator, which panics as the
    /// exception of the reference escapes `AttachShell`.
    fn create_shell_or_panic() -> Rc<MainViewModel> {
        match Self::create_shell(&*AppLocator::current()) {
            Ok(shell) => shell,
            Err(error) => panic!("InvalidOperationException: {error}"),
        }
    }
}
