//! Splat's `Locator`: [`AppLocator`] with the services of Splat registered on each new resolver.

use super::app_locator::AppLocator;
use super::dependency_resolver::{
    IDependencyResolver, IMutableDependencyResolver, IReadonlyDependencyResolver,
};
use super::service_location_initialization::initialize_splat;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    static INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

/// The service locator of Splat (`Locator`): [`AppLocator`], whose resolver initializer
/// registers the default services of Splat ([`initialize_splat`]) on every resolver set
/// afterwards.
///
/// Upstream sets the initializer in the static constructor of the class, the first time it is
/// used; here the first call of any function of `Locator` on the thread sets it.
pub struct Locator;

impl Locator {
    fn ensure_initialized() {
        if INITIALIZED.with(|initialized| initialized.replace(true)) {
            return;
        }
        AppLocator::register_resolver_initializer(|resolver| initialize_splat(resolver));
    }

    /// `Current`.
    pub fn current() -> Rc<dyn IReadonlyDependencyResolver> {
        Self::ensure_initialized();
        AppLocator::current()
    }

    /// `CurrentMutable`.
    pub fn current_mutable() -> Rc<dyn IMutableDependencyResolver> {
        Self::ensure_initialized();
        AppLocator::current_mutable()
    }

    /// `SetLocator`.
    pub fn set_locator(dependency_resolver: Rc<dyn IDependencyResolver>) {
        Self::ensure_initialized();
        AppLocator::set_locator(dependency_resolver);
    }

    /// `GetLocator`.
    pub fn get_locator() -> Rc<dyn IDependencyResolver> {
        Self::ensure_initialized();
        AppLocator::get_locator()
    }

    /// `RegisterResolverCallbackChanged`.
    pub fn register_resolver_callback_changed(callback: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        Self::ensure_initialized();
        AppLocator::register_resolver_callback_changed(callback)
    }

    /// `SuppressResolverCallbackChangedNotifications`.
    pub fn suppress_resolver_callback_changed_notifications() -> Rc<dyn IDisposable> {
        Self::ensure_initialized();
        AppLocator::suppress_resolver_callback_changed_notifications()
    }

    /// `AreResolverCallbackChangedNotificationsEnabled`.
    pub fn are_resolver_callback_changed_notifications_enabled() -> bool {
        Self::ensure_initialized();
        AppLocator::are_resolver_callback_changed_notifications_enabled()
    }
}
