//! `ContractContainer<T>`: the registrations of a service type under contracts, shared by every
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

ferroui_system::ferro_thread_local! {
    static CONTRACT_CONTAINERS: RefCell<HashMap<TypeId, HashMap<String, ErasedEntry>>> =
        RefCell::new(HashMap::new());
}

/// `NormalizeContract`: a missing contract is the empty one.
fn normalize_contract(contract: Option<&str>) -> &str {
    contract.unwrap_or("")
}

/// The contract container of a service type, type-erased: what the global resolver uses.
pub(crate) struct ErasedContractContainer;

#[allow(dead_code)]
impl ErasedContractContainer {
    fn entry(service_type: ServiceType, contract: Option<&str>) -> Option<ErasedEntry> {
        let key = normalize_contract(contract);
        CONTRACT_CONTAINERS.with(|containers| {
            containers.borrow().get(&service_type.id()).and_then(|entries| entries.get(key)).cloned()
        })
    }

    fn entry_or_add(service_type: ServiceType, contract: Option<&str>) -> ErasedEntry {
        let key = normalize_contract(contract).to_owned();
        CONTRACT_CONTAINERS.with(|containers| {
            containers
                .borrow_mut()
                .entry(service_type.id())
                .or_default()
                .entry(key)
                .or_default()
                .clone()
        })
    }

    fn remove_key(service_type: ServiceType, contract: Option<&str>) {
        let key = normalize_contract(contract);
        CONTRACT_CONTAINERS.with(|containers| {
            if let Some(entries) = containers.borrow_mut().get_mut(&service_type.id()) {
                entries.remove(key);
            }
        });
    }

    /// `Add(T service, string? contract)`.
    pub(crate) fn add_instance(
        service_type: ServiceType,
        service: Option<ErasedService>,
        contract: Option<&str>,
    ) {
        Self::entry_or_add(service_type, contract).add(Registration::from_instance(service));
    }

    /// `Add(Func<T?> factory, string? contract)`.
    pub(crate) fn add_factory(service_type: ServiceType, factory: ServiceFactory, contract: Option<&str>) {
        Self::entry_or_add(service_type, contract).add(Registration::from_factory(factory));
    }

    /// `TryGet`.
    pub(crate) fn try_get(service_type: ServiceType, contract: Option<&str>) -> Option<ErasedService> {
        let entry = Self::entry(service_type, contract)?;
        if !entry.has_items() {
            return None;
        }
        let registrations = entry.get_snapshot();
        registrations.last()?.resolve()
    }

    /// `GetAll`.
    pub(crate) fn get_all(service_type: ServiceType, contract: Option<&str>) -> Vec<ErasedService> {
        match Self::entry(service_type, contract) {
            Some(entry) => materialize_registrations(&entry.get_snapshot()),
            None => Vec::new(),
        }
    }

    /// `RemoveCurrent`: removes the last registration, and the contract when that was the last.
    pub(crate) fn remove_current(service_type: ServiceType, contract: Option<&str>) {
        let Some(entry) = Self::entry(service_type, contract) else {
            return;
        };
        if entry.remove_current() {
            Self::remove_key(service_type, contract);
        }
    }

    /// `Clear(string contract)`.
    pub(crate) fn clear(service_type: ServiceType, contract: Option<&str>) {
        Self::remove_key(service_type, contract);
    }

    /// `ClearAll` of every service type.
    pub(crate) fn clear_all() {
        CONTRACT_CONTAINERS.with(|containers| {
            for entries in containers.borrow_mut().values_mut() {
                entries.clear();
            }
        });
    }

    /// `ClearAll` of one service type.
    pub(crate) fn clear_all_of(service_type: ServiceType) {
        CONTRACT_CONTAINERS.with(|containers| {
            if let Some(entries) = containers.borrow_mut().get_mut(&service_type.id()) {
                entries.clear();
            }
        });
    }

    /// `HasRegistrations`.
    pub(crate) fn has_registrations(service_type: ServiceType, contract: Option<&str>) -> bool {
        Self::entry(service_type, contract).is_some_and(|entry| entry.has_items())
    }

    /// `GetCount`.
    pub(crate) fn get_count(service_type: ServiceType, contract: Option<&str>) -> usize {
        Self::entry(service_type, contract).map_or(0, |entry| entry.count())
    }
}

/// `ContractContainer<T>`: the typed view of the contract container of the service type `S`.
#[allow(dead_code)]
pub(crate) struct ContractContainer<S: ?Sized>(PhantomData<S>);

#[allow(dead_code)]
impl<S: ?Sized + 'static> ContractContainer<S> {
    /// `Add(T service, string? contract)`.
    pub(crate) fn add(service: Option<Rc<S>>, contract: Option<&str>) {
        ErasedContractContainer::add_instance(ServiceType::of::<S>(), service.map(erase_service), contract);
    }

    /// `Add(Func<T?> factory, string? contract)`.
    pub(crate) fn add_factory(factory: impl Fn() -> Option<Rc<S>> + 'static, contract: Option<&str>) {
        ErasedContractContainer::add_factory(
            ServiceType::of::<S>(),
            Rc::new(move || factory().map(erase_service)),
            contract,
        );
    }

    /// `TryGet`.
    pub(crate) fn try_get(contract: Option<&str>) -> Option<Rc<S>> {
        ErasedContractContainer::try_get(ServiceType::of::<S>(), contract)
            .and_then(|value| unerase_service::<S>(&value))
    }

    /// `GetAll`.
    pub(crate) fn get_all(contract: Option<&str>) -> Vec<Rc<S>> {
        ErasedContractContainer::get_all(ServiceType::of::<S>(), contract)
            .iter()
            .filter_map(unerase_service::<S>)
            .collect()
    }

    /// `RemoveCurrent`.
    pub(crate) fn remove_current(contract: Option<&str>) {
        ErasedContractContainer::remove_current(ServiceType::of::<S>(), contract);
    }

    /// `Clear`.
    pub(crate) fn clear(contract: Option<&str>) {
        ErasedContractContainer::clear(ServiceType::of::<S>(), contract);
    }

    /// `ClearAll`.
    pub(crate) fn clear_all() {
        ErasedContractContainer::clear_all_of(ServiceType::of::<S>());
    }

    /// `HasRegistrations`.
    pub(crate) fn has_registrations(contract: Option<&str>) -> bool {
        ErasedContractContainer::has_registrations(ServiceType::of::<S>(), contract)
    }

    /// `GetCount`.
    pub(crate) fn get_count(contract: Option<&str>) -> usize {
        ErasedContractContainer::get_count(ServiceType::of::<S>(), contract)
    }
}
