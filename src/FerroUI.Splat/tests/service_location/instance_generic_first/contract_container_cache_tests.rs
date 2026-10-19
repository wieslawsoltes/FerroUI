//! Ported from `Splat.Tests/ServiceLocation/InstanceGenericFirst/ContractContainerCacheTests.cs`
//! of Splat.
//! Adapted: `ContractContainerCache<T>.Get(state)` is
//! `ContractContainerCache::get(&state, ServiceType::of::<T>())`, a view of the contract containers
//! the state owns, over type-erased services.

use crate::service_location::instance_generic_first::{ContractContainerCache, ResolverState};
use crate::service_location::{erase_service, ServiceType};
use std::rc::Rc;

const FIRST_CONTRACT: &str = "first";
const SECOND_CONTRACT: &str = "second";
const EMPTY_COUNT: usize = 0;

struct ViewModelOne;

#[test]
fn get_all_for_unregistered_contract_returns_empty() {
    let state = ResolverState::new();
    let container = ContractContainerCache::get(&state, ServiceType::of::<ViewModelOne>());
    container.add_instance(Some(erase_service(Rc::new(ViewModelOne))), FIRST_CONTRACT);
    let result = container.get_all(SECOND_CONTRACT);
    assert_eq!(result.len(), EMPTY_COUNT);
}

#[test]
fn clear_all_removes_registrations_for_every_contract() {
    let state = ResolverState::new();
    let container = ContractContainerCache::get(&state, ServiceType::of::<ViewModelOne>());
    container.add_instance(Some(erase_service(Rc::new(ViewModelOne))), FIRST_CONTRACT);
    container.add_instance(Some(erase_service(Rc::new(ViewModelOne))), SECOND_CONTRACT);
    assert!(container.has_registrations(FIRST_CONTRACT));
    assert!(container.has_registrations(SECOND_CONTRACT));
    container.clear_all();
    assert!(!container.has_registrations(FIRST_CONTRACT));
    assert!(!container.has_registrations(SECOND_CONTRACT));
    assert_eq!(container.get_count(FIRST_CONTRACT), EMPTY_COUNT);
    assert_eq!(container.get_count(SECOND_CONTRACT), EMPTY_COUNT);
}
