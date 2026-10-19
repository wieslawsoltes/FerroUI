//! `AppBuilderExtensions` (`AppBuilderExtensions.cs`): configures FerroUI applications to use
//! ReactiveUI.
//!
//! These extensions configure view registration, dependency injection integration and the services
//! of ReactiveUI for a FerroUI application. Call them during application startup, before the
//! platform services finish initialization.
//!
//! Deviation (DEVIATIONS.md, ReactiveUI controls): C#'s extension members are two traits,
//! [`AppBuilderExtensions`] on the application builder of `ferroui-controls` and
//! [`ReactiveUIBuilderFerroExtensions`] on ReactiveUI's builder. The `null` checks of the arguments
//! are the type system's. `RegisterReactiveUIViewsFromAssemblyOf<TMarker>` and the parameterless
//! `RegisterReactiveUIViewsFromEntryAssembly` find assemblies by reflection and are not ported;
//! views are declared as [`ViewAssembly`] lists ([`ViewRegistrar`]).

use crate::startup_configuration::StartupConfiguration;
use crate::view_registrar::{ViewAssembly, ViewRegistrar};
use crate::{register_binding_integration, AutoDataTemplateBindingHook, FerroActivationForViewFetcher, FerroScheduler};
use ferroui_controls::AppBuilder;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactiveui::{IActivationForViewFetcher, IPropertyBindingHook, ReactiveUIBuilder};
use ferroui_splat::{AppLocator, IDependencyResolver, IMutableDependencyResolver, MutableDependencyResolverExt};
use std::rc::Rc;

/// The scheduler of the user interface thread (`FerroScheduler.Instance`).
pub(crate) fn main_thread_scheduler() -> Rc<dyn IScheduler> {
    FerroScheduler::instance()
}

/// Extends the application builder of `ferroui-controls` with ReactiveUI (`AppBuilderExtensions`,
/// `extension(AppBuilder builder)`).
pub trait AppBuilderExtensions {
    /// Configures the application to use ReactiveUI with FerroUI (`UseReactiveUI`): after the
    /// platform services are set up, ReactiveUI is built with the services of FerroUI (activation,
    /// property observation, command binding, the scheduler of the user interface thread), and
    /// `with_reactive_ui_builder` can register more services or change the configuration before
    /// the application is built.
    fn use_reactive_ui(&self, with_reactive_ui_builder: impl Fn(&ReactiveUIBuilder) + 'static) -> AppBuilder;

    /// Registers the views of `assemblies` with the service locator after the platform services
    /// are set up (`RegisterReactiveUIViews`).
    fn register_reactive_ui_views(&self, assemblies: Vec<Rc<ViewAssembly>>) -> AppBuilder;

    /// Registers the views of the entry assembly when there is one (the internal
    /// `RegisterReactiveUIViewsFromEntryAssembly(Assembly?)`); the builder unchanged otherwise.
    fn register_reactive_ui_views_from_entry_assembly(&self, entry_assembly: Option<Rc<ViewAssembly>>) -> AppBuilder;

    /// Configures ReactiveUI with a dependency injection container and its resolver
    /// (`UseReactiveUIWithDIContainer<TContainer>`): ReactiveUI is configured as by
    /// [`use_reactive_ui`](Self::use_reactive_ui); then the container is created by
    /// `container_factory`, registered with the service locator, its resolver (from
    /// `dependency_resolver_factory`) becomes the one of the locator, and `container_config`
    /// configures it.
    fn use_reactive_ui_with_di_container<C: 'static>(
        &self,
        container_factory: impl Fn() -> Rc<C> + 'static,
        container_config: impl Fn(&Rc<C>) + 'static,
        dependency_resolver_factory: impl Fn(&Rc<C>) -> Rc<dyn IDependencyResolver> + 'static,
        configure_reactive_ui: impl Fn(&ReactiveUIBuilder) + 'static,
    ) -> AppBuilder;
}

impl AppBuilderExtensions for AppBuilder {
    fn use_reactive_ui(&self, with_reactive_ui_builder: impl Fn(&ReactiveUIBuilder) + 'static) -> AppBuilder {
        self.after_platform_services_setup(move |_| StartupConfiguration::configure_reactive_ui(&with_reactive_ui_builder))
    }

    fn register_reactive_ui_views(&self, assemblies: Vec<Rc<ViewAssembly>>) -> AppBuilder {
        self.after_platform_services_setup(move |_| {
            let resolver = AppLocator::current_mutable();
            ViewRegistrar::register_views(Some(&*resolver), Some(assemblies.as_slice()));
        })
    }

    fn register_reactive_ui_views_from_entry_assembly(&self, entry_assembly: Option<Rc<ViewAssembly>>) -> AppBuilder {
        match entry_assembly {
            None => self.clone(),
            Some(entry_assembly) => self.register_reactive_ui_views(vec![entry_assembly]),
        }
    }

    fn use_reactive_ui_with_di_container<C: 'static>(
        &self,
        container_factory: impl Fn() -> Rc<C> + 'static,
        container_config: impl Fn(&Rc<C>) + 'static,
        dependency_resolver_factory: impl Fn(&Rc<C>) -> Rc<dyn IDependencyResolver> + 'static,
        configure_reactive_ui: impl Fn(&ReactiveUIBuilder) + 'static,
    ) -> AppBuilder {
        self.use_reactive_ui(configure_reactive_ui).after_platform_services_setup(move |_| {
            StartupConfiguration::configure_reactive_ui_di_container(
                Some(AppLocator::current_mutable()),
                &container_factory,
                &container_config,
                &dependency_resolver_factory,
            );
        })
    }
}

/// Extends ReactiveUI's builder with the services of FerroUI (`extension(IReactiveUIBuilder
/// builder)`).
pub trait ReactiveUIBuilderFerroExtensions {
    /// Configures the builder to use the implementations of FerroUI (`WithFerro`): the
    /// scheduler of the user interface thread as the main thread scheduler, the task pool
    /// scheduler, the activation fetcher and the data template hook of FerroUI, the property
    /// observer, command binder and view thread invoker of the binding integration, and the
    /// suspension host.
    ///
    /// Deviation (DEVIATIONS.md, ReactiveUI controls): there is no thread pool; the task pool
    /// scheduler is the default scheduler of `ferroui-reactive`. Upstream's
    /// `UsingSplatModule(new FerroBindingModule())` is the registration function of the binding
    /// integration.
    fn with_ferro(&self) -> &Self;
}

impl ReactiveUIBuilderFerroExtensions for ReactiveUIBuilder {
    fn with_ferro(&self) -> &Self {
        // Deviation (DEVIATIONS.md, ReactiveUI controls): the core services are registered first,
        // so that the services of FerroUI registered below come after them and win; upstream
        // registers them when the application is built, after these registrations, unless they
        // were registered before.
        self.with_core_services()
            .with_main_thread_scheduler(main_thread_scheduler())
            .with_task_pool_scheduler(SchedulerDefaults::default_scheduler())
            .with_registration(|splat: &dyn IMutableDependencyResolver| {
                splat.register_constant::<dyn IActivationForViewFetcher>(Rc::new(FerroActivationForViewFetcher::new()));
                splat.register_constant::<dyn IPropertyBindingHook>(Rc::new(AutoDataTemplateBindingHook::new()));
                register_binding_integration(splat);
            })
            .with_suspension_host_of::<()>()
    }
}
