//! `GlobalGenericFirstDependencyResolver`.

use super::{ErasedContainer, ErasedContractContainer};
use crate::disposables::{ActionDisposable, EmptyDisposable};
use crate::service_location::common::{run_swallowing_exceptions, throw_object_disposed};
use crate::service_location::dependency_resolver::{
    IMutableDependencyResolver, IReadonlyDependencyResolver,
};
use crate::service_location::service_type::{
    ErasedService, LazyDisposal, RegistrationCallback, ServiceFactory, ServiceType,
};
use ferroui_system::IDisposable;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

const OBJECT_NAME: &str = "GlobalGenericFirstDependencyResolver";

ferroui_system::ferro_thread_local! {
    // `_hasAnyRegistrations`: whether any global resolver has registered anything since the last
    // `clear`.
    static HAS_ANY_REGISTRATIONS: Cell<bool> = const { Cell::new(false) };
}

/// A resolver whose registrations are global: every instance shares one set of containers, per
/// service type and contract (`GlobalGenericFirstDependencyResolver`, after ReactiveMarbles'
/// `DefaultServiceLocator`).
///
/// Disposing an instance disposes the constants and lazy singletons it registered and ends its
/// callbacks, but leaves the shared registrations in place; [`clear`](Self::clear) removes them.
///
/// The registration callbacks of an instance run for a registration or a removal of any service
/// type, as upstream's do.
// Deviation (DEVIATIONS.md, Splat): the containers are per thread, not per process.
pub struct GlobalGenericFirstDependencyResolver {
    callbacks: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
    disposal_actions: RefCell<Vec<Rc<dyn Fn()>>>,
    disposed: Rc<Cell<bool>>,
}

impl Default for GlobalGenericFirstDependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalGenericFirstDependencyResolver {
    /// A resolver over the global containers.
    pub fn new() -> Self {
        Self {
            callbacks: Rc::new(RefCell::new(Vec::new())),
            disposal_actions: RefCell::new(Vec::new()),
            disposed: Rc::new(Cell::new(false)),
        }
    }

    /// A resolver that `configure` registers services with first
    /// (`GlobalGenericFirstDependencyResolver(Action<IMutableDependencyResolver>?)`).
    pub fn with_configuration(configure: impl FnOnce(&dyn IMutableDependencyResolver)) -> Self {
        let resolver = Self::new();
        configure(&resolver);
        resolver
    }

    /// Removes every global registration, of every instance (`Clear`).
    pub fn clear() {
        ErasedContainer::clear_all();
        ErasedContractContainer::clear_all();
        HAS_ANY_REGISTRATIONS.with(|flag| flag.set(false));
    }

    fn has_never_registered() -> bool {
        !HAS_ANY_REGISTRATIONS.with(|flag| flag.get())
    }

    fn mark_registered() {
        HAS_ANY_REGISTRATIONS.with(|flag| flag.set(true));
    }

    fn throw_if_disposed(&self) {
        if self.disposed.get() {
            throw_object_disposed(OBJECT_NAME);
        }
    }

    fn notify_callback_changed(&self) {
        if self.disposed.get() {
            return;
        }
        let callbacks: Vec<Rc<dyn Fn()>> = self.callbacks.borrow().clone();
        for callback in callbacks {
            run_swallowing_exceptions(|| callback());
        }
    }

    fn add_factory(&self, service_type: ServiceType, factory: ServiceFactory, contract: Option<&str>) {
        match contract {
            None => ErasedContainer::add_factory(service_type, factory),
            Some(contract) => ErasedContractContainer::add_factory(service_type, factory, Some(contract)),
        }
    }
}

impl IReadonlyDependencyResolver for GlobalGenericFirstDependencyResolver {
    fn get_service_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Option<ErasedService> {
        if Self::has_never_registered() {
            return None;
        }
        match contract {
            None => ErasedContainer::try_get(service_type),
            Some(contract) => ErasedContractContainer::try_get(service_type, Some(contract)),
        }
    }

    fn get_services_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
    ) -> Vec<ErasedService> {
        if Self::has_never_registered() {
            return Vec::new();
        }
        match contract {
            None => ErasedContainer::get_all(service_type),
            Some(contract) => ErasedContractContainer::get_all(service_type, Some(contract)),
        }
    }
}

impl IMutableDependencyResolver for GlobalGenericFirstDependencyResolver {
    fn has_registration_erased(&self, service_type: ServiceType, contract: Option<&str>) -> bool {
        if Self::has_never_registered() {
            return false;
        }
        match contract {
            None => ErasedContainer::has_registrations(service_type),
            Some(contract) => ErasedContractContainer::has_registrations(service_type, Some(contract)),
        }
    }

    fn register_erased(
        &self,
        factory: ServiceFactory,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.throw_if_disposed();
        Self::mark_registered();
        self.add_factory(service_type, factory, contract);
        self.notify_callback_changed();
    }

    fn register_constant_erased(
        &self,
        value: Option<ErasedService>,
        disposal: Option<Rc<dyn Fn()>>,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.throw_if_disposed();
        Self::mark_registered();
        if let Some(disposal) = disposal {
            self.disposal_actions.borrow_mut().push(disposal);
        }
        match contract {
            None => ErasedContainer::add_instance(service_type, value),
            Some(contract) => ErasedContractContainer::add_instance(service_type, value, Some(contract)),
        }
        self.notify_callback_changed();
    }

    fn register_lazy_singleton_erased(
        &self,
        factory: ServiceFactory,
        disposal: Option<LazyDisposal>,
        service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.throw_if_disposed();
        Self::mark_registered();

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

        self.notify_callback_changed();
    }

    fn unregister_current_erased(&self, service_type: ServiceType, contract: Option<&str>) {
        self.throw_if_disposed();
        match contract {
            None => ErasedContainer::remove_current(service_type),
            Some(contract) => ErasedContractContainer::remove_current(service_type, Some(contract)),
        }
        self.notify_callback_changed();
    }

    fn unregister_all_erased(&self, service_type: ServiceType, contract: Option<&str>) {
        self.throw_if_disposed();
        match contract {
            None => ErasedContainer::clear(service_type),
            Some(contract) => ErasedContractContainer::clear(service_type, Some(contract)),
        }
        self.notify_callback_changed();
    }

    fn service_registration_callback_erased(
        &self,
        service_type: ServiceType,
        contract: Option<&str>,
        callback: RegistrationCallback,
    ) -> Rc<dyn IDisposable> {
        self.throw_if_disposed();

        let wrapped = callback.clone();
        let wrapper: Rc<dyn Fn()> = Rc::new(move || wrapped(&*EmptyDisposable::instance()));
        self.callbacks.borrow_mut().push(wrapper.clone());

        let callbacks = self.callbacks.clone();
        let disposable: Rc<dyn IDisposable> = ActionDisposable::new(move || {
            let mut callbacks = callbacks.borrow_mut();
            if let Some(index) = callbacks
                .iter()
                .position(|c| std::ptr::addr_eq(Rc::as_ptr(c), Rc::as_ptr(&wrapper)))
            {
                callbacks.remove(index);
            }
        });

        // Once per existing registration, without invoking the factories.
        // Deviation (DEVIATIONS.md, Splat): upstream counts a generic registration twice, in the
        // generic container and in the non-generic registry that mirrors it; without the
        // `Type`-based registry each registration counts once.
        let count = match contract {
            None => ErasedContainer::get_count(service_type),
            Some(contract) => ErasedContractContainer::get_count(service_type, Some(contract)),
        };
        for _ in 0..count {
            callback(&*disposable);
        }

        disposable
    }
}

impl IDisposable for GlobalGenericFirstDependencyResolver {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }

        let callbacks: Vec<Rc<dyn Fn()>> = std::mem::take(&mut *self.callbacks.borrow_mut());
        let disposals: Vec<Rc<dyn Fn()>> = std::mem::take(&mut *self.disposal_actions.borrow_mut());

        for callback in callbacks {
            run_swallowing_exceptions(|| callback());
        }
        for disposal in disposals {
            run_swallowing_exceptions(|| disposal());
        }
    }
}
