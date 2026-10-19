//! `IReactiveUIBuilder` (`ReactiveUI.Shared/Builder/IReactiveUIBuilder.cs`).

use super::IReactiveUIInstance;
use crate::{
    DefaultViewLocator, IBindingFallbackConverter, IBindingTypeConverter, IBindingTypeConverterOf, IMessageBus, IReactiveObject,
    ISetMethodBindingConverter, ISuspensionDriver, ISuspensionHost, IViewForOf, IWantsToRegisterStuff, SuspensionHostOf,
};
use ferroui_reactive::concurrency::IScheduler;
use ferroui_splat::{IAppBuilder, IModule, IMutableDependencyResolver, IReadonlyDependencyResolver, MutableDependencyResolverExt};
use ferroui_system::{IObserver, ObservableError};
use std::rc::Rc;

/// A registration applied to a resolver (`Action<IMutableDependencyResolver>`).
pub type ResolverAction = Rc<dyn Fn(&dyn IMutableDependencyResolver)>;

/// The builder of a ReactiveUI application (`IReactiveUIBuilder`): the services, schedulers and
/// state of ReactiveUI, configured by chained calls, then built once.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): a trait object cannot have generic methods, so this
/// trait has the methods of upstream's interface that are not generic, each returning the builder
/// as a trait object; the generic ones (`RegisterView<TView, TViewModel>`, `WithPlatformModule<T>`,
/// `WithSuspensionHost<TAppState>`, `WithConverter<TFrom, TTo>`) are those of
/// [`ReactiveUIBuilderExt`], which every builder has, and `WithInstance` is
/// [`ReactiveUIInstanceMixins::with_instance`](super::ReactiveUIInstanceMixins::with_instance) of
/// the built application. Upstream's overloads have their own names: the forms with a flag that
/// says whether to set `RxSchedulers` end in `_with`, and the forms that take a factory end in
/// `_factory`. The concrete [`ReactiveUIBuilder`](super::ReactiveUIBuilder) has the same methods,
/// returning itself, so that a chain of calls on it keeps every method at hand.
pub trait IReactiveUIBuilder: IAppBuilder {
    /// Registers a new message bus when the application is built (`WithMessageBus()`).
    fn with_message_bus(&self) -> &dyn IReactiveUIBuilder;

    /// Registers a new message bus, configured by `configure`, when the application is built
    /// (`WithMessageBus(Action<IMessageBus>)`).
    fn with_message_bus_with(&self, configure: Rc<dyn Fn(&dyn IMessageBus)>) -> &dyn IReactiveUIBuilder;

    /// Registers `message_bus` when the application is built (`WithMessageBus(IMessageBus)`).
    fn with_message_bus_instance(&self, message_bus: Rc<dyn IMessageBus>) -> &dyn IReactiveUIBuilder;

    /// Configures the registered suspension driver, if there is one, when the application is built
    /// (`ConfigureSuspensionDriver(Action<ISuspensionDriver>)`).
    fn configure_suspension_driver(&self, configure: Rc<dyn Fn(&Rc<dyn ISuspensionDriver>)>) -> &dyn IReactiveUIBuilder;

    /// Configures the registered [`DefaultViewLocator`] when the application is built, keeping its
    /// mappings; with none registered, a new one is configured and registered as the view locator
    /// (`ConfigureViewLocator(Action<DefaultViewLocator>)`).
    fn configure_view_locator(&self, configure: Rc<dyn Fn(&DefaultViewLocator)>) -> &dyn IReactiveUIBuilder;

    /// Configures a platform of the application's own: its main thread scheduler, and its
    /// services, registered when the application is built
    /// (`ForCustomPlatform(ISequencer, Action<IMutableDependencyResolver>)`).
    fn for_custom_platform(&self, main_thread_scheduler: Rc<dyn IScheduler>, platform_services: ResolverAction) -> &dyn IReactiveUIBuilder;

    /// Applies the configuration of each platform (`ForPlatforms(params Action<IReactiveUIBuilder>[])`).
    fn for_platforms(&self, platform_configurations: &[&dyn Fn(&dyn IReactiveUIBuilder)]) -> &dyn IReactiveUIBuilder;

    /// Sets the scheduler of the user-interface thread, which also becomes
    /// `RxSchedulers::main_thread_scheduler` when the application is built
    /// (`WithMainThreadScheduler(ISequencer)`).
    fn with_main_thread_scheduler(&self, scheduler: Rc<dyn IScheduler>) -> &dyn IReactiveUIBuilder;

    /// Sets the scheduler of the user-interface thread; `set_rx_app` says whether it becomes
    /// `RxSchedulers::main_thread_scheduler` (`WithMainThreadScheduler(ISequencer, bool)`).
    fn with_main_thread_scheduler_with(&self, scheduler: Rc<dyn IScheduler>, set_rx_app: bool) -> &dyn IReactiveUIBuilder;

    /// Registers the services of the platform, once (`WithPlatformServices`).
    fn with_platform_services(&self) -> &dyn IReactiveUIBuilder;

    /// Applies `configure_action` to the resolver at once (`WithRegistration(Action<IMutableDependencyResolver>)`).
    fn with_registration(&self, configure_action: &dyn Fn(&dyn IMutableDependencyResolver)) -> &dyn IReactiveUIBuilder;

    /// Applies `registration` to the resolver at once, through a [`DependencyResolverRegistrar`](crate::DependencyResolverRegistrar)
    /// (`WithRegistration(IWantsToRegisterStuff)`).
    fn with_registrations(&self, registration: &dyn IWantsToRegisterStuff) -> &dyn IReactiveUIBuilder;

    /// Applies `configure_action` to the resolver when the application is built
    /// (`WithRegistrationOnBuild(Action<IMutableDependencyResolver>)`).
    fn with_registration_on_build(&self, configure_action: ResolverAction) -> &dyn IReactiveUIBuilder;

    /// Sets the scheduler of background work, which also becomes
    /// `RxSchedulers::task_pool_scheduler` when the application is built
    /// (`WithTaskPoolScheduler(ISequencer)`).
    fn with_task_pool_scheduler(&self, scheduler: Rc<dyn IScheduler>) -> &dyn IReactiveUIBuilder;

    /// Sets the scheduler of background work; `set_rx_app` says whether it becomes
    /// `RxSchedulers::task_pool_scheduler` (`WithTaskPoolScheduler(ISequencer, bool)`).
    fn with_task_pool_scheduler_with(&self, scheduler: Rc<dyn IScheduler>, set_rx_app: bool) -> &dyn IReactiveUIBuilder;

    /// Sets the handler of the errors that no `thrown_exceptions` subscriber handles
    /// (`WithExceptionHandler(IObserver<Exception>)`).
    fn with_exception_handler(&self, exception_handler: Rc<dyn IObserver<ObservableError>>) -> &dyn IReactiveUIBuilder;

    /// Sets a suspension host, which becomes the one of `RxSuspension` when the application is
    /// built (`WithSuspensionHost()`).
    fn with_suspension_host(&self) -> &dyn IReactiveUIBuilder;

    /// Sets `suspension_host` as the suspension host: the form of `WithSuspensionHost<TAppState>()`
    /// that a trait object can have, which [`ReactiveUIBuilderExt::with_suspension_host_of`] calls.
    fn with_suspension_host_instance(&self, suspension_host: Rc<dyn ISuspensionHost>) -> &dyn IReactiveUIBuilder;

    /// Sets the limits of the caches of ReactiveUI (`WithCacheSizes(int, int)`).
    ///
    /// # Panics
    /// When a limit is zero (`ArgumentOutOfRangeException`).
    fn with_cache_sizes(&self, small_cache_limit: usize, big_cache_limit: usize) -> &dyn IReactiveUIBuilder;

    /// Registers a converter of a pair of types (`WithConverter(IBindingTypeConverter)`).
    fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> &dyn IReactiveUIBuilder;

    /// Registers the converter that `factory` creates (`WithConverter(Func<IBindingTypeConverter>)`).
    fn with_converter_factory(&self, factory: &dyn Fn() -> Rc<dyn IBindingTypeConverter>) -> &dyn IReactiveUIBuilder;

    /// Registers a fallback converter (`WithFallbackConverter(IBindingFallbackConverter)`).
    fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> &dyn IReactiveUIBuilder;

    /// Registers the fallback converter that `factory` creates
    /// (`WithFallbackConverter(Func<IBindingFallbackConverter>)`).
    fn with_fallback_converter_factory(&self, factory: &dyn Fn() -> Rc<dyn IBindingFallbackConverter>) -> &dyn IReactiveUIBuilder;

    /// Registers a set-method converter (`WithSetMethodConverter(ISetMethodBindingConverter)`).
    fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> &dyn IReactiveUIBuilder;

    /// Registers the set-method converter that `factory` creates
    /// (`WithSetMethodConverter(Func<ISetMethodBindingConverter>)`).
    fn with_set_method_converter_factory(&self, factory: &dyn Fn() -> Rc<dyn ISetMethodBindingConverter>) -> &dyn IReactiveUIBuilder;

    /// Registers the converters of the three kinds that `resolver` has
    /// (`WithConvertersFrom(IReadonlyDependencyResolver)`).
    fn with_converters_from(&self, resolver: &dyn IReadonlyDependencyResolver) -> &dyn IReactiveUIBuilder;

    /// Adds a Splat module (`UsingSplatModule<T>(T)`).
    fn using_splat_module(&self, registration_module: Rc<dyn IModule>) -> &dyn IReactiveUIBuilder;

    /// Calls `app_builder`, if any, with the builder as a Splat builder
    /// (`UsingSplatBuilder(Action<IAppBuilder>)`).
    fn using_splat_builder(&self, app_builder: Option<&dyn Fn(&dyn IAppBuilder)>) -> &dyn IReactiveUIBuilder;

    /// Builds the application once, with its core services, and initializes the state of
    /// ReactiveUI (`Build()` and `BuildApp()`).
    ///
    /// # Panics
    /// When the application has no resolver (`InvalidOperationException`).
    fn build_app(&self) -> &dyn IReactiveUIInstance;
}

/// The generic methods of [`IReactiveUIBuilder`] (`RegisterView<TView, TViewModel>`,
/// `RegisterViewModel<TViewModel>`, `WithPlatformModule<T>`, `WithSuspensionHost<TAppState>`,
/// `WithConverter<TFrom, TTo>`), which every builder has.
pub trait ReactiveUIBuilderExt: IReactiveUIBuilder {
    /// Registers `V` as the view of `VM`, one view created at the first resolution
    /// (`RegisterSingletonView<TView, TViewModel>()`).
    fn register_singleton_view<V: IViewForOf<VM> + Default + 'static, VM: IReactiveObject>(&self) -> &dyn IReactiveUIBuilder {
        self.with_registration(&|resolver: &dyn IMutableDependencyResolver| {
            resolver.register_lazy_singleton::<dyn IViewForOf<VM>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>));
        })
    }

    /// Registers `VM`, one view model created at the first resolution
    /// (`RegisterSingletonViewModel<TViewModel>()`).
    fn register_singleton_view_model<VM: IReactiveObject + Default>(&self) -> &dyn IReactiveUIBuilder {
        self.with_registration(&|resolver: &dyn IMutableDependencyResolver| {
            resolver.register_lazy_singleton::<VM>(|| Some(Rc::new(VM::default())));
        })
    }

    /// Registers `V` as the view of `VM`, a new view at each resolution
    /// (`RegisterView<TView, TViewModel>()`).
    fn register_view<V: IViewForOf<VM> + Default + 'static, VM: IReactiveObject>(&self) -> &dyn IReactiveUIBuilder {
        self.with_registration(&|resolver: &dyn IMutableDependencyResolver| {
            resolver.register::<dyn IViewForOf<VM>>(|| Some(Rc::new(V::default()) as Rc<dyn IViewForOf<VM>>));
        })
    }

    /// Registers `VM`, a new view model at each resolution (`RegisterViewModel<TViewModel>()`).
    fn register_view_model<VM: IReactiveObject + Default>(&self) -> &dyn IReactiveUIBuilder {
        self.with_registration(&|resolver: &dyn IMutableDependencyResolver| {
            resolver.register::<VM>(|| Some(Rc::new(VM::default())));
        })
    }

    /// Registers one view model of `VM`, created now, as a constant
    /// (`RegisterConstantViewModel<TViewModel>()`).
    fn register_constant_view_model<VM: IReactiveObject + Default>(&self) -> &dyn IReactiveUIBuilder {
        self.with_registration(&|resolver: &dyn IMutableDependencyResolver| {
            resolver.register_constant::<VM>(Rc::new(VM::default()));
        })
    }

    /// Applies the registrations of a new `T` at once (`WithPlatformModule<T>()`).
    fn with_platform_module<T: IWantsToRegisterStuff + Default>(&self) -> &dyn IReactiveUIBuilder {
        self.with_registrations(&T::default())
    }

    /// Sets a suspension host of an application state of type `T`
    /// (`WithSuspensionHost<TAppState>()`).
    fn with_suspension_host_of<T: 'static>(&self) -> &dyn IReactiveUIBuilder {
        self.with_suspension_host_instance(SuspensionHostOf::<T>::new())
    }

    /// Registers a converter of `F` to `T` (`WithConverter<TFrom, TTo>(IBindingTypeConverter<TFrom, TTo>)`).
    fn with_converter_of<F: 'static, T: 'static>(&self, converter: Rc<dyn IBindingTypeConverterOf<F, T>>) -> &dyn IReactiveUIBuilder {
        self.with_converter(converter as Rc<dyn IBindingTypeConverter>)
    }

    /// Registers the converter of `F` to `T` that `factory` creates
    /// (`WithConverter<TFrom, TTo>(Func<IBindingTypeConverter<TFrom, TTo>>)`).
    fn with_converter_of_factory<F: 'static, T: 'static>(
        &self,
        factory: impl FnOnce() -> Rc<dyn IBindingTypeConverterOf<F, T>>,
    ) -> &dyn IReactiveUIBuilder {
        self.with_converter(factory() as Rc<dyn IBindingTypeConverter>)
    }
}

impl<B: IReactiveUIBuilder + ?Sized> ReactiveUIBuilderExt for B {}
