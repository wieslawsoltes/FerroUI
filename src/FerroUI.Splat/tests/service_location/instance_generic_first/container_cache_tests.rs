//! Ported from `Splat.Tests/ServiceLocation/InstanceGenericFirst/ContainerCacheTests.cs` of Splat.
//! Adapted: `ContainerCache<T>.Get(state)` is `ContainerCache::get(&state, ServiceType::of::<T>())`,
//! a view of the container the state owns, over type-erased services (`erase_service`,
//! `unerase_service`); `ReferenceEquals` of two containers of a state compares the entries the
//! state hands out; `TryGet(out T)` returns an `Option`.

use crate::service_location::instance_generic_first::{ContainerCache, ResolverState};
use crate::service_location::{erase_service, unerase_service, ServiceType};
use std::cell::Cell;
use std::rc::Rc;

const FIRST_STATE_VALUE: i32 = 100;
const SECOND_STATE_VALUE: i32 = 200;
const FACTORY_VALUE: i32 = 42;
const FIRST_VALUE: i32 = 1;
const SECOND_VALUE: i32 = 2;
const THIRD_VALUE: i32 = 3;
const EXPECTED_REGISTRATION_COUNT: usize = 3;

struct TestService {
    value: i32,
}

fn service_type() -> ServiceType {
    ServiceType::of::<TestService>()
}

fn add(container: &ContainerCache<'_>, value: i32) {
    container.add_instance(Some(erase_service(Rc::new(TestService { value }))));
}

fn try_get(container: &ContainerCache<'_>) -> Option<Rc<TestService>> {
    container.try_get().and_then(|service| unerase_service::<TestService>(&service))
}

#[test]
fn container_cache_should_isolate_per_resolver_state() {
    let state1 = ResolverState::new();
    let state2 = ResolverState::new();
    let container1 = ContainerCache::get(&state1, service_type());
    let container2 = ContainerCache::get(&state2, service_type());
    add(&container1, FIRST_STATE_VALUE);
    add(&container2, SECOND_STATE_VALUE);
    let service1 = try_get(&container1).expect("a service in the first state");
    let service2 = try_get(&container2).expect("a service in the second state");
    assert_eq!(service1.value, FIRST_STATE_VALUE);
    assert_eq!(service2.value, SECOND_STATE_VALUE);
}

#[test]
fn container_cache_should_reuse_container_for_same_state() {
    let state = ResolverState::new();
    let container1 = state.container(service_type());
    let container2 = state.container(service_type());
    assert!(Rc::ptr_eq(&container1, &container2));
}

#[test]
fn container_cache_should_return_empty_for_new_state() {
    let state = ResolverState::new();
    let container = ContainerCache::get(&state, service_type());
    assert!(try_get(&container).is_none());
    assert!(!container.has_registrations());
}

#[test]
fn container_cache_should_handle_factory_registrations() {
    let state = ResolverState::new();
    let container = ContainerCache::get(&state, service_type());
    let call_count = Rc::new(Cell::new(0));
    let count = call_count.clone();
    container.add_factory(Rc::new(move || {
        count.set(count.get() + 1);
        Some(erase_service(Rc::new(TestService { value: FACTORY_VALUE })))
    }));
    let service = try_get(&container).expect("a service");
    assert_eq!(service.value, FACTORY_VALUE);
    assert_eq!(call_count.get(), 1);
}

#[test]
fn container_cache_should_return_most_recent_registration() {
    let state = ResolverState::new();
    let container = ContainerCache::get(&state, service_type());
    add(&container, FIRST_VALUE);
    add(&container, SECOND_VALUE);
    add(&container, THIRD_VALUE);
    assert_eq!(try_get(&container).expect("a service").value, THIRD_VALUE);
}

#[test]
fn container_cache_should_remove_current() {
    let state = ResolverState::new();
    let container = ContainerCache::get(&state, service_type());
    add(&container, FIRST_VALUE);
    add(&container, SECOND_VALUE);
    container.remove_current();
    assert_eq!(try_get(&container).expect("a service").value, FIRST_VALUE);
}

#[test]
fn container_cache_should_clear_all_registrations() {
    let state = ResolverState::new();
    let container = ContainerCache::get(&state, service_type());
    add(&container, FIRST_VALUE);
    add(&container, SECOND_VALUE);
    container.clear();
    assert!(try_get(&container).is_none());
    assert!(!container.has_registrations());
}

#[test]
fn container_cache_get_all_should_return_all_registrations() {
    let state = ResolverState::new();
    let container = ContainerCache::get(&state, service_type());
    add(&container, FIRST_VALUE);
    add(&container, SECOND_VALUE);
    add(&container, THIRD_VALUE);
    let all: Vec<Rc<TestService>> =
        container.get_all().iter().filter_map(unerase_service::<TestService>).collect();
    assert_eq!(all.len(), EXPECTED_REGISTRATION_COUNT);
    assert_eq!(all[0].value, FIRST_VALUE);
    assert_eq!(all[1].value, SECOND_VALUE);
    assert_eq!(all[all.len() - 1].value, THIRD_VALUE);
}
