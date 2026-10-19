//! `Container<T>`: the registrations of a service type without a contract, shared by every
//! global resolver.

use crate::service_location::common::{materialize_registrations, ErasedEntry, Registration};
use crate::service_location::service_type::{
    erase_service, unerase_service, ErasedService, ServiceFactory, ServiceType,
};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

// Deviation (DEVIATIONS.md, Splat): `Container<T>` is a static class per service type upstream,
// process-wide; here one map per thread keyed by the type, since the services are `Rc`.
ferroui_system::ferro_thread_local! {
    static CONTAINERS: RefCell<HashMap<TypeId, ErasedEntry>> = RefCell::new(HashMap::new());
}

/// The container of a service type, type-erased: what the global resolver uses.
pub(crate) struct ErasedContainer;

#[allow(dead_code)]
impl ErasedContainer {
    fn entry(service_type: ServiceType) -> Option<ErasedEntry> {
        CONTAINERS.with(|containers| containers.borrow().get(&service_type.id()).cloned())
    }

    fn entry_or_add(service_type: ServiceType) -> ErasedEntry {
        CONTAINERS.with(|containers| {
            containers.borrow_mut().entry(service_type.id()).or_default().clone()
        })
    }

    /// `HasRegistrations`.
    pub(crate) fn has_registrations(service_type: ServiceType) -> bool {
        Self::entry(service_type).is_some_and(|entry| entry.has_items())
    }

    /// `GetCount`.
    pub(crate) fn get_count(service_type: ServiceType) -> usize {
        Self::entry(service_type).map_or(0, |entry| entry.count())
    }

    /// `Add(T service)`.
    pub(crate) fn add_instance(service_type: ServiceType, service: Option<ErasedService>) {
        Self::entry_or_add(service_type).add(Registration::from_instance(service));
    }

    /// `Add(Func<T?> factory)`.
    pub(crate) fn add_factory(service_type: ServiceType, factory: ServiceFactory) {
        Self::entry_or_add(service_type).add(Registration::from_factory(factory));
    }

    /// `TryGet`: the value of the last registration, `None` when there is none or it gives none.
    pub(crate) fn try_get(service_type: ServiceType) -> Option<ErasedService> {
        let entry = Self::entry(service_type)?;
        if !entry.has_items() {
            return None;
        }
        let registrations = entry.get_snapshot();
        registrations.last()?.resolve()
    }

    /// `GetAll`.
    pub(crate) fn get_all(service_type: ServiceType) -> Vec<ErasedService> {
        match Self::entry(service_type) {
            Some(entry) if entry.has_items() => materialize_registrations(&entry.get_snapshot()),
            _ => Vec::new(),
        }
    }

    /// `RemoveCurrent`.
    pub(crate) fn remove_current(service_type: ServiceType) {
        if let Some(entry) = Self::entry(service_type) {
            entry.remove_current();
        }
    }

    /// `Clear`.
    pub(crate) fn clear(service_type: ServiceType) {
        if let Some(entry) = Self::entry(service_type) {
            entry.clear();
        }
    }

    /// Clears the containers of every service type (the clear actions of the containers that
    /// `GlobalGenericFirstDependencyResolver.Clear` runs).
    pub(crate) fn clear_all() {
        let entries: Vec<ErasedEntry> =
            CONTAINERS.with(|containers| containers.borrow().values().cloned().collect());
        for entry in entries {
            entry.clear();
        }
    }
}

/// `Container<T>`: the typed view of the container of the service type `S`.
#[allow(dead_code)]
pub(crate) struct Container<S: ?Sized>(PhantomData<S>);

#[allow(dead_code)]
impl<S: ?Sized + 'static> Container<S> {
    /// `HasRegistrations`.
    pub(crate) fn has_registrations() -> bool {
        ErasedContainer::has_registrations(ServiceType::of::<S>())
    }

    /// `GetCount`.
    pub(crate) fn get_count() -> usize {
        ErasedContainer::get_count(ServiceType::of::<S>())
    }

    /// `Add(T service)`.
    pub(crate) fn add(service: Option<Rc<S>>) {
        ErasedContainer::add_instance(ServiceType::of::<S>(), service.map(erase_service));
    }

    /// `Add(Func<T?> factory)`.
    pub(crate) fn add_factory(factory: impl Fn() -> Option<Rc<S>> + 'static) {
        ErasedContainer::add_factory(
            ServiceType::of::<S>(),
            Rc::new(move || factory().map(erase_service)),
        );
    }

    /// `TryGet`.
    pub(crate) fn try_get() -> Option<Rc<S>> {
        ErasedContainer::try_get(ServiceType::of::<S>()).and_then(|value| unerase_service::<S>(&value))
    }

    /// `GetAll`.
    pub(crate) fn get_all() -> Vec<Rc<S>> {
        ErasedContainer::get_all(ServiceType::of::<S>())
            .iter()
            .filter_map(unerase_service::<S>)
            .collect()
    }

    /// `RemoveCurrent`.
    pub(crate) fn remove_current() {
        ErasedContainer::remove_current(ServiceType::of::<S>());
    }

    /// `Clear`.
    pub(crate) fn clear() {
        ErasedContainer::clear(ServiceType::of::<S>());
    }
}
