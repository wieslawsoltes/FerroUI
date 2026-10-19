//! `StartupConfiguration` (`StartupConfiguration.cs`): builds ReactiveUI once the platform
//! services of the application are ready.
//!
//! The application builder defers these calls until after the platform services are set up, so
//! the service locator and the scheduler of the user interface thread are both available by the
//! time they run.

use crate::app_builder_extensions::{main_thread_scheduler, ReactiveUIBuilderFerroExtensions};
use crate::FerroScheduler;
use ferroui_reactiveui::{RxAppBuilder, RxSchedulers, ReactiveUIBuilder};
use ferroui_splat::{AppBuilder as SplatAppBuilder, AppLocator, IDependencyResolver, IMutableDependencyResolver, MutableDependencyResolverExt};
use std::rc::Rc;

/// Builds ReactiveUI once the platform services of the application are ready
/// (`StartupConfiguration`).
pub struct StartupConfiguration;

impl StartupConfiguration {
    /// Configures and builds ReactiveUI after the platform services are ready
    /// (`ConfigureReactiveUI`): a builder with the services of FerroUI
    /// ([`with_ferro`](ReactiveUIBuilderFerroExtensions::with_ferro)), then the application's
    /// configuration, then the build, unless an application was built already.
    ///
    /// The synchronization context of the dispatcher becomes the one of the user interface thread
    /// ([`FerroScheduler::install_on_ui_thread`]).
    pub fn configure_reactive_ui(configure_reactive_ui: &dyn Fn(&ReactiveUIBuilder)) {
        FerroScheduler::install_on_ui_thread();

        let rxui_builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = rxui_builder.with_ferro();

        configure_reactive_ui(&rxui_builder);

        if SplatAppBuilder::has_been_built() {
            return;
        }

        let _ = rxui_builder.build_app();
    }

    /// Configures ReactiveUI with a dependency injection container when a mutable resolver is
    /// available (`ConfigureReactiveUIDIContainer`): the container is created, registered as a
    /// constant, its resolver becomes the one of the locator, the scheduler of the user interface
    /// thread becomes the main thread scheduler, and the container is configured.
    pub fn configure_reactive_ui_di_container<C: 'static>(
        resolver: Option<Rc<dyn IMutableDependencyResolver>>,
        container_factory: &dyn Fn() -> Rc<C>,
        container_config: &dyn Fn(&Rc<C>),
        dependency_resolver_factory: &dyn Fn(&Rc<C>) -> Rc<dyn IDependencyResolver>,
    ) {
        let Some(resolver) = resolver else {
            return;
        };

        let container = container_factory();
        resolver.register_constant::<C>(container.clone());
        let dependency_resolver = dependency_resolver_factory(&container);
        AppLocator::set_locator(dependency_resolver);
        RxSchedulers::set_main_thread_scheduler(main_thread_scheduler());
        container_config(&container);
    }
}
