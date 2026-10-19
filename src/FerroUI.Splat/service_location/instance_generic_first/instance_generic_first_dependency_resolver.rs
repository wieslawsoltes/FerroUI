//! `InstanceGenericFirstDependencyResolver`.

use super::{ContainerCache, ContractContainerCache, ResolverState};
use crate::disposables::{ActionDisposable, EmptyDisposable};
use crate::service_location::common::{run_swallowing_exceptions, throw_object_disposed, Entry};
use crate::service_location::dependency_resolver::{
    IMutableDependencyResolver, IReadonlyDependencyResolver,
};
use crate::service_location::service_type::{
    ErasedService, LazyDisposal, RegistrationCallback, ServiceFactory, ServiceType,
};
use ferroui_system::IDisposable;
use std::any::TypeId;
use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

const OBJECT_NAME: &str = "InstanceGenericFirstDependencyResolver";

type CallbackKey = (TypeId, Option<String>);
type CallbackRegistry = Rc<RefCell<Option<HashMap<CallbackKey, Rc<Entry<RegistrationCallback>>>>>>;

fn same_callback(a: &RegistrationCallback, b: &RegistrationCallback) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// A resolver whose registrations belong to the instance, kept per service type and contract
/// (`InstanceGenericFirstDependencyResolver`). It is the resolver [`AppLocator`] starts with.
///
/// A registration runs the callbacks of its service type and contract; disposing the resolver
/// drops its registrations, disposes the constants and the created lazy singletons that were
/// registered as disposable, and runs every callback once more.
///
/// [`AppLocator`]: crate::AppLocator
pub struct InstanceGenericFirstDependencyResolver {
    state: RefCell<Rc<ResolverState>>,
    callback_registry: CallbackRegistry,
    disposal_actions: RefCell<Vec<Rc<dyn Fn()>>>,
    disposed: Rc<Cell<bool>>,
}

impl Default for InstanceGenericFirstDependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceGenericFirstDependencyResolver {
    /// An empty resolver.
    pub fn new() -> Self {
        Self {
            state: RefCell::new(Rc::new(ResolverState::new())),
            callback_registry: Rc::new(RefCell::new(None)),
            disposal_actions: RefCell::new(Vec::new()),
            disposed: Rc::new(Cell::new(false)),
        }
    }

    /// A resolver that `configure` registers services with first
    /// (`InstanceGenericFirstDependencyResolver(Action<IMutableDependencyResolver>?)`).
    pub fn with_configuration(configure: impl FnOnce(&dyn IMutableDependencyResolver)) -> Self {
        let resolver = Self::new();
        configure(&resolver);
        resolver
    }

    fn state(&self) -> Rc<ResolverState> {
        self.state.borrow().clone()
    }

    /// Whether nothing was registered (the inverted `HasAnyRegistrations` of upstream).
    fn has_no_registrations(&self) -> bool {
        !self.state().has_any_registrations()
    }

    fn mark_registered(&self) {
        let state = self.state();
        if !state.has_any_registrations() {
            state.mark_has_registrations();
        }
    }

    fn throw_if_disposed(&self) {
        if self.disposed.get() {
            throw_object_disposed(OBJECT_NAME);
        }
    }

    fn notify_callback_changed(&self, service_type: ServiceType, contract: Option<&str>) {
        if self.disposed.get() {
            return;
        }
        let key: CallbackKey = (service_type.id(), contract.map(str::to_owned));
        let snapshot = {
            let registry = self.callback_registry.borrow();
            let Some(entry) = registry.as_ref().and_then(|registry| registry.get(&key)) else {
                return;
            };
            entry.get_snapshot()
        };
        for callback in snapshot.iter() {
            run_swallowing_exceptions(|| callback(&*EmptyDisposable::instance()));
        }
    }

    fn add_callback(&self, service_type: ServiceType, contract: Option<&str>, callback: RegistrationCallback) -> Rc<dyn IDisposable> {
        let key: CallbackKey = (service_type.id(), contract.map(str::to_owned));
        self.callback_registry
            .borrow_mut()
            .get_or_insert_with(HashMap::new)
            .entry(key.clone())
            .or_default()
            .add(callback.clone());

        let registry = self.callback_registry.clone();
        ActionDisposable::new(move || {
            let mut registry = registry.borrow_mut();
            let Some(map) = registry.as_mut() else {
                return;
            };
            let Some(entry) = map.get(&key).cloned() else {
                return;
            };
            if !entry.remove_where(|c| same_callback(c, &callback)) {
                return;
            }
            if entry.count() == 0 {
                map.remove(&key);
            }
        })
    }

    fn drain_callbacks(&self) -> Vec<RegistrationCallback> {
        let registry = self.callback_registry.borrow_mut().take();
        let mut flattened = Vec::new();
        if let Some(registry) = registry {
            for entry in registry.values() {
                entry.copy_items_to(&mut flattened);
            }
        }
        flattened
    }

    fn add_factory(&self, service_type: ServiceType, factory: ServiceFactory, contract: Option<&str>) {
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).add_factory(factory),
            Some(contract) => ContractContainerCache::get(&state, service_type).add_factory(factory, contract),
        }
    }
}

impl IReadonlyDependencyResolver for InstanceGenericFirstDependencyResolver {
    fn get_service_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Option<ErasedService> {
        if self.has_no_registrations() {
            return None;
        }
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).try_get(),
            Some(contract) => ContractContainerCache::get(&state, service_type).try_get(contract),
        }
    }

    fn get_services_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Vec<ErasedService> {
        if self.has_no_registrations() {
            return Vec::new();
        }
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).get_all(),
            Some(contract) => ContractContainerCache::get(&state, service_type).get_all(contract),
        }
    }
}

impl IMutableDependencyResolver for InstanceGenericFirstDependencyResolver {
    fn has_registration_erased(&self, service_type: ServiceType, contract: Option<&str>) -> bool {
        if self.has_no_registrations() {
            return false;
        }
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).has_registrations(),
            Some(contract) => ContractContainerCache::get(&state, service_type).has_registrations(contract),
        }
    }

    fn register_erased(
        &self,
        factory: ServiceFactory,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.throw_if_disposed();
        self.mark_registered();
        self.add_factory(service_type, factory, contract);
        self.notify_callback_changed(service_type, contract);
    }

    fn register_constant_erased(
        &self,
        value: Option<ErasedService>,
        disposal: Option<Rc<dyn Fn()>>,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.throw_if_disposed();
        self.mark_registered();
        if let Some(disposal) = disposal {
            self.disposal_actions.borrow_mut().push(disposal);
        }
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).add_instance(value),
            Some(contract) => ContractContainerCache::get(&state, service_type).add_instance(value, contract),
        }
        self.notify_callback_changed(service_type, contract);
    }

    fn register_lazy_singleton_erased(
        &self,
        factory: ServiceFactory,
        disposal: Option<LazyDisposal>,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.throw_if_disposed();
        self.mark_registered();

        let lazy: Rc<OnceCell<Option<ErasedService>>> = Rc::new(OnceCell::new());

        // The value of the lazy singleton; when the resolver was disposed while it was created,
        // it is disposed and the access fails.
        let value_lazy = lazy.clone();
        let value_disposal = disposal.clone();
        let disposed = self.disposed.clone();
        let value_factory: ServiceFactory = Rc::new(move || {
            let value = value_lazy.get_or_init(|| factory()).clone();
            if disposed.get() {
                if let (Some(value), Some(disposal)) = (value.as_ref(), value_disposal.as_ref()) {
                    disposal(value);
                }
                throw_object_disposed(OBJECT_NAME);
            }
            value
        });
        self.add_factory(service_type, value_factory, contract);

        // `AddLazyDisposal`: dispose the value with the resolver, only when it was created.
        if let Some(disposal) = disposal {
            self.disposal_actions.borrow_mut().push(Rc::new(move || {
                if let Some(Some(value)) = lazy.get() {
                    disposal(value);
                }
            }));
        }

        self.notify_callback_changed(service_type, contract);
    }

    fn unregister_current_erased(&self, service_type: ServiceType, contract: Option<&str>) {
        self.throw_if_disposed();
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).remove_current(),
            Some(contract) => ContractContainerCache::get(&state, service_type).remove_current(contract),
        }
    }

    fn unregister_all_erased(&self, service_type: ServiceType, contract: Option<&str>) {
        self.throw_if_disposed();
        let state = self.state();
        match contract {
            None => ContainerCache::get(&state, service_type).clear(),
            Some(contract) => ContractContainerCache::get(&state, service_type).clear(contract),
        }
    }

    fn service_registration_callback_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
        callback: RegistrationCallback,
    ) -> Rc<dyn IDisposable> {
        self.throw_if_disposed();

        let disposable = self.add_callback(service_type, contract, callback.clone());

        // Once per existing registration, without invoking the factories.
        // Deviation (DEVIATIONS.md, Splat): upstream counts a generic registration twice, in the
        // generic container and in the non-generic registry that mirrors it; without the
        // `Type`-based registry each registration counts once.
        let state = self.state();
        let count = match contract {
            None => ContainerCache::get(&state, service_type).get_count(),
            Some(contract) => ContractContainerCache::get(&state, service_type).get_count(contract),
        };
        for _ in 0..count {
            callback(&*disposable);
        }

        disposable
    }
}

impl IDisposable for InstanceGenericFirstDependencyResolver {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        // A new state, so that later lookups find nothing; the old one is dropped with its
        // registrations.
        let _old_state = self.state.replace(Rc::new(ResolverState::new()));

        let callbacks = self.drain_callbacks();
        let disposals: Vec<Rc<dyn Fn()>> = std::mem::take(&mut *self.disposal_actions.borrow_mut());

        for callback in callbacks {
            run_swallowing_exceptions(|| callback(&*EmptyDisposable::instance()));
        }
        for disposal in disposals {
            run_swallowing_exceptions(|| disposal());
        }
    }
}
