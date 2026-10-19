//! `ReactiveUIBuilder` (`ReactiveUI.Shared/Builder/ReactiveUIBuilder.cs`).

use super::{IReactiveUIBuilder, IReactiveUIInstance, ResolverAction};
use crate::{
    register_default_converters, ConverterService, DefaultViewLocator, DependencyResolverRegistrar, IBindingFallbackConverter,
    IBindingTypeConverter, IBindingTypeConverterOf, IMessageBus, IReactiveObject, ISetMethodBindingConverter, ISuspensionDriver,
    ISuspensionHost, IViewForOf, IViewLocator, IWantsToRegisterStuff, MessageBus, PlatformRegistrations, ReactiveUIBindingModule,
    Registrations, RxSchedulers, SuspensionHost, SuspensionHostOf,
};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_splat::service_location::initialize_splat;
use ferroui_splat::{
    AppBuilder, IAppBuilder, IAppInstance, IModule, IMutableDependencyResolver, IReadonlyDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
};
use ferroui_system::{IObserver, ObservableError};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// The default limit of the small caches on mobile targets (`DefaultSmallCacheLimit`).
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(super) const DEFAULT_SMALL_CACHE_LIMIT: usize = 32;
/// The default limit of the big caches on mobile targets (`DefaultBigCacheLimit`).
#[cfg(any(target_os = "android", target_os = "ios"))]
pub(super) const DEFAULT_BIG_CACHE_LIMIT: usize = 64;
/// The default limit of the small caches (`DefaultSmallCacheLimit`).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(super) const DEFAULT_SMALL_CACHE_LIMIT: usize = 64;
/// The default limit of the big caches (`DefaultBigCacheLimit`).
#[cfg(not(any(target_os = "android", target_os = "ios")))]
pub(super) const DEFAULT_BIG_CACHE_LIMIT: usize = 256;

/// The configuration of a builder, shared with the registrations it applies when the application
/// is built.
pub(super) struct BuilderState {
    /// `_platformRegistered`.
    pub(super) platform_registered: Cell<bool>,
    /// `_coreRegistered`.
    pub(super) core_registered: Cell<bool>,
    /// `_setRxAppMainScheduler`.
    pub(super) set_rx_app_main_scheduler: Cell<bool>,
    /// `_setRxAppTaskPoolScheduler`.
    pub(super) set_rx_app_task_pool_scheduler: Cell<bool>,
    /// `_exceptionHandler`.
    pub(super) exception_handler: RefCell<Option<Rc<dyn IObserver<ObservableError>>>>,
    /// `_suspensionHost`.
    pub(super) suspension_host: RefCell<Option<Rc<dyn ISuspensionHost>>>,
    /// `_smallCacheLimit`.
    pub(super) small_cache_limit: Cell<Option<usize>>,
    /// `_bigCacheLimit`.
    pub(super) big_cache_limit: Cell<Option<usize>>,
    /// `_messageBus`.
    pub(super) message_bus: RefCell<Option<Rc<dyn IMessageBus>>>,
    /// `MainThreadScheduler`.
    pub(super) main_thread_scheduler: RefCell<Option<Rc<dyn IScheduler>>>,
    /// `TaskpoolScheduler`.
    pub(super) task_pool_scheduler: RefCell<Option<Rc<dyn IScheduler>>>,
}

/// The builder of a ReactiveUI application (`ReactiveUIBuilder`): Splat's [`AppBuilder`] with the
/// services, schedulers, converters and state of ReactiveUI.
///
/// ```
/// use ferroui_reactiveui::RxAppBuilder;
///
/// RxAppBuilder::reset_for_testing();
/// let builder = RxAppBuilder::create_reactive_ui_builder();
/// builder.with_core_services().with_cache_sizes(32, 128).build_app();
/// RxAppBuilder::ensure_initialized();
/// ```
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream derives the class from Splat's `AppBuilder` and
/// overrides its virtual `WithCoreServices`; the builder here holds an [`AppBuilder`] and
/// implements [`IAppBuilder`] itself, registering its core services before the inner builder
/// applies the registrations. Each method returns the builder, to chain the calls on the concrete
/// type, where upstream returns the interface; the same methods, returning trait objects, are
/// those of [`IReactiveUIBuilder`].
pub struct ReactiveUIBuilder {
    pub(super) app: Rc<AppBuilder>,
    pub(super) state: Rc<BuilderState>,
    /// `ConverterService`.
    pub(super) converter_service: Rc<ConverterService>,
}

impl fmt::Debug for ReactiveUIBuilder {
    /// `[DebuggerDisplay("ReactiveUIBuilder")]`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReactiveUIBuilder")
    }
}

impl ReactiveUIBuilder {
    /// A builder over `resolver`, with `current` as the resolver of the application
    /// (`ReactiveUIBuilder(IMutableDependencyResolver, IReadonlyDependencyResolver?)`): it
    /// initializes Splat's services in the resolver and registers its converter service there.
    pub fn new(resolver: Rc<dyn IMutableDependencyResolver>, current: Option<Rc<dyn IReadonlyDependencyResolver>>) -> Self {
        let app = Rc::new(AppBuilder::with_current(resolver, current));
        let converter_service = Rc::new(ConverterService::new());
        let builder = Self {
            app,
            state: Rc::new(BuilderState {
                platform_registered: Cell::new(false),
                core_registered: Cell::new(false),
                set_rx_app_main_scheduler: Cell::new(false),
                set_rx_app_task_pool_scheduler: Cell::new(false),
                exception_handler: RefCell::new(None),
                suspension_host: RefCell::new(None),
                small_cache_limit: Cell::new(None),
                big_cache_limit: Cell::new(None),
                message_bus: RefCell::new(None),
                main_thread_scheduler: RefCell::new(None),
                task_pool_scheduler: RefCell::new(None),
            }),
            converter_service,
        };
        let current_mutable = builder.current_mutable();
        initialize_splat(&*current_mutable);
        current_mutable.register_constant::<ConverterService>(builder.converter_service.clone());
        builder
    }

    /// The service of the converters of the bindings (`ConverterService`): its typed, fallback and
    /// set-method registries, which the `with_*converter*` methods fill.
    pub fn converter_service(&self) -> &Rc<ConverterService> {
        &self.converter_service
    }

    /// The scheduler of the user-interface thread the builder was given (`MainThreadScheduler`).
    pub fn main_thread_scheduler(&self) -> Option<Rc<dyn IScheduler>> {
        self.state.main_thread_scheduler.borrow().clone()
    }

    /// The scheduler of background work the builder was given (`TaskpoolScheduler`).
    pub fn task_pool_scheduler(&self) -> Option<Rc<dyn IScheduler>> {
        self.state.task_pool_scheduler.borrow().clone()
    }

    /// The resolver of the application, once known (`Current`).
    pub fn current(&self) -> Option<Rc<dyn IReadonlyDependencyResolver>> {
        self.app.current()
    }

    /// The resolver the builder registers with (`CurrentMutable`).
    pub fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver> {
        self.app.current_mutable()
    }

    /// Sets the scheduler of the user-interface thread, which also becomes
    /// `RxSchedulers::main_thread_scheduler` when the application is built
    /// (`WithMainThreadScheduler(ISequencer)`).
    pub fn with_main_thread_scheduler(&self, scheduler: Rc<dyn IScheduler>) -> &Self {
        self.with_main_thread_scheduler_with(scheduler, true)
    }

    /// Sets the scheduler of the user-interface thread; `set_rx_app` says whether it becomes
    /// `RxSchedulers::main_thread_scheduler` (`WithMainThreadScheduler(ISequencer, bool)`).
    pub fn with_main_thread_scheduler_with(&self, scheduler: Rc<dyn IScheduler>, set_rx_app: bool) -> &Self {
        self.state.set_rx_app_main_scheduler.set(set_rx_app);
        *self.state.main_thread_scheduler.borrow_mut() = Some(scheduler);
        self
    }

    /// Sets the scheduler of background work, which also becomes
    /// `RxSchedulers::task_pool_scheduler` when the application is built
    /// (`WithTaskPoolScheduler(ISequencer)`).
    pub fn with_task_pool_scheduler(&self, scheduler: Rc<dyn IScheduler>) -> &Self {
        self.with_task_pool_scheduler_with(scheduler, true)
    }

    /// Sets the scheduler of background work; `set_rx_app` says whether it becomes
    /// `RxSchedulers::task_pool_scheduler` (`WithTaskPoolScheduler(ISequencer, bool)`).
    pub fn with_task_pool_scheduler_with(&self, scheduler: Rc<dyn IScheduler>, set_rx_app: bool) -> &Self {
        self.state.set_rx_app_task_pool_scheduler.set(set_rx_app);
        *self.state.task_pool_scheduler.borrow_mut() = Some(scheduler);
        self
    }

    /// Applies `configure_action` to the resolver when the application is built
    /// (`WithRegistrationOnBuild(Action<IMutableDependencyResolver>)`).
    pub fn with_registration_on_build(&self, configure_action: impl Fn(&dyn IMutableDependencyResolver) + 'static) -> &Self {
        let _ = self.app.with_custom_registration(Rc::new(configure_action));
        self
    }

    /// Adds registrations, applied to the resolver when the application is built
    /// (`WithCustomRegistration(Action<IMutableDependencyResolver>)` of Splat's builder).
    pub fn with_custom_registration(&self, configure_action: impl Fn(&dyn IMutableDependencyResolver) + 'static) -> &Self {
        let _ = self.app.with_custom_registration(Rc::new(configure_action));
        self
    }

    /// Applies `configure_action` to the resolver at once (`WithRegistration(Action<IMutableDependencyResolver>)`).
    pub fn with_registration(&self, configure_action: impl FnOnce(&dyn IMutableDependencyResolver)) -> &Self {
        configure_action(&*self.current_mutable());
        self
    }

    /// Applies `registration` to the resolver at once, through a [`DependencyResolverRegistrar`]
    /// (`WithRegistration(IWantsToRegisterStuff)`), the migration path of the types that
    /// registered their services by assembly scanning.
    pub fn with_registrations(&self, registration: &dyn IWantsToRegisterStuff) -> &Self {
        let registrar = DependencyResolverRegistrar::new(self.current_mutable());
        registration.register(&registrar);
        self
    }

    /// Registers the services of the platform, once (`WithPlatformServices`).
    pub fn with_platform_services(&self) -> &Self {
        if self.state.platform_registered.replace(true) {
            return self;
        }

        self.with_platform_module::<PlatformRegistrations>()
    }

    /// Registers the core services of ReactiveUI, once: the default converters, the property
    /// observers, the view locator and the activation fetcher; and, each time, the scheduler
    /// configuration applied when the application is built (`WithCoreServices`).
    pub fn with_core_services(&self) -> &Self {
        if !self.state.core_registered.get() {
            register_default_converters(&self.converter_service);
            ReactiveUIBindingModule.configure(&*self.current_mutable());

            let _ = self.with_platform_module::<Registrations>();
            self.state.core_registered.set(true);
        }

        self.configure_schedulers();
        self
    }

    /// Applies the registrations of a new `T` at once (`WithPlatformModule<T>()`).
    pub fn with_platform_module<T: IWantsToRegisterStuff + Default>(&self) -> &Self {
        self.with_registrations(&T::default())
    }

    /// Adds a Splat module, applied when the application is built (`UsingSplatModule<T>(T)`).
    pub fn using_splat_module(&self, registration_module: Rc<dyn IModule>) -> &Self {
        let _ = self.app.using_module(registration_module);
        self
    }

    /// Calls `app_builder`, if any, with the builder as a Splat builder
    /// (`UsingSplatBuilder(Action<IAppBuilder>)`).
    pub fn using_splat_builder(&self, app_builder: Option<&dyn Fn(&dyn IAppBuilder)>) -> &Self {
        if let Some(app_builder) = app_builder {
            app_builder(self as &dyn IAppBuilder);
        }
        self
    }

    /// Configures a platform of the application's own: its main thread scheduler, and its
    /// services, registered when the application is built
    /// (`ForCustomPlatform(ISequencer, Action<IMutableDependencyResolver>)`).
    pub fn for_custom_platform(
        &self,
        main_thread_scheduler: Rc<dyn IScheduler>,
        platform_services: impl Fn(&dyn IMutableDependencyResolver) + 'static,
    ) -> &Self {
        self.with_main_thread_scheduler(main_thread_scheduler).with_registration_on_build(platform_services)
    }

    /// Applies the configuration of each platform (`ForPlatforms(params Action<IReactiveUIBuilder>[])`).
    pub fn for_platforms(&self, platform_configurations: &[&dyn Fn(&dyn IReactiveUIBuilder)]) -> &Self {
        for configure_platform in platform_configurations {
            configure_platform(self as &dyn IReactiveUIBuilder);
        }
        self
    }

    /// Registers a new message bus when the application is built (`WithMessageBus()`).
    pub fn with_message_bus(&self) -> &Self {
        let state = self.state.clone();
        self.with_registration_on_build(move |resolver: &dyn IMutableDependencyResolver| {
            let message_bus: Rc<dyn IMessageBus> = MessageBus::new();
            *state.message_bus.borrow_mut() = Some(message_bus.clone());
            resolver.register_constant::<dyn IMessageBus>(message_bus);
        })
    }

    /// Registers a new message bus, configured by `configure`, when the application is built
    /// (`WithMessageBus(Action<IMessageBus>)`).
    pub fn with_message_bus_with(&self, configure: impl Fn(&dyn IMessageBus) + 'static) -> &Self {
        let state = self.state.clone();
        self.with_registration_on_build(move |resolver: &dyn IMutableDependencyResolver| {
            let message_bus: Rc<dyn IMessageBus> = MessageBus::new();
            *state.message_bus.borrow_mut() = Some(message_bus.clone());
            configure(&*message_bus);
            resolver.register_constant::<dyn IMessageBus>(message_bus);
        })
    }

    /// Registers `message_bus` when the application is built (`WithMessageBus(IMessageBus)`).
    pub fn with_message_bus_instance(&self, message_bus: Rc<dyn IMessageBus>) -> &Self {
        *self.state.message_bus.borrow_mut() = Some(message_bus.clone());
        self.with_registration_on_build(move |resolver: &dyn IMutableDependencyResolver| {
            resolver.register_constant::<dyn IMessageBus>(message_bus.clone());
        })
    }

    /// Configures the registered [`DefaultViewLocator`] when the application is built, keeping the
    /// mappings that `register_views` or `with_view_module` added; with none registered, a new one
    /// is configured and registered as the view locator
    /// (`ConfigureViewLocator(Action<DefaultViewLocator>)`).
    pub fn configure_view_locator(&self, configure: impl Fn(&DefaultViewLocator) + 'static) -> &Self {
        let app = Rc::downgrade(&self.app);
        self.with_registration_on_build(move |resolver: &dyn IMutableDependencyResolver| {
            let registered = app
                .upgrade()
                .and_then(|app| app.current())
                .and_then(|current| current.get_service::<dyn IViewLocator>())
                .and_then(|locator| (locator as Rc<dyn Any>).downcast::<DefaultViewLocator>().ok());
            if let Some(registered) = registered {
                configure(&*registered);
                return;
            }

            let view_locator = Rc::new(DefaultViewLocator::new());
            configure(&*view_locator);
            resolver.register_constant::<dyn IViewLocator>(view_locator);
        })
    }

    /// Configures the registered suspension driver, if there is one, when the application is built
    /// (`ConfigureSuspensionDriver(Action<ISuspensionDriver>)`).
    pub fn configure_suspension_driver(&self, configure: impl Fn(&Rc<dyn ISuspensionDriver>) + 'static) -> &Self {
        let app = Rc::downgrade(&self.app);
        self.with_registration_on_build(move |_resolver: &dyn IMutableDependencyResolver| {
            let current_driver = app
                .upgrade()
                .and_then(|app| app.current())
                .and_then(|current| current.get_service::<dyn ISuspensionDriver>());
            let Some(current_driver) = current_driver else { return };
            configure(&current_driver);
        })
    }

    /// Registers a converter of a pair of types (`WithConverter(IBindingTypeConverter)`).
    pub fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> &Self {
        self.converter_service.typed_converters().register(converter);
        self
    }

    /// Registers a converter of `F` to `T` (`WithConverter<TFrom, TTo>(IBindingTypeConverter<TFrom, TTo>)`).
    pub fn with_converter_of<F: 'static, T: 'static>(&self, converter: Rc<dyn IBindingTypeConverterOf<F, T>>) -> &Self {
        self.with_converter(converter as Rc<dyn IBindingTypeConverter>)
    }

    /// Registers the converter that `factory` creates (`WithConverter(Func<IBindingTypeConverter>)`).
    pub fn with_converter_factory(&self, factory: impl FnOnce() -> Rc<dyn IBindingTypeConverter>) -> &Self {
        self.with_converter(factory())
    }

    /// Registers the converter of `F` to `T` that `factory` creates
    /// (`WithConverter<TFrom, TTo>(Func<IBindingTypeConverter<TFrom, TTo>>)`).
    pub fn with_converter_of_factory<F: 'static, T: 'static>(
        &self,
        factory: impl FnOnce() -> Rc<dyn IBindingTypeConverterOf<F, T>>,
    ) -> &Self {
        self.with_converter(factory() as Rc<dyn IBindingTypeConverter>)
    }

    /// Registers a fallback converter, asked when no converter of the exact pair of types is
    /// found (`WithFallbackConverter(IBindingFallbackConverter)`).
    pub fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> &Self {
        self.converter_service.fallback_converters().register(converter);
        self
    }

    /// Registers the fallback converter that `factory` creates
    /// (`WithFallbackConverter(Func<IBindingFallbackConverter>)`).
    pub fn with_fallback_converter_factory(&self, factory: impl FnOnce() -> Rc<dyn IBindingFallbackConverter>) -> &Self {
        self.with_fallback_converter(factory())
    }

    /// Registers a set-method converter, for the targets set through a method
    /// (`WithSetMethodConverter(ISetMethodBindingConverter)`).
    pub fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> &Self {
        self.converter_service.set_method_converters().register(converter);
        self
    }

    /// Registers the set-method converter that `factory` creates
    /// (`WithSetMethodConverter(Func<ISetMethodBindingConverter>)`).
    pub fn with_set_method_converter_factory(&self, factory: impl FnOnce() -> Rc<dyn ISetMethodBindingConverter>) -> &Self {
        self.with_set_method_converter(factory())
    }

    /// Registers the converters of the three kinds that `resolver` has, a migration helper from
    /// the registration in Splat (`WithConvertersFrom(IReadonlyDependencyResolver)`).
    pub fn with_converters_from(&self, resolver: &dyn IReadonlyDependencyResolver) -> &Self {
        for converter in resolver.get_services::<dyn IBindingTypeConverter>() {
            self.converter_service.typed_converters().register(converter);
        }

        for converter in resolver.get_services::<dyn IBindingFallbackConverter>() {
            self.converter_service.fallback_converters().register(converter);
        }

        for converter in resolver.get_services::<dyn ISetMethodBindingConverter>() {
            self.converter_service.set_method_converters().register(converter);
        }

        self
    }

    /// Sets the handler of the errors that no `thrown_exceptions` subscriber handles
    /// (`WithExceptionHandler(IObserver<Exception>)`).
    pub fn with_exception_handler(&self, exception_handler: Rc<dyn IObserver<ObservableError>>) -> &Self {
        *self.state.exception_handler.borrow_mut() = Some(exception_handler);
        self
    }

    /// Sets a suspension host, which becomes the one of `RxSuspension` when the application is
    /// built (`WithSuspensionHost()`).
    pub fn with_suspension_host(&self) -> &Self {
        self.with_suspension_host_instance(SuspensionHost::new())
    }

    /// Sets a suspension host of an application state of type `T`
    /// (`WithSuspensionHost<TAppState>()`).
    pub fn with_suspension_host_of<T: 'static>(&self) -> &Self {
        self.with_suspension_host_instance(SuspensionHostOf::<T>::new())
    }

    /// Sets `suspension_host` as the suspension host, the common part of
    /// [`with_suspension_host`](Self::with_suspension_host) and
    /// [`with_suspension_host_of`](Self::with_suspension_host_of).
    pub fn with_suspension_host_instance(&self, suspension_host: Rc<dyn ISuspensionHost>) -> &Self {
        *self.state.suspension_host.borrow_mut() = Some(suspension_host);
        self
    }

    /// Sets the limits of the caches of ReactiveUI (`WithCacheSizes(int, int)`).
    ///
    /// # Panics
    /// When a limit is zero (`ArgumentOutOfRangeException`).
    pub fn with_cache_sizes(&self, small_cache_limit: usize, big_cache_limit: usize) -> &Self {
        assert!(small_cache_limit > 0, "ArgumentOutOfRangeException: smallCacheLimit must be greater than zero");
        assert!(big_cache_limit > 0, "ArgumentOutOfRangeException: bigCacheLimit must be greater than zero");

        self.state.small_cache_limit.set(Some(small_cache_limit));
        self.state.big_cache_limit.set(Some(big_cache_limit));
        self
    }

    /// Registers `VM`, a new view model at each resolution (`RegisterViewModel<TViewModel>()`).
    pub fn register_view_model<VM: IReactiveObject + Default>(&self) -> &Self {
        self.with_registration(|resolver| resolver.register::<VM>(|| Some(Rc::new(VM::default()))))
    }

    /// Registers one view model of `VM`, created now, as a constant
    /// (`RegisterConstantViewModel<TViewModel>()`).
    pub fn register_constant_view_model<VM: IReactiveObject + Default>(&self) -> &Self {
        self.with_registration(|resolver| resolver.register_constant::<VM>(Rc::new(VM::default())))
    }

    /// Registers `VM`, one view model created at the first resolution
    /// (`RegisterSingletonViewModel<TViewModel>()`).
    pub fn register_singleton_view_model<VM: IReactiveObject + Default>(&self) -> &Self {
        self.with_registration(|resolver| resolver.register_lazy_singleton::<VM>(|| Some(Rc::new(VM::default()))))
    }

    /// Registers `V` as the view of `VM`, a new view at each resolution
    /// (`RegisterView<TView, TViewModel>()`).
    pub fn register_view<V: IViewForOf<VM> + Default + 'static, VM: IReactiveObject>(&self) -> &Self {
        self.with_registration(|resolver| {
            resolver.register::<dyn IViewForOf<VM>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>));
        })
    }

    /// Registers `V` as the view of `VM`, one view created at the first resolution
    /// (`RegisterSingletonView<TView, TViewModel>()`).
    pub fn register_singleton_view<V: IViewForOf<VM> + Default + 'static, VM: IReactiveObject>(&self) -> &Self {
        self.with_registration(|resolver| {
            resolver.register_lazy_singleton::<dyn IViewForOf<VM>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>));
        })
    }

    /// `InitializeStaticState`: the limits of the caches, the handler of the errors and the
    /// suspension host.
    pub(super) fn initialize_static_state(&self) {
        crate::RxCacheSize::initialize(
            self.state.small_cache_limit.get().unwrap_or(DEFAULT_SMALL_CACHE_LIMIT),
            self.state.big_cache_limit.get().unwrap_or(DEFAULT_BIG_CACHE_LIMIT),
        );

        if let Some(exception_handler) = self.state.exception_handler.borrow().clone() {
            crate::RxState::initialize_exception_handler(exception_handler);
        }

        let Some(suspension_host) = self.state.suspension_host.borrow().clone() else {
            return;
        };
        crate::RxSuspension::initialize_suspension_host(suspension_host);
    }

    /// `ConfigureSchedulers`: when the application is built, the schedulers the builder was given
    /// become those of [`RxSchedulers`], unless the builder was told not to set them.
    fn configure_schedulers(&self) {
        let state = self.state.clone();
        let _ = self.with_custom_registration(move |_resolver: &dyn IMutableDependencyResolver| {
            if let Some(scheduler) = state.main_thread_scheduler.borrow().clone() {
                if state.set_rx_app_main_scheduler.get() {
                    RxSchedulers::set_main_thread_scheduler(scheduler);
                }
            }

            let Some(scheduler) = state.task_pool_scheduler.borrow().clone() else { return };
            if !state.set_rx_app_task_pool_scheduler.get() {
                return;
            }
            RxSchedulers::set_task_pool_scheduler(scheduler);
        });
    }
}

impl IAppInstance for ReactiveUIBuilder {
    fn current(&self) -> Option<Rc<dyn IReadonlyDependencyResolver>> {
        self.app.current()
    }

    fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver> {
        self.app.current_mutable()
    }
}

impl IReactiveUIInstance for ReactiveUIBuilder {
    fn main_thread_scheduler(&self) -> Option<Rc<dyn IScheduler>> {
        ReactiveUIBuilder::main_thread_scheduler(self)
    }

    fn task_pool_scheduler(&self) -> Option<Rc<dyn IScheduler>> {
        ReactiveUIBuilder::task_pool_scheduler(self)
    }
}

impl IAppBuilder for ReactiveUIBuilder {
    /// Builds the application once, as Splat's builder does (`AppBuilder.Build`), with the core
    /// services of ReactiveUI. The state of ReactiveUI is initialized by
    /// [`build`](ReactiveUIBuilder::build) and [`build_app`](ReactiveUIBuilder::build_app), which
    /// hide this method upstream.
    fn build(&self) -> &dyn IAppInstance {
        if !AppBuilder::has_been_built() {
            // Upstream's base class calls the overridden `WithCoreServices` before it applies the
            // registrations.
            let _ = ReactiveUIBuilder::with_core_services(self);
        }
        let _ = self.app.build();
        self
    }

    fn use_current_splat_locator(&self) -> &dyn IAppBuilder {
        let _ = self.app.use_current_splat_locator();
        self
    }

    fn using_module(&self, registration_module: Rc<dyn IModule>) -> &dyn IAppBuilder {
        ReactiveUIBuilder::using_splat_module(self, registration_module)
    }

    fn with_core_services(&self) -> &dyn IAppBuilder {
        ReactiveUIBuilder::with_core_services(self)
    }

    fn with_custom_registration(&self, configure_action: Rc<dyn Fn(&dyn IMutableDependencyResolver)>) -> &dyn IAppBuilder {
        let _ = self.app.with_custom_registration(configure_action);
        self
    }
}

impl IReactiveUIBuilder for ReactiveUIBuilder {
    fn with_message_bus(&self) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_message_bus(self)
    }

    fn with_message_bus_with(&self, configure: Rc<dyn Fn(&dyn IMessageBus)>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_message_bus_with(self, move |message_bus: &dyn IMessageBus| configure(message_bus))
    }

    fn with_message_bus_instance(&self, message_bus: Rc<dyn IMessageBus>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_message_bus_instance(self, message_bus)
    }

    fn configure_suspension_driver(&self, configure: Rc<dyn Fn(&Rc<dyn ISuspensionDriver>)>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::configure_suspension_driver(self, move |driver: &Rc<dyn ISuspensionDriver>| configure(driver))
    }

    fn configure_view_locator(&self, configure: Rc<dyn Fn(&DefaultViewLocator)>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::configure_view_locator(self, move |locator: &DefaultViewLocator| configure(locator))
    }

    fn for_custom_platform(&self, main_thread_scheduler: Rc<dyn IScheduler>, platform_services: ResolverAction) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::for_custom_platform(self, main_thread_scheduler, move |resolver: &dyn IMutableDependencyResolver| {
            platform_services(resolver)
        })
    }

    fn for_platforms(&self, platform_configurations: &[&dyn Fn(&dyn IReactiveUIBuilder)]) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::for_platforms(self, platform_configurations)
    }

    fn with_main_thread_scheduler(&self, scheduler: Rc<dyn IScheduler>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_main_thread_scheduler(self, scheduler)
    }

    fn with_main_thread_scheduler_with(&self, scheduler: Rc<dyn IScheduler>, set_rx_app: bool) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_main_thread_scheduler_with(self, scheduler, set_rx_app)
    }

    fn with_platform_services(&self) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_platform_services(self)
    }

    fn with_registration(&self, configure_action: &dyn Fn(&dyn IMutableDependencyResolver)) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_registration(self, configure_action)
    }

    fn with_registrations(&self, registration: &dyn IWantsToRegisterStuff) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_registrations(self, registration)
    }

    fn with_registration_on_build(&self, configure_action: ResolverAction) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_registration_on_build(self, move |resolver: &dyn IMutableDependencyResolver| configure_action(resolver))
    }

    fn with_task_pool_scheduler(&self, scheduler: Rc<dyn IScheduler>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_task_pool_scheduler(self, scheduler)
    }

    fn with_task_pool_scheduler_with(&self, scheduler: Rc<dyn IScheduler>, set_rx_app: bool) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_task_pool_scheduler_with(self, scheduler, set_rx_app)
    }

    fn with_exception_handler(&self, exception_handler: Rc<dyn IObserver<ObservableError>>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_exception_handler(self, exception_handler)
    }

    fn with_suspension_host(&self) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_suspension_host(self)
    }

    fn with_suspension_host_instance(&self, suspension_host: Rc<dyn ISuspensionHost>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_suspension_host_instance(self, suspension_host)
    }

    fn with_cache_sizes(&self, small_cache_limit: usize, big_cache_limit: usize) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_cache_sizes(self, small_cache_limit, big_cache_limit)
    }

    fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_converter(self, converter)
    }

    fn with_converter_factory(&self, factory: &dyn Fn() -> Rc<dyn IBindingTypeConverter>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_converter_factory(self, factory)
    }

    fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_fallback_converter(self, converter)
    }

    fn with_fallback_converter_factory(&self, factory: &dyn Fn() -> Rc<dyn IBindingFallbackConverter>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_fallback_converter_factory(self, factory)
    }

    fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_set_method_converter(self, converter)
    }

    fn with_set_method_converter_factory(&self, factory: &dyn Fn() -> Rc<dyn ISetMethodBindingConverter>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_set_method_converter_factory(self, factory)
    }

    fn with_converters_from(&self, resolver: &dyn IReadonlyDependencyResolver) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::with_converters_from(self, resolver)
    }

    fn using_splat_module(&self, registration_module: Rc<dyn IModule>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::using_splat_module(self, registration_module)
    }

    fn using_splat_builder(&self, app_builder: Option<&dyn Fn(&dyn IAppBuilder)>) -> &dyn IReactiveUIBuilder {
        ReactiveUIBuilder::using_splat_builder(self, app_builder)
    }

    fn build_app(&self) -> &dyn IReactiveUIInstance {
        ReactiveUIBuilder::build_app(self)
    }
}
