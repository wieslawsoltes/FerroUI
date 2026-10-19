//! `ResolverState`: the registrations of one instance resolver.

use crate::service_location::common::ErasedEntry;
use crate::service_location::service_type::ServiceType;
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// The state of an instance resolver: an identifier, whether anything was registered, and the
/// containers of its service types (`ResolverState`).
///
/// Upstream keeps the containers in `ConditionalWeakTable`s keyed by the state, one per service
/// type (`ContainerCache<T>`, `ContractContainerCache<T>`, `ServiceTypeRegistryCache`), so that
/// they live as long as the state; here the state owns them.
// Deviation (DEVIATIONS.md, Splat): the containers are fields of the state instead of weak tables
// keyed by it.
pub(crate) struct ResolverState {
    id: u64,
    has_any_registrations: Cell<bool>,
    pub(crate) containers: RefCell<HashMap<TypeId, ErasedEntry>>,
    pub(crate) contract_containers: RefCell<HashMap<TypeId, HashMap<String, ErasedEntry>>>,
}

impl Default for ResolverState {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl ResolverState {
    pub(crate) fn new() -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed) + 1,
            has_any_registrations: Cell::new(false),
            containers: RefCell::new(HashMap::new()),
            contract_containers: RefCell::new(HashMap::new()),
        }
    }

    /// `Id`: unique per state.
    pub(crate) fn id(&self) -> u64 {
        self.id
    }

    /// `HasAnyRegistrations`.
    pub(crate) fn has_any_registrations(&self) -> bool {
        self.has_any_registrations.get()
    }

    /// `MarkHasRegistrations`: `true` when this call was the first.
    pub(crate) fn mark_has_registrations(&self) -> bool {
        !self.has_any_registrations.replace(true)
    }

    /// The container of `service_type` without a contract, created on first use
    /// (`ContainerCache<T>.Get(state)`).
    pub(crate) fn container(&self, service_type: ServiceType) -> ErasedEntry {
        self.containers.borrow_mut().entry(service_type.id()).or_default().clone()
    }

    /// The container of `service_type` under `contract`, if it exists.
    pub(crate) fn contract_entry(&self, service_type: ServiceType, contract: &str) -> Option<ErasedEntry> {
        self.contract_containers
            .borrow()
            .get(&service_type.id())
            .and_then(|entries| entries.get(contract))
            .cloned()
    }

    /// The container of `service_type` under `contract`, created on first use.
    pub(crate) fn contract_entry_or_add(&self, service_type: ServiceType, contract: &str) -> ErasedEntry {
        self.contract_containers
            .borrow_mut()
            .entry(service_type.id())
            .or_default()
            .entry(contract.to_owned())
            .or_default()
            .clone()
    }

    /// Removes the container of `service_type` under `contract`.
    pub(crate) fn remove_contract_entry(&self, service_type: ServiceType, contract: &str) {
        if let Some(entries) = self.contract_containers.borrow_mut().get_mut(&service_type.id()) {
            entries.remove(contract);
        }
    }

    /// Removes the containers of `service_type` under every contract (`ClearAll`).
    pub(crate) fn clear_contract_entries(&self, service_type: ServiceType) {
        if let Some(entries) = self.contract_containers.borrow_mut().get_mut(&service_type.id()) {
            entries.clear();
        }
    }
}
