use std::any::{type_name, Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::reactive::IDisposable;

/// A type-erased service handle: an `Rc<dyn Any>` whose content is the
/// `Rc<TService>` handle of the service. Boxing the handle rather than the
/// service is what lets trait objects (`Rc<dyn IFoo>`) be stored and
/// recovered with their exact type.
pub type ServiceHandle = Rc<dyn Any>;

type Factory = Rc<dyn Fn() -> Option<ServiceHandle>>;

/// Resolves services by type.
pub trait IFerroDependencyResolver {
    /// Returns the service registered for `service_type`, as a handle whose
    /// content is `Rc<TService>`. Use
    /// [`LocatorExtensions::get_service`] for the typed form.
    fn get_service_untyped(&self, service_type: TypeId) -> Option<ServiceHandle>;
}

/// A scoped service registry.
///
/// Services are keyed by the `TypeId` of the service type, which can be a
/// concrete type (`Foo`, fetched as `Rc<Foo>`) or a trait object type
/// (`dyn IFoo`, fetched as `Rc<dyn IFoo>`):
///
/// ```ignore
/// FerroLocator::current_mutable().bind::<dyn IFoo>().to_constant(Rc::new(Foo::new()));
/// let foo: Option<Rc<dyn IFoo>> = FerroLocator::current().get_service::<dyn IFoo>();
/// ```
///
/// Like the rest of the object model the locator is per-thread: the current
/// locator and its registrations are only visible on the thread that made
/// them.
pub struct FerroLocator {
    parent_scope: Option<Rc<dyn IFerroDependencyResolver>>,
    registry: RefCell<HashMap<TypeId, Factory>>,
}

thread_local! {
    static CURRENT: RefCell<Option<(Rc<dyn IFerroDependencyResolver>, Rc<FerroLocator>)>> =
        const { RefCell::new(None) };
}

fn with_current<R>(f: impl FnOnce(&mut (Rc<dyn IFerroDependencyResolver>, Rc<FerroLocator>)) -> R) -> R {
    CURRENT.with(|current| {
        let mut current = current.borrow_mut();
        let current = current.get_or_insert_with(|| {
            let locator = Rc::new(FerroLocator::new());
            (locator.clone() as Rc<dyn IFerroDependencyResolver>, locator)
        });
        f(current)
    })
}

impl Default for FerroLocator {
    fn default() -> Self {
        Self::new()
    }
}

impl FerroLocator {
    /// The resolver services are looked up in.
    pub fn current() -> Rc<dyn IFerroDependencyResolver> {
        with_current(|current| current.0.clone())
    }

    pub fn set_current(value: Rc<dyn IFerroDependencyResolver>) {
        let previous = with_current(|current| std::mem::replace(&mut current.0, value));
        drop(previous);
    }

    /// The locator services are registered in.
    pub fn current_mutable() -> Rc<FerroLocator> {
        with_current(|current| current.1.clone())
    }

    pub fn set_current_mutable(value: Rc<FerroLocator>) {
        let previous = with_current(|current| std::mem::replace(&mut current.1, value));
        drop(previous);
    }

    pub fn new() -> Self {
        Self { parent_scope: None, registry: RefCell::new(HashMap::new()) }
    }

    /// Creates a locator that falls back to `parent_scope` for services it
    /// has no registration for.
    pub fn with_parent_scope(parent_scope: Rc<dyn IFerroDependencyResolver>) -> Self {
        Self { parent_scope: Some(parent_scope), registry: RefCell::new(HashMap::new()) }
    }

    fn register(&self, service_type: TypeId, factory: Factory) {
        let previous = self.registry.borrow_mut().insert(service_type, factory);
        drop(previous);
    }

    /// Starts the registration of the service type `T`, which may be a trait
    /// object type.
    pub fn bind<T: ?Sized + 'static>(&self) -> RegistrationHelper<'_, T> {
        RegistrationHelper { locator: self, _service: PhantomData }
    }

    /// Registers `constant` as the service of its own type.
    pub fn bind_to_self<T: 'static>(&self, constant: Rc<T>) -> &FerroLocator {
        self.bind::<T>().to_constant(constant)
    }

    /// Registers a lazily created instance of `T` as the service of its own
    /// type.
    pub fn bind_to_self_singleton<T: Default + 'static>(&self) -> &FerroLocator {
        self.bind::<T>().to_singleton::<T>(|instance| instance)
    }

    /// Makes a new locator, whose parent is the current resolver, the
    /// current one. Disposing the returned value restores the previous
    /// resolver and locator.
    pub fn enter_scope() -> Rc<dyn IDisposable> {
        let (resolver, mutable) = with_current(|current| current.clone());
        let scope = Rc::new(FerroLocator::with_parent_scope(resolver.clone()));
        Self::set_current(scope.clone());
        Self::set_current_mutable(scope);
        Rc::new(ResolverDisposable { resolver, mutable })
    }
}

impl IFerroDependencyResolver for FerroLocator {
    fn get_service_untyped(&self, service_type: TypeId) -> Option<ServiceHandle> {
        // The factory is cloned out so that it can use the locator itself.
        let factory = self.registry.borrow().get(&service_type).cloned();
        match factory {
            Some(factory) => factory(),
            None => self.parent_scope.as_ref().and_then(|parent| parent.get_service_untyped(service_type)),
        }
    }
}

/// Binds the service type `T` to an implementation; see
/// [`FerroLocator::bind`].
pub struct RegistrationHelper<'a, T: ?Sized + 'static> {
    locator: &'a FerroLocator,
    _service: PhantomData<fn() -> Rc<T>>,
}

impl<'a, T: ?Sized + 'static> RegistrationHelper<'a, T> {
    /// Every lookup returns `constant`.
    pub fn to_constant(self, constant: Rc<T>) -> &'a FerroLocator {
        let handle: ServiceHandle = Rc::new(constant);
        self.locator.register(TypeId::of::<T>(), Rc::new(move || Some(handle.clone())));
        self.locator
    }

    /// Every lookup calls `func`.
    pub fn to_func(self, func: impl Fn() -> Option<Rc<T>> + 'static) -> &'a FerroLocator {
        self.locator.register(
            TypeId::of::<T>(),
            Rc::new(move || func().map(|instance| Rc::new(instance) as ServiceHandle)),
        );
        self.locator
    }

    /// The first lookup calls `func`; later lookups return what it produced.
    pub fn to_lazy(self, func: impl Fn() -> Option<Rc<T>> + 'static) -> &'a FerroLocator {
        let constructed = Cell::new(false);
        let instance: RefCell<Option<ServiceHandle>> = RefCell::new(None);
        self.locator.register(
            TypeId::of::<T>(),
            Rc::new(move || {
                if !constructed.get() {
                    let created = func().map(|instance| Rc::new(instance) as ServiceHandle);
                    *instance.borrow_mut() = created;
                    constructed.set(true);
                }

                instance.borrow().clone()
            }),
        );
        self.locator
    }

    /// The first lookup creates a `TImpl`, which every lookup then returns.
    ///
    /// `upcast` converts the implementation handle to the service handle;
    /// for a trait object service it is the identity closure
    /// `|instance| instance`, which performs the unsizing coercion.
    pub fn to_singleton<TImpl: Default + 'static>(self, upcast: fn(Rc<TImpl>) -> Rc<T>) -> &'a FerroLocator {
        let instance: RefCell<Option<Rc<T>>> = RefCell::new(None);
        self.to_func(move || {
            let existing = instance.borrow().clone();
            Some(match existing {
                Some(existing) => existing,
                None => {
                    let created = upcast(Rc::new(TImpl::default()));
                    *instance.borrow_mut() = Some(created.clone());
                    created
                }
            })
        })
    }

    /// Every lookup creates a new `TImpl`. See
    /// [`to_singleton`](Self::to_singleton) for `upcast`.
    pub fn to_transient<TImpl: Default + 'static>(self, upcast: fn(Rc<TImpl>) -> Rc<T>) -> &'a FerroLocator {
        self.to_func(move || Some(upcast(Rc::new(TImpl::default()))))
    }
}

struct ResolverDisposable {
    resolver: Rc<dyn IFerroDependencyResolver>,
    mutable: Rc<FerroLocator>,
}

impl IDisposable for ResolverDisposable {
    fn dispose(&self) {
        FerroLocator::set_current(self.resolver.clone());
        FerroLocator::set_current_mutable(self.mutable.clone());
    }
}

/// Typed lookups for every [`IFerroDependencyResolver`].
pub trait LocatorExtensions: IFerroDependencyResolver {
    /// Returns the service registered for `T`, which may be a trait object
    /// type (`get_service::<dyn IFoo>()` returns `Option<Rc<dyn IFoo>>`).
    fn get_service<T: ?Sized + 'static>(&self) -> Option<Rc<T>> {
        self.get_service_untyped(TypeId::of::<T>()).and_then(|handle| handle.downcast_ref::<Rc<T>>().cloned())
    }

    /// Returns the service registered for `service_type`.
    ///
    /// # Panics
    /// Panics when the service is not registered.
    fn get_required_service_untyped(&self, service_type: TypeId) -> ServiceHandle {
        match self.get_service_untyped(service_type) {
            Some(service) => service,
            None => panic!("Unable to locate '{service_type:?}'."),
        }
    }

    /// Returns the service registered for `T`.
    ///
    /// # Panics
    /// Panics when the service is not registered.
    fn get_required_service<T: ?Sized + 'static>(&self) -> Rc<T> {
        match self.get_service::<T>() {
            Some(service) => service,
            None => panic!("Unable to locate '{}'.", type_name::<T>()),
        }
    }
}

impl<R: IFerroDependencyResolver + ?Sized> LocatorExtensions for R {}

#[cfg(test)]
mod tests {
    use super::*;

    trait IGreeter {
        fn greet(&self) -> String;
    }

    #[derive(Default)]
    struct English;

    impl IGreeter for English {
        fn greet(&self) -> String {
            "hello".to_string()
        }
    }

    struct Named(&'static str);

    impl IGreeter for Named {
        fn greet(&self) -> String {
            self.0.to_string()
        }
    }

    #[derive(Default)]
    struct Counter {
        value: Cell<i32>,
    }

    #[test]
    fn missing_service_is_none() {
        let locator = FerroLocator::new();
        assert!(locator.get_service::<dyn IGreeter>().is_none());
        assert!(locator.get_service::<Counter>().is_none());
    }

    #[test]
    fn to_constant_works_for_trait_objects_and_concrete_types() {
        let locator = FerroLocator::new();
        let greeter: Rc<dyn IGreeter> = Rc::new(English);
        locator.bind::<dyn IGreeter>().to_constant(greeter.clone()).bind_to_self(Rc::new(Counter::default()));

        let resolved = locator.get_service::<dyn IGreeter>().unwrap();
        assert!(Rc::ptr_eq(&resolved, &greeter));
        assert_eq!(resolved.greet(), "hello");

        let counter = locator.get_required_service::<Counter>();
        counter.value.set(3);
        assert_eq!(locator.get_service::<Counter>().unwrap().value.get(), 3);

        // The trait object registration does not register the implementation type.
        assert!(locator.get_service::<English>().is_none());
    }

    #[test]
    fn rebinding_replaces_the_registration() {
        let locator = FerroLocator::new();
        locator.bind::<dyn IGreeter>().to_constant(Rc::new(Named("first")));
        locator.bind::<dyn IGreeter>().to_constant(Rc::new(Named("second")));
        assert_eq!(locator.get_required_service::<dyn IGreeter>().greet(), "second");
    }

    #[test]
    fn to_func_is_called_for_every_lookup() {
        let locator = FerroLocator::new();
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        locator.bind::<dyn IGreeter>().to_func(move || {
            c.set(c.get() + 1);
            Some(Rc::new(English) as Rc<dyn IGreeter>)
        });
        let a = locator.get_service::<dyn IGreeter>().unwrap();
        let b = locator.get_service::<dyn IGreeter>().unwrap();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn to_lazy_constructs_once() {
        let locator = FerroLocator::new();
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        locator.bind::<dyn IGreeter>().to_lazy(move || {
            c.set(c.get() + 1);
            Some(Rc::new(English) as Rc<dyn IGreeter>)
        });
        assert_eq!(calls.get(), 0);
        let a = locator.get_service::<dyn IGreeter>().unwrap();
        let b = locator.get_service::<dyn IGreeter>().unwrap();
        assert!(Rc::ptr_eq(&a, &b));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn to_singleton_and_to_transient() {
        let locator = FerroLocator::new();
        locator.bind::<dyn IGreeter>().to_singleton::<English>(|instance| instance);
        let a = locator.get_service::<dyn IGreeter>().unwrap();
        let b = locator.get_service::<dyn IGreeter>().unwrap();
        assert!(Rc::ptr_eq(&a, &b));

        locator.bind::<dyn IGreeter>().to_transient::<English>(|instance| instance);
        let a = locator.get_service::<dyn IGreeter>().unwrap();
        let b = locator.get_service::<dyn IGreeter>().unwrap();
        assert!(!Rc::ptr_eq(&a, &b));

        locator.bind_to_self_singleton::<Counter>();
        let a = locator.get_service::<Counter>().unwrap();
        a.value.set(7);
        assert_eq!(locator.get_service::<Counter>().unwrap().value.get(), 7);
    }

    #[test]
    fn enter_scope_inherits_and_restores() {
        // The current locator is per-thread, so this test cannot interfere
        // with tests running on other threads.
        FerroLocator::current_mutable().bind::<dyn IGreeter>().to_constant(Rc::new(Named("outer")));
        let outer = FerroLocator::current_mutable();

        let scope = FerroLocator::enter_scope();
        assert!(!Rc::ptr_eq(&outer, &FerroLocator::current_mutable()));
        // Inherited from the parent scope.
        assert_eq!(FerroLocator::current().get_required_service::<dyn IGreeter>().greet(), "outer");

        FerroLocator::current_mutable().bind::<dyn IGreeter>().to_constant(Rc::new(Named("inner")));
        FerroLocator::current_mutable().bind_to_self(Rc::new(Counter::default()));
        assert_eq!(FerroLocator::current().get_required_service::<dyn IGreeter>().greet(), "inner");

        scope.dispose();
        assert!(Rc::ptr_eq(&outer, &FerroLocator::current_mutable()));
        assert_eq!(FerroLocator::current().get_required_service::<dyn IGreeter>().greet(), "outer");
        assert!(FerroLocator::current().get_service::<Counter>().is_none());
    }

    #[test]
    #[should_panic(expected = "Unable to locate")]
    fn get_required_service_panics_when_missing() {
        FerroLocator::new().get_required_service::<dyn IGreeter>();
    }
}
