//! Ported from `Splat.Tests/ServiceLocation/GlobalGenericFirst/ContainerTests.cs` of Splat.
//! Adapted: `Container<T>` is the typed view `Container::<S>` over the containers of the thread,
//! holding `Rc<S>` (`Container<int?>` is `Container::<i32>` with a factory that returns `Some`);
//! `TryGet(out T)` returns an `Option`. The containers are per thread and each test runs on a
//! thread of its own, so the `[Before(Test)]`/`[After(Test)]` reset is the `clear` at the start.

use crate::service_location::global_generic_first::Container;
use std::cell::Cell;
use std::rc::Rc;

const FIRST: &str = "first";
const SECOND: &str = "second";
const THIRD: &str = "third";
const THREE_ITEMS: usize = 3;
const SAMPLE_VALUE: i32 = 42;
const FIRST_VALUE: i32 = 1;
const SECOND_VALUE: i32 = 2;
const THIRD_VALUE: i32 = 3;
const SECOND_INDEX: usize = 1;
const THIRD_INDEX: usize = 2;

#[derive(Debug, PartialEq)]
struct TestService {
    id: i32,
    name: Option<String>,
}

fn reset_containers() {
    Container::<String>::clear();
    Container::<i32>::clear();
    Container::<TestService>::clear();
}

fn add_string(value: &str) {
    Container::<String>::add(Some(Rc::new(value.to_owned())));
}

fn try_get_string() -> Option<String> {
    Container::<String>::try_get().map(|value| (*value).clone())
}

#[test]
fn has_registrations_when_empty_returns_false() {
    reset_containers();
    assert!(!Container::<String>::has_registrations());
}

#[test]
fn has_registrations_after_adding_returns_true() {
    reset_containers();
    add_string("test");
    assert!(Container::<String>::has_registrations());
}

#[test]
fn add_with_instance_stores_instance() {
    reset_containers();
    add_string("test value");
    assert_eq!(try_get_string().as_deref(), Some("test value"));
}

#[test]
fn add_with_factory_stores_factory() {
    reset_containers();
    Container::<i32>::add_factory(|| Some(Rc::new(42)));
    assert_eq!(Container::<i32>::try_get().map(|value| *value), Some(42));
}

#[test]
fn add_multiple_instances_returns_latest() {
    reset_containers();
    add_string(FIRST);
    add_string(SECOND);
    add_string(THIRD);
    assert_eq!(try_get_string().as_deref(), Some(THIRD));
}

#[test]
fn try_get_when_empty_returns_false() {
    reset_containers();
    assert!(Container::<String>::try_get().is_none());
}

#[test]
fn try_get_with_factory_returning_null_returns_false() {
    reset_containers();
    Container::<String>::add_factory(|| None);
    assert!(Container::<String>::try_get().is_none());
}

#[test]
fn try_get_invokes_factory_each_time() {
    reset_containers();
    let invocation_count = Rc::new(Cell::new(0));
    let count = invocation_count.clone();
    Container::<i32>::add_factory(move || {
        count.set(count.get() + 1);
        Some(Rc::new(count.get()))
    });
    let result1 = Container::<i32>::try_get().map(|value| *value);
    let result2 = Container::<i32>::try_get().map(|value| *value);
    assert_eq!(invocation_count.get(), 2);
    assert_eq!(result1, Some(FIRST_VALUE));
    assert_eq!(result2, Some(SECOND_VALUE));
}

#[test]
fn get_all_when_empty_returns_empty_array() {
    reset_containers();
    assert_eq!(Container::<String>::get_all().len(), 0);
}

#[test]
fn get_all_with_multiple_registrations_returns_all_values() {
    reset_containers();
    add_string(FIRST);
    add_string(SECOND);
    add_string(THIRD);
    let result = Container::<String>::get_all();
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(*result[0], FIRST);
    assert_eq!(*result[SECOND_INDEX], SECOND);
    assert_eq!(*result[THIRD_INDEX], THIRD);
}

#[test]
fn get_all_with_factories_invokes_all_factories() {
    reset_containers();
    let invocation_count = Rc::new(Cell::new(0usize));
    for value in [FIRST_VALUE, SECOND_VALUE, THIRD_VALUE] {
        let count = invocation_count.clone();
        Container::<i32>::add_factory(move || {
            count.set(count.get() + 1);
            Some(Rc::new(value))
        });
    }
    let result = Container::<i32>::get_all();
    assert_eq!(invocation_count.get(), THREE_ITEMS);
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(*result[0], FIRST_VALUE);
    assert_eq!(*result[SECOND_INDEX], SECOND_VALUE);
    assert_eq!(*result[THIRD_INDEX], THIRD_VALUE);
}

#[test]
fn get_all_with_mixed_registrations_returns_all_values() {
    reset_containers();
    add_string("instance");
    Container::<String>::add_factory(|| Some(Rc::new("factory".to_owned())));
    add_string("another instance");
    let result = Container::<String>::get_all();
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(*result[0], "instance");
    assert_eq!(*result[SECOND_INDEX], "factory");
    assert_eq!(*result[THIRD_INDEX], "another instance");
}

#[test]
fn remove_current_when_empty_does_not_throw() {
    reset_containers();
    Container::<String>::remove_current();
    assert!(!Container::<String>::has_registrations());
}

#[test]
fn remove_current_with_single_item_clears_container() {
    reset_containers();
    add_string("test");
    Container::<String>::remove_current();
    assert!(!Container::<String>::has_registrations());
    assert!(Container::<String>::try_get().is_none());
}

#[test]
fn remove_current_with_multiple_items_removes_latest() {
    reset_containers();
    add_string(FIRST);
    add_string(SECOND);
    add_string(THIRD);
    Container::<String>::remove_current();
    assert_eq!(try_get_string().as_deref(), Some(SECOND));
}

#[test]
fn remove_current_multiple_removes_in_reverse_order() {
    reset_containers();
    Container::<i32>::add(Some(Rc::new(FIRST_VALUE)));
    Container::<i32>::add(Some(Rc::new(SECOND_VALUE)));
    Container::<i32>::add(Some(Rc::new(THIRD_VALUE)));
    Container::<i32>::remove_current();
    assert_eq!(Container::<i32>::try_get().map(|value| *value), Some(SECOND_VALUE));
    Container::<i32>::remove_current();
    assert_eq!(Container::<i32>::try_get().map(|value| *value), Some(FIRST_VALUE));
    Container::<i32>::remove_current();
    assert!(Container::<i32>::try_get().is_none());
}

#[test]
fn clear_removes_all_registrations() {
    reset_containers();
    add_string(FIRST);
    add_string(SECOND);
    add_string(THIRD);
    Container::<String>::clear();
    assert!(!Container::<String>::has_registrations());
    assert_eq!(Container::<String>::get_all().len(), 0);
}

#[test]
fn clear_when_empty_does_not_throw() {
    reset_containers();
    Container::<String>::clear();
    assert!(!Container::<String>::has_registrations());
}

#[test]
fn container_with_complex_type_works_correctly() {
    reset_containers();
    let service = Rc::new(TestService { id: 1, name: Some("Test".to_owned()) });
    Container::<TestService>::add(Some(service.clone()));
    let result = Container::<TestService>::try_get().expect("a service");
    assert!(Rc::ptr_eq(&result, &service));
    assert_eq!(result.id, 1);
    assert_eq!(result.name.as_deref(), Some("Test"));
}

#[test]
fn container_different_types_are_isolated() {
    reset_containers();
    add_string("string value");
    Container::<i32>::add(Some(Rc::new(SAMPLE_VALUE)));
    assert_eq!(try_get_string().as_deref(), Some("string value"));
    assert_eq!(Container::<i32>::try_get().map(|value| *value), Some(SAMPLE_VALUE));
}

#[test]
fn add_large_number_of_items_maintains_performance() {
    reset_containers();
    let item_count = 1000;
    for i in 0..item_count {
        Container::<i32>::add(Some(Rc::new(i)));
    }
    let result = Container::<i32>::get_all();
    assert_eq!(result.len(), item_count as usize);
    assert_eq!(*result[0], 0);
    assert_eq!(*result[result.len() - 1], item_count - 1);
}

// Not ported: Container_ThreadSafety_ConcurrentAdds - real threads; the containers are per thread in the port
