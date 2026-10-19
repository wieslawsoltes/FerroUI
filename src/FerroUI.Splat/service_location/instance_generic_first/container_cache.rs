//! `ContainerCache<T>` and `ContractContainerCache<T>`: the containers of one service type in the
//! state of an instance resolver.

use super::ResolverState;
use crate::service_location::common::{materialize_registrations, Registration};
use crate::service_location::service_type::{ErasedService, ServiceFactory, ServiceType};

/// The container of a service type without a contract in a resolver state
/// (`ContainerCache<T>.Container`).
pub(crate) struct ContainerCache<'a> {
    state: &'a ResolverState,
    service_type: ServiceType,
}

#[allow(dead_code)]
impl<'a> ContainerCache<'a> {
    /// `ContainerCache<T>.Get(state)`.
    pub(crate) fn get(state: &'a ResolverState, service_type: ServiceType) -> Self {
        Self { state, service_type }
    }

    /// `HasRegistrations`.
    pub(crate) fn has_registrations(&self) -> bool {
        self.state.container(self.service_type).has_items()
    }

    /// `GetCount`.
    pub(crate) fn get_count(&self) -> usize {
        self.state.container(self.service_type).count()
    }

    /// `TryGet`.
    pub(crate) fn try_get(&self) -> Option<ErasedService> {
        let entry = self.state.container(self.service_type);
        if !entry.has_items() {
            return None;
        }
        let registrations = entry.get_snapshot();
        registrations.last()?.resolve()
    }

    /// `GetAll`.
    pub(crate) fn get_all(&self) -> Vec<ErasedService> {
        let registrations = self.state.container(self.service_type).get_snapshot();
        materialize_registrations(&registrations)
    }

    /// `Add(T service)`.
    pub(crate) fn add_instance(&self, service: Option<ErasedService>) {
        self.state.container(self.service_type).add(Registration::from_instance(service));
    }

    /// `Add(Func<T?> factory)`.
    pub(crate) fn add_factory(&self, factory: ServiceFactory) {
        self.state.container(self.service_type).add(Registration::from_factory(factory));
    }

    /// `RemoveCurrent`.
    pub(crate) fn remove_current(&self) {
        self.state.container(self.service_type).remove_current();
    }

    /// `Clear`.
    pub(crate) fn clear(&self) {
        self.state.container(self.service_type).clear();
    }
}

/// The containers of a service type under contracts in a resolver state
/// (`ContractContainerCache<T>.ContractContainer`).
pub(crate) struct ContractContainerCache<'a> {
    state: &'a ResolverState,
    service_type: ServiceType,
}

#[allow(dead_code)]
impl<'a> ContractContainerCache<'a> {
    /// `ContractContainerCache<T>.Get(state)`.
    pub(crate) fn get(state: &'a ResolverState, service_type: ServiceType) -> Self {
        Self { state, service_type }
    }

    /// `HasRegistrations(contract)`.
    pub(crate) fn has_registrations(&self, contract: &str) -> bool {
        self.state
            .contract_entry(self.service_type, contract)
            .is_some_and(|entry| entry.has_items())
    }

    /// `GetCount(contract)`.
    pub(crate) fn get_count(&self, contract: &str) -> usize {
        self.state.contract_entry(self.service_type, contract).map_or(0, |entry| entry.count())
    }

    /// `TryGet(contract)`.
    pub(crate) fn try_get(&self, contract: &str) -> Option<ErasedService> {
        let entry = self.state.contract_entry(self.service_type, contract)?;
        if !entry.has_items() {
            return None;
        }
        let registrations = entry.get_snapshot();
        registrations.last()?.resolve()
    }

    /// `GetAll(contract)`.
    pub(crate) fn get_all(&self, contract: &str) -> Vec<ErasedService> {
        match self.state.contract_entry(self.service_type, contract) {
            Some(entry) => materialize_registrations(&entry.get_snapshot()),
            None => Vec::new(),
        }
    }

    /// `Add(T service, string contract)`.
    pub(crate) fn add_instance(&self, service: Option<ErasedService>, contract: &str) {
        self.state
            .contract_entry_or_add(self.service_type, contract)
            .add(Registration::from_instance(service));
    }

    /// `Add(Func<T?> factory, string contract)`.
    pub(crate) fn add_factory(&self, factory: ServiceFactory, contract: &str) {
        self.state
            .contract_entry_or_add(self.service_type, contract)
            .add(Registration::from_factory(factory));
    }

    /// `RemoveCurrent(contract)`: removes the last registration, and the contract when that was
    /// the last.
    pub(crate) fn remove_current(&self, contract: &str) {
        let Some(entry) = self.state.contract_entry(self.service_type, contract) else {
            return;
        };
        if entry.remove_current() {
            self.state.remove_contract_entry(self.service_type, contract);
        }
    }

    /// `Clear(contract)`.
    pub(crate) fn clear(&self, contract: &str) {
        self.state.remove_contract_entry(self.service_type, contract);
    }

    /// `ClearAll`.
    pub(crate) fn clear_all(&self) {
        self.state.clear_contract_entries(self.service_type);
    }
}
