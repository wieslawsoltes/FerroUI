//! `ModernDependencyResolver`.

use super::common::{run_swallowing_exceptions, throw_object_disposed};
use super::dependency_resolver::{IMutableDependencyResolver, IReadonlyDependencyResolver};
use super::service_type::{
    ErasedService, LazyDisposal, RegistrationCallback, ServiceFactory, ServiceType,
};
use crate::disposables::{ActionDisposable, BooleanDisposable, EmptyDisposable};
use ferroui_system::IDisposable;
use std::any::TypeId;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const OBJECT_NAME: &str = "ModernDependencyResolver";

/// The key of a registration: the service type and the contract, a missing contract being the
/// empty one (`GetKey`).
type Key = (TypeId, String);

fn key(service_type: ServiceType, contract: Option<&str>) -> Key {
    (service_type.id(), contract.unwrap_or("").to_owned())
}

/// A lazy singleton (upstream registers a `Lazy<object?>` that resolution unwraps).
struct LazyRegistration {
    factory: ServiceFactory,
    value: OnceCell<Option<ErasedService>>,
    disposal: Option<LazyDisposal>,
}

#[derive(Clone)]
enum Registration {
    Factory(ServiceFactory),
    Lazy(Rc<LazyRegistration>),
}

type Registry = HashMap<Key, Rc<Vec<Registration>>>;

/// What disposing the resolver disposes (`_disposables`).
enum Tracked {
    Constant(Rc<dyn Fn()>),
    Lazy(Rc<LazyRegistration>),
}

fn same_callback(a: &RegistrationCallback, b: &RegistrationCallback) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// A resolver that keeps a list of factories per service type and contract
/// (`ModernDependencyResolver`): the last registration wins for `get_service`, `get_services`
/// returns them all in the order of registration, and a registration runs the callbacks of its
/// key.
///
/// The registry is copied on write: a factory or a callback that registers while the registry is
/// read does not change what is being read.
///
/// A missing contract and the empty contract are the same key.
pub struct ModernDependencyResolver {
    snapshot: RefCell<Option<Rc<Registry>>>,
    callback_registry: Rc<RefCell<HashMap<Key, Vec<RegistrationCallback>>>>,
    disposables: RefCell<Vec<Tracked>>,
    disposed: Cell<bool>,
}

impl Default for ModernDependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ModernDependencyResolver {
    /// An empty resolver.
    pub fn new() -> Self {
        Self::with_registry(Registry::new())
    }

    fn with_registry(registry: Registry) -> Self {
        Self {
            snapshot: RefCell::new(Some(Rc::new(registry))),
            callback_registry: Rc::new(RefCell::new(HashMap::new())),
            disposables: RefCell::new(Vec::new()),
            disposed: Cell::new(false),
        }
    }

    /// A new resolver with the registrations of this one, without its callbacks (`Duplicate`).
    /// The lazy singletons are shared with this resolver.
    pub fn duplicate(&self) -> Self {
        match self.snapshot.borrow().as_ref() {
            None => Self::new(),
            Some(registry) => Self::with_registry(
                registry.iter().map(|(key, list)| (key.clone(), Rc::new((**list).clone()))).collect(),
            ),
        }
    }

    fn throw_if_disposed(&self) {
        if self.disposed.get() {
            throw_object_disposed(OBJECT_NAME);
        }
    }

    fn current_snapshot(&self) -> Option<Rc<Registry>> {
        self.snapshot.borrow().clone()
    }

    /// Adds `registration` under `key` and runs the callbacks of the key.
    fn add_registration(&self, key: Key, registration: Registration) {
        self.throw_if_disposed();

        let callbacks_to_run = {
            let Some(snapshot) = self.current_snapshot() else {
                throw_object_disposed(OBJECT_NAME);
            };
            let mut registry: Registry = (*snapshot).clone();
            let mut list: Vec<Registration> =
                registry.get(&key).map(|list| (**list).clone()).unwrap_or_default();
            list.push(registration);
            registry.insert(key.clone(), Rc::new(list));
            *self.snapshot.borrow_mut() = Some(Rc::new(registry));

            self.callback_registry.borrow().get(&key).filter(|list| !list.is_empty()).cloned()
        };

        if let Some(callbacks) = callbacks_to_run {
            self.run_callbacks(callbacks, &key);
        }
    }

    /// Runs each callback with a disposable of its own, and removes those that disposed it
    /// (`RunCallbacks`).
    fn run_callbacks(&self, callbacks: Vec<RegistrationCallback>, key: &Key) {
        let mut to_remove = Vec::new();
        for callback in callbacks {
            let disposable = BooleanDisposable::new();
            callback(&disposable);
            if disposable.is_disposed() {
                to_remove.push(callback);
            }
        }
        if to_remove.is_empty() {
            return;
        }
        let mut registry = self.callback_registry.borrow_mut();
        if let Some(list) = registry.get_mut(key) {
            for removed in &to_remove {
                if let Some(index) = list.iter().position(|c| same_callback(c, removed)) {
                    list.remove(index);
                }
            }
        }
    }

    /// The service of a registration (`Unwrap`): a lazy singleton is created on first use, and
    /// when the resolver was disposed while it was created, it is disposed and the access fails.
    fn resolve(&self, registration: &Registration) -> Option<ErasedService> {
        match registration {
            Registration::Factory(factory) => factory(),
            Registration::Lazy(lazy) => {
                let value = lazy.value.get_or_init(|| (lazy.factory)()).clone();
                if self.disposed.get() {
                    if let (Some(value), Some(disposal)) = (value.as_ref(), lazy.disposal.as_ref()) {
                        disposal(value);
                    }
                    throw_object_disposed(OBJECT_NAME);
                }
                value
            }
        }
    }

    fn release_managed_state(&self) {
        if self.disposed.replace(true) {
            return;
        }
        *self.snapshot.borrow_mut() = None;
        let callbacks = std::mem::take(&mut *self.callback_registry.borrow_mut());
        let disposables = std::mem::take(&mut *self.disposables.borrow_mut());

        // `DisposeCallbacks`.
        for list in callbacks.into_values() {
            for callback in list {
                run_swallowing_exceptions(|| {
                    let disposable = BooleanDisposable::new();
                    callback(&disposable);
                    disposable.dispose();
                });
            }
        }

        // `DisposeTrackedSingletons`: a lazy singleton only when it was created.
        for tracked in disposables {
            run_swallowing_exceptions(|| match tracked {
                Tracked::Constant(disposal) => disposal(),
                Tracked::Lazy(lazy) => {
                    if let (Some(Some(value)), Some(disposal)) = (lazy.value.get(), lazy.disposal.as_ref()) {
                        disposal(value);
                    }
                }
            });
        }
    }
}

impl IReadonlyDependencyResolver for ModernDependencyResolver {
    fn get_service_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Option<ErasedService> {
        let Some(snapshot) = self.current_snapshot() else {
            throw_object_disposed(OBJECT_NAME);
        };
        let registration = snapshot.get(&key(service_type, contract))?.last()?.clone();
        let result = self.resolve(&registration);
        // The resolver may have been disposed while the factory ran.
        self.throw_if_disposed();
        result
    }

    // Deviation (DEVIATIONS.md, Splat): upstream returns a `null` of a factory that gives none in
    // the list; the list holds `Rc`s, so it is left out.
    fn get_services_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Vec<ErasedService> {
        let Some(snapshot) = self.current_snapshot() else {
            throw_object_disposed(OBJECT_NAME);
        };
        let Some(list) = snapshot.get(&key(service_type, contract)).cloned() else {
            return Vec::new();
        };
        let results = list.iter().filter_map(|registration| self.resolve(registration)).collect();
        self.throw_if_disposed();
        results
    }
}

impl IMutableDependencyResolver for ModernDependencyResolver {
    fn has_registration_erased(&self, service_type: ServiceType, contract: Option<&str>) -> bool {
        self.current_snapshot()
            .is_some_and(|snapshot| snapshot.get(&key(service_type, contract)).is_some_and(|list| !list.is_empty()))
    }

    fn register_erased(
        &self,
        factory: ServiceFactory,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.add_registration(key(service_type, contract), Registration::Factory(factory));
    }

    fn register_constant_erased(
        &self,
        value: Option<ErasedService>,
        disposal: Option<Rc<dyn Fn()>>,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        if let Some(disposal) = disposal {
            self.disposables.borrow_mut().push(Tracked::Constant(disposal));
        }
        self.add_registration(
            key(service_type, contract),
            Registration::Factory(Rc::new(move || value.clone())),
        );
    }

    fn register_lazy_singleton_erased(
        &self,
        factory: ServiceFactory,
        disposal: Option<LazyDisposal>,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        let lazy = Rc::new(LazyRegistration { factory, value: OnceCell::new(), disposal });
        self.disposables.borrow_mut().push(Tracked::Lazy(lazy.clone()));
        self.add_registration(key(service_type, contract), Registration::Lazy(lazy));
    }

    fn unregister_current_erased(&self, service_type: ServiceType, contract: Option<&str>) {
        self.throw_if_disposed();
        let Some(snapshot) = self.current_snapshot() else {
            throw_object_disposed(OBJECT_NAME);
        };
        let key = key(service_type, contract);
        let Some(list) = snapshot.get(&key).filter(|list| !list.is_empty()) else {
            return;
        };
        let mut new_list: Vec<Registration> = (**list).clone();
        new_list.pop();
        let mut registry: Registry = (*snapshot).clone();
        registry.insert(key, Rc::new(new_list));
        *self.snapshot.borrow_mut() = Some(Rc::new(registry));
    }

    fn unregister_all_erased(&self, service_type: ServiceType, contract: Option<&str>) {
        self.throw_if_disposed();
        let Some(snapshot) = self.current_snapshot() else {
            throw_object_disposed(OBJECT_NAME);
        };
        let mut registry: Registry = (*snapshot).clone();
        registry.insert(key(service_type, contract), Rc::new(Vec::new()));
        *self.snapshot.borrow_mut() = Some(Rc::new(registry));
    }

    fn service_registration_callback_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
        callback: RegistrationCallback,
    ) -> Rc<dyn IDisposable> {
        let Some(snapshot) = self.current_snapshot() else {
            return EmptyDisposable::instance();
        };
        let key = key(service_type, contract);

        // `AddCallbackToken`.
        self.callback_registry.borrow_mut().entry(key.clone()).or_default().push(callback.clone());
        let registry = self.callback_registry.clone();
        let removed = callback.clone();
        let removal_key = key.clone();
        let disposable: Rc<dyn IDisposable> = ActionDisposable::new(move || {
            if let Some(list) = registry.borrow_mut().get_mut(&removal_key) {
                if let Some(index) = list.iter().position(|c| same_callback(c, &removed)) {
                    list.remove(index);
                }
            }
        });

        // Once per existing registration.
        let count = snapshot.get(&key).map_or(0, |list| list.len());
        for _ in 0..count {
            callback(&*disposable);
        }

        disposable
    }
}

impl IDisposable for ModernDependencyResolver {
    fn dispose(&self) {
        if self.disposed.get() {
            return;
        }
        self.release_managed_state();
    }
}
