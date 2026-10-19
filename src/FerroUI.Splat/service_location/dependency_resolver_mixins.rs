//! `DependencyResolverMixins`: making a resolver the current one for a scope.

use super::app_locator::AppLocator;
use super::dependency_resolver::IDependencyResolver;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// `WithResolver` of a resolver (`DependencyResolverMixins`).
///
/// The `Type`-based `RegisterConstant(object?, Type)` and `RegisterLazySingleton(Func<object?>,
/// Type)` of upstream are not ported: they take the service type at run time.
pub trait DependencyResolverMixins {
    /// Makes the resolver the current one of [`AppLocator`] until the returned disposable is
    /// disposed, without notifying the callbacks of the locator (`WithResolver()`).
    fn with_resolver(&self) -> Rc<dyn IDisposable> {
        self.with_resolver_suppressing(true)
    }

    /// Makes the resolver the current one of [`AppLocator`] until the returned disposable is
    /// disposed; the callbacks of the locator are notified unless `suppress_resolver_callback`
    /// (`WithResolver(bool)`).
    fn with_resolver_suppressing(&self, suppress_resolver_callback: bool) -> Rc<dyn IDisposable>;
}

impl<R: IDependencyResolver + 'static> DependencyResolverMixins for Rc<R> {
    fn with_resolver_suppressing(&self, suppress_resolver_callback: bool) -> Rc<dyn IDisposable> {
        let resolver: Rc<dyn IDependencyResolver> = self.clone();
        AppLocator::internal_locator().with_resolver(resolver, suppress_resolver_callback)
    }
}

impl DependencyResolverMixins for Rc<dyn IDependencyResolver> {
    fn with_resolver_suppressing(&self, suppress_resolver_callback: bool) -> Rc<dyn IDisposable> {
        AppLocator::internal_locator().with_resolver(self.clone(), suppress_resolver_callback)
    }
}
