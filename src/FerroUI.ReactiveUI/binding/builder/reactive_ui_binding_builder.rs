//! `ReactiveUIBindingBuilder` (ReactiveUI.Binding, `Builder/ReactiveUIBindingBuilder.cs`).

use super::{IReactiveUIBindingBuilder, IReactiveUIBindingInstance, ReactiveUIBindingModule, RxBindingBuilder};
use crate::binding::interfaces::{ICreatesCommandBinding, IViewLocator};
use crate::binding::view::{DefaultViewLocator, ViewMappingBuilder};
use crate::{register_default_converters, BindingConverters, ConverterService, IBindingFallbackConverter, IBindingTypeConverter, ISetMethodBindingConverter};
use ferroui_splat::{
    AppBuilder, IAppBuilder, IAppInstance, IModule, IMutableDependencyResolver, IReadonlyDependencyResolver, MutableDependencyResolverExt,
    ServiceLocationInitializationExt,
};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::{Rc, Weak};

ferroui_system::ferro_thread_local! {
    /// The builders created on the thread, so that [`BuilderMixins`](crate::BuilderMixins) finds
    /// the builder behind a `&dyn IAppBuilder` (upstream's cast `appBuilder is
    /// IReactiveUIBindingBuilder`).
    static BUILDERS: RefCell<Vec<Weak<ReactiveUIBindingBuilder>>> = const { RefCell::new(Vec::new()) };
}

/// The builder of ReactiveUI.Binding (`ReactiveUIBindingBuilder`): it configures the services of
/// the property observation and the bindings, the converters and the platform modules on a Splat
/// [`AppBuilder`], and is the application it builds.
///
/// The builder is shared as an `Rc`; its methods take `self: &Rc<Self>` and return the builder, to
/// chain the calls:
///
/// ```
/// use ferroui_reactiveui::{RxBindingBuilder, ViewLocator};
///
/// RxBindingBuilder::reset_for_testing();
/// RxBindingBuilder::create_reactive_ui_binding_builder().with_core_services().build_app();
/// let _locator = ViewLocator::current();
/// ```
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream subclasses `AppBuilder` and overrides
/// `WithCoreServices`; here the builder wraps one and implements [`IAppBuilder`], adding its core
/// services before it builds the inner builder. The overloads of upstream's methods are the
/// methods of the builder (`self: &Rc<Self>`, returning the builder) and those of
/// [`IReactiveUIBindingBuilder`] (object safe); the null checks of the arguments have no
/// counterpart.
pub struct ReactiveUIBindingBuilder {
    /// The Splat builder.
    inner: AppBuilder,
    /// `_coreRegistered`.
    core_registered: Cell<bool>,
    /// `ConverterService`.
    converter_service: Rc<ConverterService>,
}

impl fmt::Debug for ReactiveUIBindingBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReactiveUIBindingBuilder").field("core_registered", &self.core_registered.get()).finish_non_exhaustive()
    }
}

impl ReactiveUIBindingBuilder {
    /// A builder over `resolver`, reading the configured services from `current`; it initializes
    /// Splat on the resolver and registers its [`ConverterService`] there
    /// (`ReactiveUIBindingBuilder(IMutableDependencyResolver, IReadonlyDependencyResolver?)`).
    pub fn new(resolver: Rc<dyn IMutableDependencyResolver>, current: Option<Rc<dyn IReadonlyDependencyResolver>>) -> Rc<Self> {
        let inner = AppBuilder::with_current(resolver.clone(), current);
        let converter_service = Rc::new(ConverterService::new());
        resolver.initialize_splat();
        resolver.register_constant::<ConverterService>(converter_service.clone());

        let builder = Rc::new(Self { inner, core_registered: Cell::new(false), converter_service });
        BUILDERS.with(|builders| {
            let mut builders = builders.borrow_mut();
            builders.retain(|registered| registered.strong_count() > 0);
            builders.push(Rc::downgrade(&builder));
        });
        builder
    }

    /// The builder behind `app_builder`, when it is a [`ReactiveUIBindingBuilder`] (upstream's
    /// `appBuilder is IReactiveUIBindingBuilder`).
    pub fn find(app_builder: &dyn IAppBuilder) -> Option<Rc<ReactiveUIBindingBuilder>> {
        let address = app_builder as *const dyn IAppBuilder;
        BUILDERS.with(|builders| {
            builders
                .borrow()
                .iter()
                .find(|registered| std::ptr::addr_eq(registered.as_ptr(), address))
                .and_then(Weak::upgrade)
        })
    }

    /// The converter service of the bindings, with its three registries (`ConverterService`).
    pub fn converter_service(&self) -> &Rc<ConverterService> {
        &self.converter_service
    }

    /// The resolver the builder registers with (`CurrentMutable`).
    pub fn resolver(&self) -> Rc<dyn IMutableDependencyResolver> {
        IAppInstance::current_mutable(&self.inner)
    }

    /// Registers the core services once (`WithCoreServices`): the default converters, the
    /// observers of `PropertyChanged` and of plain objects ([`ReactiveUIBindingModule`]), and a
    /// [`DefaultViewLocator`] as the `dyn IViewLocator`. A second call registers nothing.
    pub fn with_core_services(self: &Rc<Self>) -> Rc<Self> {
        self.register_core_services();
        self.clone()
    }

    /// Adds the module of a platform, applied at build time (`WithPlatformModule<T>(T)`).
    pub fn with_platform_module<M: IModule + 'static>(self: &Rc<Self>, module: M) -> Rc<Self> {
        let _ = self.inner.using_module(Rc::new(module));
        self.clone()
    }

    /// Runs `configure_action` on the resolver now (`WithRegistration`).
    pub fn with_registration(self: &Rc<Self>, configure_action: impl FnOnce(&dyn IMutableDependencyResolver)) -> Rc<Self> {
        configure_action(&*self.resolver());
        self.clone()
    }

    /// Registers a converter of a pair of types in the typed registry (`WithConverter`).
    pub fn with_converter(self: &Rc<Self>, converter: Rc<dyn IBindingTypeConverter>) -> Rc<Self> {
        self.converter_service.typed_converters().register(converter);
        self.clone()
    }

    /// Registers a fallback converter (`WithFallbackConverter`).
    pub fn with_fallback_converter(self: &Rc<Self>, converter: Rc<dyn IBindingFallbackConverter>) -> Rc<Self> {
        self.converter_service.fallback_converters().register(converter);
        self.clone()
    }

    /// Registers a converter of set methods (`WithSetMethodConverter`).
    pub fn with_set_method_converter(self: &Rc<Self>, converter: Rc<dyn ISetMethodBindingConverter>) -> Rc<Self> {
        self.converter_service.set_method_converters().register(converter);
        self.clone()
    }

    /// Registers `binder`, a binder of commands to controls, as a lazy singleton of
    /// `dyn ICreatesCommandBinding` (`WithCommandBinder`).
    pub fn with_command_binder(self: &Rc<Self>, binder: Rc<dyn ICreatesCommandBinding>) -> Rc<Self> {
        self.resolver().register_lazy_singleton::<dyn ICreatesCommandBinding>(move || Some(binder.clone()));
        self.clone()
    }

    /// Creates a [`DefaultViewLocator`] with the mappings `configure` adds, and registers it as the
    /// `dyn IViewLocator` (`ConfigureViewLocator`).
    pub fn configure_view_locator(self: &Rc<Self>, configure: impl FnOnce(&ViewMappingBuilder<'_>)) -> Rc<Self> {
        let locator = Rc::new(DefaultViewLocator::new());
        configure(&ViewMappingBuilder::new(&locator));
        self.resolver().register_constant::<dyn IViewLocator>(locator);
        self.clone()
    }

    /// Builds the application, publishes the [`ConverterService`] through [`BindingConverters`] and
    /// marks ReactiveUI.Binding initialized (`BuildApp`). It panics with an
    /// `InvalidOperationException` message when the build gives no resolver.
    pub fn build_app(self: &Rc<Self>) -> Rc<dyn IReactiveUIBindingInstance> {
        self.build_app_core();
        self.clone()
    }

    /// `WithCoreServices` (the override).
    fn register_core_services(&self) {
        if self.core_registered.get() {
            return;
        }

        // The standard converters.
        register_default_converters(&self.converter_service);

        // The property observers.
        let _ = self.inner.using_module(Rc::new(ReactiveUIBindingModule));

        // The default view locator.
        self.resolver().register_lazy_singleton::<dyn IViewLocator>(|| Some(Rc::new(DefaultViewLocator::new()) as Rc<dyn IViewLocator>));

        self.core_registered.set(true);
    }

    /// `BuildApp`.
    fn build_app_core(&self) {
        let _ = IAppBuilder::build(self);
        if IAppInstance::current(self).is_none() {
            panic!("InvalidOperationException: Failed to create ReactiveUIBindingInstance instance");
        }
        BindingConverters::set_service(self.converter_service.clone());
        RxBindingBuilder::mark_as_initialized();
    }
}

impl IAppInstance for ReactiveUIBindingBuilder {
    fn current(&self) -> Option<Rc<dyn IReadonlyDependencyResolver>> {
        IAppInstance::current(&self.inner)
    }

    fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver> {
        IAppInstance::current_mutable(&self.inner)
    }
}

impl IReactiveUIBindingInstance for ReactiveUIBindingBuilder {}

impl IAppBuilder for ReactiveUIBindingBuilder {
    /// Registers the core services, then applies the registrations of the inner builder, once per
    /// thread (`Build`).
    fn build(&self) -> &dyn IAppInstance {
        if AppBuilder::has_been_built() {
            return self;
        }
        self.register_core_services();
        let _ = self.inner.build();
        self
    }

    fn use_current_splat_locator(&self) -> &dyn IAppBuilder {
        let _ = self.inner.use_current_splat_locator();
        self
    }

    fn using_module(&self, registration_module: Rc<dyn IModule>) -> &dyn IAppBuilder {
        let _ = self.inner.using_module(registration_module);
        self
    }

    fn with_core_services(&self) -> &dyn IAppBuilder {
        self.register_core_services();
        self
    }

    fn with_custom_registration(&self, configure_action: Rc<dyn Fn(&dyn IMutableDependencyResolver)>) -> &dyn IAppBuilder {
        let _ = self.inner.with_custom_registration(configure_action);
        self
    }
}

impl IReactiveUIBindingBuilder for ReactiveUIBindingBuilder {
    fn with_platform_module(&self, module: Rc<dyn IModule>) -> &dyn IReactiveUIBindingBuilder {
        let _ = self.inner.using_module(module);
        self
    }

    fn with_registration(&self, configure_action: &dyn Fn(&dyn IMutableDependencyResolver)) -> &dyn IReactiveUIBindingBuilder {
        configure_action(&*self.resolver());
        self
    }

    fn with_converter(&self, converter: Rc<dyn IBindingTypeConverter>) -> &dyn IReactiveUIBindingBuilder {
        self.converter_service.typed_converters().register(converter);
        self
    }

    fn with_fallback_converter(&self, converter: Rc<dyn IBindingFallbackConverter>) -> &dyn IReactiveUIBindingBuilder {
        self.converter_service.fallback_converters().register(converter);
        self
    }

    fn with_set_method_converter(&self, converter: Rc<dyn ISetMethodBindingConverter>) -> &dyn IReactiveUIBindingBuilder {
        self.converter_service.set_method_converters().register(converter);
        self
    }

    fn with_command_binder(&self, binder: Rc<dyn ICreatesCommandBinding>) -> &dyn IReactiveUIBindingBuilder {
        self.resolver().register_lazy_singleton::<dyn ICreatesCommandBinding>(move || Some(binder.clone()));
        self
    }

    fn configure_view_locator(&self, configure: &dyn Fn(&ViewMappingBuilder<'_>)) -> &dyn IReactiveUIBindingBuilder {
        let locator = Rc::new(DefaultViewLocator::new());
        configure(&ViewMappingBuilder::new(&locator));
        self.resolver().register_constant::<dyn IViewLocator>(locator);
        self
    }

    fn build_app(&self) -> &dyn IReactiveUIBindingInstance {
        self.build_app_core();
        self
    }
}
