//! `AppLocator`: the current resolver of the application.

use super::dependency_resolver::{
    IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver,
    MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
};
use super::internal_locator::InternalLocator;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

type ResolverInitializer = Rc<dyn Fn(&dyn IMutableDependencyResolver)>;

// Deviation (DEVIATIONS.md, Splat): upstream's locator is a global static of the process; the
// resolvers hand out `Rc` services, so the locator is per thread.
ferroui_system::ferro_thread_local! {
    static INTERNAL_LOCATOR: RefCell<Option<Rc<InternalLocator>>> = const { RefCell::new(None) };
    static RE_INIT: RefCell<Option<ResolverInitializer>> = const { RefCell::new(None) };
}

/// The service locator of the application (`AppLocator`): the current resolver, which
/// applications and libraries resolve their services from, and the notifications of its changes.
///
/// It starts with an empty [`InstanceGenericFirstDependencyResolver`]; [`set_locator`] replaces
/// it.
///
/// [`InstanceGenericFirstDependencyResolver`]: super::InstanceGenericFirstDependencyResolver
/// [`set_locator`]: Self::set_locator
pub struct AppLocator;

impl AppLocator {
    /// The locator of the thread, created on first use (`InternalLocator`). For the tests of the
    /// crate.
    #[doc(hidden)]
    pub fn internal_locator() -> Rc<InternalLocator> {
        if let Some(locator) = INTERNAL_LOCATOR.with(|locator| locator.borrow().clone()) {
            return locator;
        }
        // Created outside of the borrow: creating it runs the resolver initializer.
        let locator = InternalLocator::new();
        INTERNAL_LOCATOR.with(|slot| {
            let mut slot = slot.borrow_mut();
            if let Some(existing) = slot.as_ref() {
                return existing.clone();
            }
            *slot = Some(locator.clone());
            locator
        })
    }

    /// Replaces the locator of the thread (the setter of `InternalLocator`). For the tests of the
    /// crate.
    #[doc(hidden)]
    pub fn set_internal_locator(locator: Rc<InternalLocator>) {
        INTERNAL_LOCATOR.with(|slot| *slot.borrow_mut() = Some(locator));
    }

    /// Runs the resolver initializer on `resolver` (`ReInit`).
    pub(crate) fn re_init(resolver: &dyn IMutableDependencyResolver) {
        let initializer = RE_INIT.with(|initializer| initializer.borrow().clone());
        if let Some(initializer) = initializer {
            initializer(resolver);
        }
    }

    /// The current resolver, to resolve services from (`Current`).
    pub fn current() -> Rc<dyn IReadonlyDependencyResolver> {
        Self::internal_locator().current()
    }

    /// The current resolver, to register services with (`CurrentMutable`).
    pub fn current_mutable() -> Rc<dyn IMutableDependencyResolver> {
        Self::internal_locator().current_mutable()
    }

    /// Sets what the locator runs on every new resolver (`RegisterResolverInitializer`); it
    /// replaces the previous initializer.
    pub fn register_resolver_initializer(initializer: impl Fn(&dyn IMutableDependencyResolver) + 'static) {
        RE_INIT.with(|slot| *slot.borrow_mut() = Some(Rc::new(initializer)));
    }

    /// Makes `dependency_resolver` the current resolver and notifies the callbacks
    /// (`SetLocator`).
    pub fn set_locator(dependency_resolver: Rc<dyn IDependencyResolver>) {
        Self::internal_locator().set_locator(dependency_resolver);
    }

    /// The current resolver (`GetLocator`).
    pub fn get_locator() -> Rc<dyn IDependencyResolver> {
        Self::internal_locator().internal()
    }

    /// Calls `callback` now, while notifications are enabled, and whenever the resolver changes,
    /// until the returned disposable is disposed (`RegisterResolverCallbackChanged`).
    pub fn register_resolver_callback_changed(callback: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        Self::internal_locator().register_resolver_callback_changed(Rc::new(callback))
    }

    /// Suppresses the notifications of the changes of the resolver until the returned disposable
    /// is disposed (`SuppressResolverCallbackChangedNotifications`).
    pub fn suppress_resolver_callback_changed_notifications() -> Rc<dyn IDisposable> {
        Self::internal_locator().suppress_resolver_callback_changed_notifications()
    }

    /// `AreResolverCallbackChangedNotificationsEnabled`.
    pub fn are_resolver_callback_changed_notifications_enabled() -> bool {
        Self::internal_locator().are_resolver_callback_changed_notifications_enabled()
    }

    /// `GetService<T>()` of the current resolver.
    pub fn get_service<S: ?Sized + 'static>() -> Option<Rc<S>> {
        Self::current().get_service::<S>()
    }

    /// `GetService<T>(string contract)` of the current resolver.
    pub fn get_service_with_contract<S: ?Sized + 'static>(contract: &str) -> Option<Rc<S>> {
        Self::current().get_service_with_contract::<S>(Some(contract))
    }

    /// `GetServices<T>()` of the current resolver.
    pub fn get_services<S: ?Sized + 'static>() -> Vec<Rc<S>> {
        Self::current().get_services::<S>()
    }

    /// `GetServices<T>(string contract)` of the current resolver.
    pub fn get_services_with_contract<S: ?Sized + 'static>(contract: &str) -> Vec<Rc<S>> {
        Self::current().get_services_with_contract::<S>(Some(contract))
    }

    /// `HasRegistration<T>()` of the current resolver.
    pub fn has_registration<S: ?Sized + 'static>() -> bool {
        Self::current_mutable().has_registration::<S>()
    }

    /// `HasRegistration<T>(string contract)` of the current resolver.
    pub fn has_registration_with_contract<S: ?Sized + 'static>(contract: &str) -> bool {
        Self::current_mutable().has_registration_with_contract::<S>(Some(contract))
    }

    /// `Register<T>(Func<T?>)` with the current resolver.
    pub fn register<S: ?Sized + 'static>(factory: impl Fn() -> Option<Rc<S>> + 'static) {
        Self::current_mutable().register::<S>(factory);
    }

    /// `Register<T>(Func<T?>, string contract)` with the current resolver.
    pub fn register_with_contract<S: ?Sized + 'static>(
        factory: impl Fn() -> Option<Rc<S>> + 'static,
        contract: &str,
    ) {
        Self::current_mutable().register_with_contract::<S>(factory, Some(contract));
    }

    /// `RegisterConstant<T>(T)` with the current resolver.
    pub fn register_constant<S: ?Sized + 'static>(value: Rc<S>) {
        Self::current_mutable().register_constant::<S>(value);
    }

    /// `RegisterConstant<T>(T, string contract)` with the current resolver.
    pub fn register_constant_with_contract<S: ?Sized + 'static>(value: Rc<S>, contract: &str) {
        Self::current_mutable().register_constant_with_contract::<S>(value, Some(contract));
    }

    /// `RegisterLazySingleton<T>(Func<T?>)` with the current resolver.
    pub fn register_lazy_singleton<S: ?Sized + 'static>(factory: impl Fn() -> Option<Rc<S>> + 'static) {
        Self::current_mutable().register_lazy_singleton::<S>(factory);
    }

    /// `RegisterLazySingleton<T>(Func<T?>, string contract)` with the current resolver.
    pub fn register_lazy_singleton_with_contract<S: ?Sized + 'static>(
        factory: impl Fn() -> Option<Rc<S>> + 'static,
        contract: &str,
    ) {
        Self::current_mutable().register_lazy_singleton_with_contract::<S>(factory, Some(contract));
    }

    /// `UnregisterCurrent<T>()` of the current resolver.
    pub fn unregister_current<S: ?Sized + 'static>() {
        Self::current_mutable().unregister_current::<S>();
    }

    /// `UnregisterCurrent<T>(string contract)` of the current resolver.
    pub fn unregister_current_with_contract<S: ?Sized + 'static>(contract: &str) {
        Self::current_mutable().unregister_current_with_contract::<S>(Some(contract));
    }

    /// `UnregisterAll<T>()` of the current resolver.
    pub fn unregister_all<S: ?Sized + 'static>() {
        Self::current_mutable().unregister_all::<S>();
    }

    /// `UnregisterAll<T>(string contract)` of the current resolver.
    pub fn unregister_all_with_contract<S: ?Sized + 'static>(contract: &str) {
        Self::current_mutable().unregister_all_with_contract::<S>(Some(contract));
    }
}
