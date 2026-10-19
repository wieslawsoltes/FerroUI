//! `ResolverMixins`: registrations that return the resolver, to chain them.

use super::dependency_resolver::{IMutableDependencyResolver, MutableDependencyResolverExt};
use std::cell::OnceCell;
use std::rc::Rc;

/// Registrations that return the resolver (`ResolverMixins`).
///
/// `TService : new()` is `S: Default`.
// Deviation (DEVIATIONS.md, Splat): the forms with an implementation type
// (`RegisterAnd<TService, TImplementation>`) and the `Type`-based ones (`RegisterConstantAnd(object,
// Type)`, `RegisterLazySingletonAnd(Func<object>, Type)`) are not ported; a generic `Rc<I>` does
// not coerce to `Rc<dyn S>`, so the factory forms take the coerced service.
pub trait ResolverMixins: IMutableDependencyResolver {
    /// `RegisterAnd<TService>()`: a new `S` per resolution.
    fn register_and<S: Default + 'static>(&self) -> &Self {
        self.register::<S>(|| Some(Rc::new(S::default())));
        self
    }

    /// `RegisterAnd<TService>(string contract)`.
    fn register_and_with_contract<S: Default + 'static>(&self, contract: &str) -> &Self {
        self.register_with_contract::<S>(|| Some(Rc::new(S::default())), Some(contract));
        self
    }

    /// `RegisterAnd<TService>(Func<TService>)`.
    fn register_and_factory<S: ?Sized + 'static>(&self, factory: impl Fn() -> Rc<S> + 'static) -> &Self {
        self.register::<S>(move || Some(factory()));
        self
    }

    /// `RegisterAnd<TService>(Func<TService>, string contract)`.
    fn register_and_factory_with_contract<S: ?Sized + 'static>(
        &self,
        factory: impl Fn() -> Rc<S> + 'static,
        contract: &str,
    ) -> &Self {
        self.register_with_contract::<S>(move || Some(factory()), Some(contract));
        self
    }

    /// `RegisterConstantAnd<TService>()`: one new `S`, registered as a factory that returns it.
    fn register_constant_and<S: Default + 'static>(&self) -> &Self {
        let value = Rc::new(S::default());
        self.register_and_factory::<S>(move || value.clone())
    }

    /// `RegisterConstantAnd<TService>(string contract)`.
    fn register_constant_and_with_contract<S: Default + 'static>(&self, contract: &str) -> &Self {
        let value = Rc::new(S::default());
        self.register_and_factory_with_contract::<S>(move || value.clone(), contract)
    }

    /// `RegisterConstantAnd<TService>(TService value)`.
    fn register_constant_and_value<S: ?Sized + 'static>(&self, value: Rc<S>) -> &Self {
        self.register_and_factory::<S>(move || value.clone())
    }

    /// `RegisterConstantAnd<TService>(TService value, string contract)`.
    fn register_constant_and_value_with_contract<S: ?Sized + 'static>(&self, value: Rc<S>, contract: &str) -> &Self {
        self.register_and_factory_with_contract::<S>(move || value.clone(), contract)
    }

    /// `RegisterLazySingletonAnd<TService>()`: one `S`, created on the first resolution.
    fn register_lazy_singleton_and<S: Default + 'static>(&self) -> &Self {
        let value: OnceCell<Rc<S>> = OnceCell::new();
        self.register::<S>(move || Some(value.get_or_init(|| Rc::new(S::default())).clone()));
        self
    }

    /// `RegisterLazySingletonAnd<TService>(string contract)`.
    fn register_lazy_singleton_and_with_contract<S: Default + 'static>(&self, contract: &str) -> &Self {
        let value: OnceCell<Rc<S>> = OnceCell::new();
        self.register_with_contract::<S>(
            move || Some(value.get_or_init(|| Rc::new(S::default())).clone()),
            Some(contract),
        );
        self
    }

    /// `RegisterLazySingletonAnd<TService>(Func<TService>)`.
    fn register_lazy_singleton_and_factory<S: ?Sized + 'static>(
        &self,
        factory: impl Fn() -> Rc<S> + 'static,
    ) -> &Self {
        let value: OnceCell<Rc<S>> = OnceCell::new();
        self.register::<S>(move || Some(value.get_or_init(&factory).clone()));
        self
    }

    /// `RegisterLazySingletonAnd<TService>(Func<TService>, string contract)`.
    fn register_lazy_singleton_and_factory_with_contract<S: ?Sized + 'static>(
        &self,
        factory: impl Fn() -> Rc<S> + 'static,
        contract: &str,
    ) -> &Self {
        let value: OnceCell<Rc<S>> = OnceCell::new();
        self.register_with_contract::<S>(move || Some(value.get_or_init(&factory).clone()), Some(contract));
        self
    }
}

impl<R: IMutableDependencyResolver + ?Sized> ResolverMixins for R {}
