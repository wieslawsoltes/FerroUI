//! `IReadonlyDependencyResolver`, `IMutableDependencyResolver` and `IDependencyResolver`, with the
//! generic API over them.

use super::service_type::{
    erase_service, unerase_service, ErasedService, LazyDisposal, RegistrationCallback,
    ServiceFactory, ServiceType,
};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Resolves services (`IReadonlyDependencyResolver`).
///
/// The methods are the object-safe core: callers use the generic forms of
/// [`ReadonlyDependencyResolverExt`].
// Deviation (DEVIATIONS.md, Splat): the `Type`-based overloads (`GetService(Type)`, ...) have no
// counterpart; these erased methods are what the generic forms call, keyed by the `TypeId` of the
// service type, and share the registrations of the generic forms.
pub trait IReadonlyDependencyResolver {
    /// The last service registered for `service_type` and `contract`, or `None`
    /// (`GetService<T>(string? contract)`).
    fn get_service_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Option<ErasedService>;

    /// Every service registered for `service_type` and `contract`, in the order of registration
    /// (`GetServices<T>(string? contract)`).
    fn get_services_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Vec<ErasedService>;
}

/// Registers services (`IMutableDependencyResolver`).
///
/// The methods are the object-safe core: callers use the generic forms of
/// [`MutableDependencyResolverExt`].
pub trait IMutableDependencyResolver {
    /// Whether a service is registered for `service_type` and `contract` (`HasRegistration<T>`).
    fn has_registration_erased(&self, service_type: ServiceType, contract: Option<&str>) -> bool;

    /// Registers a factory, called for each resolution (`Register<T>(Func<T?>, string?)`).
    fn register_erased(
        &self,
        factory: ServiceFactory,
        service_type: ServiceType,
        contract: Option<&str>,
    );

    /// Registers a constant (`RegisterConstant<T>(T?, string?)`). `disposal` is what disposing the
    /// resolver does with it: upstream disposes a constant that is `IDisposable`.
    fn register_constant_erased(
        &self,
        value: Option<ErasedService>,
        disposal: Option<Rc<dyn Fn()>>,
        service_type: ServiceType,
        contract: Option<&str>,
    );

    /// Registers a singleton created by `factory` on its first resolution
    /// (`RegisterLazySingleton<T>(Func<T?>, string?)`). `disposal` is what disposing the resolver
    /// does with the value once it was created.
    fn register_lazy_singleton_erased(
        &self,
        factory: ServiceFactory,
        disposal: Option<LazyDisposal>,
        service_type: ServiceType,
        contract: Option<&str>,
    );

    /// Removes the last registration (`UnregisterCurrent<T>(string?)`).
    fn unregister_current_erased(&self, service_type: ServiceType, contract: Option<&str>);

    /// Removes every registration (`UnregisterAll<T>(string?)`).
    fn unregister_all_erased(&self, service_type: ServiceType, contract: Option<&str>);

    /// Calls `callback` for every registration, those that exist and those to come, until the
    /// returned disposable is disposed (`ServiceRegistrationCallback<T>(string?, Action<IDisposable>)`).
    fn service_registration_callback_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
        callback: RegistrationCallback,
    ) -> Rc<dyn IDisposable>;
}

/// A resolver that both resolves and registers services, and is disposed
/// (`IDependencyResolver`).
pub trait IDependencyResolver: IReadonlyDependencyResolver + IMutableDependencyResolver + IDisposable {}

impl<R: IReadonlyDependencyResolver + IMutableDependencyResolver + IDisposable + ?Sized>
    IDependencyResolver for R
{
}

/// The generic forms of [`IReadonlyDependencyResolver`].
pub trait ReadonlyDependencyResolverExt: IReadonlyDependencyResolver {
    /// `GetService<T>()`.
    fn get_service<S: ?Sized + 'static>(&self) -> Option<Rc<S>> {
        self.get_service_with_contract::<S>(None)
    }

    /// `GetService<T>(string? contract)`.
    fn get_service_with_contract<S: ?Sized + 'static>(&self, contract: Option<&str>) -> Option<Rc<S>> {
        self.get_service_erased(ServiceType::of::<S>(), contract)
            .and_then(|service| unerase_service::<S>(&service))
    }

    /// `GetServices<T>()`.
    fn get_services<S: ?Sized + 'static>(&self) -> Vec<Rc<S>> {
        self.get_services_with_contract::<S>(None)
    }

    /// `GetServices<T>(string? contract)`.
    fn get_services_with_contract<S: ?Sized + 'static>(&self, contract: Option<&str>) -> Vec<Rc<S>> {
        self.get_services_erased(ServiceType::of::<S>(), contract)
            .iter()
            .filter_map(unerase_service::<S>)
            .collect()
    }
}

impl<R: IReadonlyDependencyResolver + ?Sized> ReadonlyDependencyResolverExt for R {}

fn erase_factory<S: ?Sized + 'static>(factory: impl Fn() -> Option<Rc<S>> + 'static) -> ServiceFactory {
    Rc::new(move || factory().map(erase_service))
}

/// The generic forms of [`IMutableDependencyResolver`].
pub trait MutableDependencyResolverExt: IMutableDependencyResolver {
    /// `HasRegistration<T>()`.
    fn has_registration<S: ?Sized + 'static>(&self) -> bool {
        self.has_registration_erased(ServiceType::of::<S>(), None)
    }

    /// `HasRegistration<T>(string? contract)`.
    fn has_registration_with_contract<S: ?Sized + 'static>(&self, contract: Option<&str>) -> bool {
        self.has_registration_erased(ServiceType::of::<S>(), contract)
    }

    /// `Register<T>(Func<T?>)`.
    fn register<S: ?Sized + 'static>(&self, factory: impl Fn() -> Option<Rc<S>> + 'static) {
        self.register_erased(erase_factory(factory), ServiceType::of::<S>(), None);
    }

    /// `Register<T>(Func<T?>, string? contract)`.
    fn register_with_contract<S: ?Sized + 'static>(
        &self,
        factory: impl Fn() -> Option<Rc<S>> + 'static,
        contract: Option<&str>,
    ) {
        self.register_erased(erase_factory(factory), ServiceType::of::<S>(), contract);
    }

    /// `RegisterConstant<T>(T)`.
    // Deviation (DEVIATIONS.md, Splat): upstream disposes a constant that is `IDisposable` with the
    // resolver; without reflection, `register_constant_disposable` is the form that does.
    fn register_constant<S: ?Sized + 'static>(&self, value: Rc<S>) {
        self.register_constant_erased(Some(erase_service(value)), None, ServiceType::of::<S>(), None);
    }

    /// `RegisterConstant<T>(T, string? contract)`.
    fn register_constant_with_contract<S: ?Sized + 'static>(&self, value: Rc<S>, contract: Option<&str>) {
        self.register_constant_erased(Some(erase_service(value)), None, ServiceType::of::<S>(), contract);
    }

    /// `RegisterConstant<T>(T)` of a constant that the resolver disposes when it is disposed.
    fn register_constant_disposable<S: ?Sized + IDisposable + 'static>(&self, value: Rc<S>) {
        self.register_constant_disposable_with_contract(value, None);
    }

    /// `RegisterConstant<T>(T, string? contract)` of a constant that the resolver disposes when it
    /// is disposed.
    fn register_constant_disposable_with_contract<S: ?Sized + IDisposable + 'static>(
        &self,
        value: Rc<S>,
        contract: Option<&str>,
    ) {
        let disposed = value.clone();
        self.register_constant_erased(
            Some(erase_service(value)),
            Some(Rc::new(move || disposed.dispose())),
            ServiceType::of::<S>(),
            contract,
        );
    }

    /// `RegisterLazySingleton<T>(Func<T?>)`.
    fn register_lazy_singleton<S: ?Sized + 'static>(&self, factory: impl Fn() -> Option<Rc<S>> + 'static) {
        self.register_lazy_singleton_erased(erase_factory(factory), None, ServiceType::of::<S>(), None);
    }

    /// `RegisterLazySingleton<T>(Func<T?>, string? contract)`.
    fn register_lazy_singleton_with_contract<S: ?Sized + 'static>(
        &self,
        factory: impl Fn() -> Option<Rc<S>> + 'static,
        contract: Option<&str>,
    ) {
        self.register_lazy_singleton_erased(erase_factory(factory), None, ServiceType::of::<S>(), contract);
    }

    /// `RegisterLazySingleton<T>(Func<T?>)` of a singleton that the resolver disposes, once
    /// created, when it is disposed.
    fn register_lazy_singleton_disposable<S: ?Sized + IDisposable + 'static>(
        &self,
        factory: impl Fn() -> Option<Rc<S>> + 'static,
    ) {
        self.register_lazy_singleton_disposable_with_contract(factory, None);
    }

    /// `RegisterLazySingleton<T>(Func<T?>, string? contract)` of a singleton that the resolver
    /// disposes, once created, when it is disposed.
    fn register_lazy_singleton_disposable_with_contract<S: ?Sized + IDisposable + 'static>(
        &self,
        factory: impl Fn() -> Option<Rc<S>> + 'static,
        contract: Option<&str>,
    ) {
        let disposal: LazyDisposal = Rc::new(|value: &ErasedService| {
            if let Some(value) = unerase_service::<S>(value) {
                value.dispose();
            }
        });
        self.register_lazy_singleton_erased(
            erase_factory(factory),
            Some(disposal),
            ServiceType::of::<S>(),
            contract,
        );
    }

    /// `UnregisterCurrent<T>()`.
    fn unregister_current<S: ?Sized + 'static>(&self) {
        self.unregister_current_erased(ServiceType::of::<S>(), None);
    }

    /// `UnregisterCurrent<T>(string? contract)`.
    fn unregister_current_with_contract<S: ?Sized + 'static>(&self, contract: Option<&str>) {
        self.unregister_current_erased(ServiceType::of::<S>(), contract);
    }

    /// `UnregisterAll<T>()`.
    fn unregister_all<S: ?Sized + 'static>(&self) {
        self.unregister_all_erased(ServiceType::of::<S>(), None);
    }

    /// `UnregisterAll<T>(string? contract)`.
    fn unregister_all_with_contract<S: ?Sized + 'static>(&self, contract: Option<&str>) {
        self.unregister_all_erased(ServiceType::of::<S>(), contract);
    }

    /// `ServiceRegistrationCallback<T>(Action<IDisposable>)`.
    fn service_registration_callback<S: ?Sized + 'static>(
        &self,
        callback: impl Fn(&dyn IDisposable) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.service_registration_callback_erased(ServiceType::of::<S>(), None, Rc::new(callback))
    }

    /// `ServiceRegistrationCallback<T>(string? contract, Action<IDisposable>)`.
    fn service_registration_callback_with_contract<S: ?Sized + 'static>(
        &self,
        contract: Option<&str>,
        callback: impl Fn(&dyn IDisposable) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.service_registration_callback_erased(ServiceType::of::<S>(), contract, Rc::new(callback))
    }
}

impl<R: IMutableDependencyResolver + ?Sized> MutableDependencyResolverExt for R {}
