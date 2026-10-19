//! `IAppBuilder`, `IAppInstance`, `IModule`, `AppBuilder` and `SplatBuilderExtensions`.

use crate::service_location::{AppLocator, IMutableDependencyResolver, IReadonlyDependencyResolver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A module that registers its services with a resolver (`IModule`).
pub trait IModule {
    /// Registers the services of the module with `resolver` (`Configure`).
    fn configure(&self, resolver: &dyn IMutableDependencyResolver);
}

/// A built application: the resolvers of its services (`IAppInstance`).
pub trait IAppInstance {
    /// The resolver the application resolves its services from, if any (`Current`).
    fn current(&self) -> Option<Rc<dyn IReadonlyDependencyResolver>>;

    /// The resolver the application registers its services with (`CurrentMutable`).
    fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver>;
}

/// Configures the services of an application, then builds it (`IAppBuilder`). Each method returns
/// the builder, to chain the calls.
pub trait IAppBuilder {
    /// Applies the registrations and returns the application (`Build`).
    fn build(&self) -> &dyn IAppInstance;

    /// Applies the registrations to the current resolver of [`AppLocator`] at build time instead
    /// of the builder's (`UseCurrentSplatLocator`).
    fn use_current_splat_locator(&self) -> &dyn IAppBuilder;

    /// Adds a module, whose registrations are applied at build time (`UsingModule<T>`).
    fn using_module(&self, registration_module: Rc<dyn IModule>) -> &dyn IAppBuilder;

    /// Adds the core services (`WithCoreServices`).
    fn with_core_services(&self) -> &dyn IAppBuilder;

    /// Adds registrations, applied at build time (`WithCustomRegistration`).
    fn with_custom_registration(
        &self,
        configure_action: Rc<dyn Fn(&dyn IMutableDependencyResolver)>,
    ) -> &dyn IAppBuilder;
}

// Deviation (DEVIATIONS.md, Splat): `HasBeenBuilt` and `UsingBuilder` are static properties of the
// process upstream; here they are per thread, as the locator is.
ferroui_system::ferro_thread_local! {
    static HAS_BEEN_BUILT: Cell<bool> = const { Cell::new(false) };
    static USING_BUILDER: Cell<bool> = const { Cell::new(false) };
}

type Registration = Rc<dyn Fn(&dyn IMutableDependencyResolver)>;
type ResolverProvider = Rc<dyn Fn() -> Rc<dyn IMutableDependencyResolver>>;
type ServiceProvider = Rc<dyn Fn() -> Option<Rc<dyn IReadonlyDependencyResolver>>>;

/// The builder of Splat (`AppBuilder`): it collects modules and registrations and applies them
/// once, when the application is built for the first time.
///
/// Upstream subclasses override `WithCoreServices`; a builder built on this one wraps it and
/// implements [`IAppBuilder`], adding its core services before it builds the inner builder.
pub struct AppBuilder {
    registrations: RefCell<Vec<Registration>>,
    resolver_provider: RefCell<ResolverProvider>,
    service_provider: RefCell<ServiceProvider>,
    current: RefCell<Option<Rc<dyn IReadonlyDependencyResolver>>>,
    current_mutable: Rc<dyn IMutableDependencyResolver>,
}

impl AppBuilder {
    /// A builder over `resolver` (`AppBuilder(IMutableDependencyResolver)`).
    pub fn new(resolver: Rc<dyn IMutableDependencyResolver>) -> Self {
        Self::with_current(resolver, None)
    }

    /// A builder over `resolver`, with `current` as the resolver of the application
    /// (`AppBuilder(IMutableDependencyResolver, IReadonlyDependencyResolver?)`).
    pub fn with_current(
        resolver: Rc<dyn IMutableDependencyResolver>,
        current: Option<Rc<dyn IReadonlyDependencyResolver>>,
    ) -> Self {
        USING_BUILDER.with(|flag| flag.set(true));
        let provided_mutable = resolver.clone();
        let provided_current = current.clone();
        Self {
            registrations: RefCell::new(Vec::new()),
            resolver_provider: RefCell::new(Rc::new(move || provided_mutable.clone())),
            service_provider: RefCell::new(Rc::new(move || provided_current.clone())),
            current: RefCell::new(current),
            current_mutable: resolver,
        }
    }

    /// A builder over the current resolver of [`AppLocator`] (`CreateSplatBuilder`).
    pub fn create_splat_builder() -> Self {
        Self::with_current(AppLocator::current_mutable(), Some(AppLocator::current()))
    }

    /// Whether an application was built (`HasBeenBuilt`).
    pub fn has_been_built() -> bool {
        HAS_BEEN_BUILT.with(|flag| flag.get())
    }

    /// Whether a builder was created (`UsingBuilder`).
    pub fn using_builder() -> bool {
        USING_BUILDER.with(|flag| flag.get())
    }

    /// Resets [`has_been_built`](Self::has_been_built) and [`using_builder`](Self::using_builder)
    /// (`ResetBuilderStateForTests`).
    pub fn reset_builder_state_for_tests() {
        Self::restore_state(false, false);
    }

    /// `GetState`. For the tests of the crate.
    #[doc(hidden)]
    pub fn get_state() -> (bool, bool) {
        (Self::has_been_built(), Self::using_builder())
    }

    /// `RestoreState`. For the tests of the crate.
    #[doc(hidden)]
    pub fn restore_state(has_been_built: bool, using_builder: bool) {
        HAS_BEEN_BUILT.with(|flag| flag.set(has_been_built));
        USING_BUILDER.with(|flag| flag.set(using_builder));
    }

    /// `ResetState`. For the tests of the crate.
    #[doc(hidden)]
    pub fn reset_state() {
        Self::restore_state(false, false);
    }
}

impl IAppInstance for AppBuilder {
    fn current(&self) -> Option<Rc<dyn IReadonlyDependencyResolver>> {
        self.current.borrow().clone()
    }

    fn current_mutable(&self) -> Rc<dyn IMutableDependencyResolver> {
        self.current_mutable.clone()
    }
}

impl IAppBuilder for AppBuilder {
    fn build(&self) -> &dyn IAppInstance {
        // An application is built once.
        if Self::has_been_built() {
            return self;
        }

        HAS_BEEN_BUILT.with(|flag| flag.set(true));

        // The core services are always registered.
        let _ = self.with_core_services();

        // The registrations are applied to the resolver the builder provides now.
        let registrations: Vec<Registration> = self.registrations.borrow().clone();
        for registration in registrations {
            let provider = self.resolver_provider.borrow().clone();
            let target_resolver = provider();
            registration(&*target_resolver);
        }

        let provider = self.service_provider.borrow().clone();
        *self.current.borrow_mut() = provider();
        self
    }

    fn use_current_splat_locator(&self) -> &dyn IAppBuilder {
        *self.resolver_provider.borrow_mut() = Rc::new(AppLocator::current_mutable);
        *self.service_provider.borrow_mut() = Rc::new(|| Some(AppLocator::current()));
        self
    }

    fn using_module(&self, registration_module: Rc<dyn IModule>) -> &dyn IAppBuilder {
        self.registrations
            .borrow_mut()
            .push(Rc::new(move |resolver: &dyn IMutableDependencyResolver| registration_module.configure(resolver)));
        self
    }

    fn with_core_services(&self) -> &dyn IAppBuilder {
        self
    }

    fn with_custom_registration(
        &self,
        configure_action: Rc<dyn Fn(&dyn IMutableDependencyResolver)>,
    ) -> &dyn IAppBuilder {
        self.registrations.borrow_mut().push(configure_action);
        self
    }
}

/// `Apply` of a module (`SplatBuilderExtensions`).
pub trait ModuleExt {
    /// Registers the services of the module with the current resolver of [`AppLocator`]
    /// (`Apply`).
    fn apply(&self);
}

impl<M: IModule + ?Sized> ModuleExt for M {
    fn apply(&self) {
        self.configure(&*AppLocator::current_mutable());
    }
}

/// `CreateSplatBuilder` of a resolver (`SplatBuilderExtensions`).
pub trait SplatBuilderExt {
    /// A builder over the resolver (`CreateSplatBuilder()`).
    fn create_splat_builder(&self) -> AppBuilder;

    /// A builder over the resolver, with `configure_action` as its first registration
    /// (`CreateSplatBuilder(Action<IMutableDependencyResolver>)`).
    fn create_splat_builder_with(
        &self,
        configure_action: impl Fn(&dyn IMutableDependencyResolver) + 'static,
    ) -> AppBuilder;
}

impl<R: IMutableDependencyResolver + 'static> SplatBuilderExt for Rc<R> {
    fn create_splat_builder(&self) -> AppBuilder {
        AppBuilder::new(self.clone())
    }

    fn create_splat_builder_with(
        &self,
        configure_action: impl Fn(&dyn IMutableDependencyResolver) + 'static,
    ) -> AppBuilder {
        let builder = AppBuilder::new(self.clone());
        let _ = builder.with_custom_registration(Rc::new(configure_action));
        builder
    }
}

impl SplatBuilderExt for Rc<dyn IMutableDependencyResolver> {
    fn create_splat_builder(&self) -> AppBuilder {
        AppBuilder::new(self.clone())
    }

    fn create_splat_builder_with(
        &self,
        configure_action: impl Fn(&dyn IMutableDependencyResolver) + 'static,
    ) -> AppBuilder {
        let builder = AppBuilder::new(self.clone());
        let _ = builder.with_custom_registration(Rc::new(configure_action));
        builder
    }
}
