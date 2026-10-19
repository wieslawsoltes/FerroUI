//! Ported from `Splat.Tests/ServiceLocation/GlobalGenericFirst/ContractContainerTests.cs` of Splat.
//! Adapted: `ContractContainer<T>` is the typed view `ContractContainer::<S>` over the contract
//! containers of the thread, holding `Rc<S>` (`ContractContainer<int?>` is
//! `ContractContainer::<i32>`); contracts are `Option<&str>`; `TryGet(contract, out T)` returns an
//! `Option`. The containers are per thread and each test runs on a thread of its own, so the
//! `[Before(Test)]`/`[After(Test)]` reset is the `clear_all` at the start.

use crate::service_location::global_generic_first::ContractContainer;
use std::cell::Cell;
use std::rc::Rc;

const CONTRACT1: &str = "Contract1";
const CONTRACT2: &str = "Contract2";
const CONTRACT1_VALUE: &str = "contract1";
const CONTRACT2_VALUE: &str = "contract2";
const FIRST: &str = "first";
const SECOND: &str = "second";
const THIRD: &str = "third";
const TWO_ITEMS: usize = 2;
const THREE_ITEMS: usize = 3;
const SAMPLE_VALUE: i32 = 42;
const FIRST_VALUE: i32 = 1;
const SECOND_VALUE: i32 = 2;
const SECOND_INDEX: usize = 1;
const THIRD_INDEX: usize = 2;

fn reset_containers() {
    ContractContainer::<String>::clear_all();
    ContractContainer::<i32>::clear_all();
}

fn add_string(value: &str, contract: Option<&str>) {
    ContractContainer::<String>::add(Some(Rc::new(value.to_owned())), contract);
}

fn try_get_string(contract: &str) -> Option<String> {
    ContractContainer::<String>::try_get(Some(contract)).map(|value| (*value).clone())
}

#[test]
fn has_registrations_when_empty_returns_false() {
    reset_containers();
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
}

#[test]
fn has_registrations_after_adding_returns_true() {
    reset_containers();
    add_string("test", Some(CONTRACT1));
    assert!(ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
}

#[test]
fn add_with_instance_stores_instance() {
    reset_containers();
    add_string("test value", Some(CONTRACT1));
    assert_eq!(try_get_string(CONTRACT1).as_deref(), Some("test value"));
}

#[test]
fn add_with_factory_stores_factory() {
    reset_containers();
    ContractContainer::<i32>::add_factory(|| Some(Rc::new(42)), Some(CONTRACT1));
    assert_eq!(ContractContainer::<i32>::try_get(Some(CONTRACT1)).map(|value| *value), Some(42));
}

#[test]
fn add_with_null_contract_uses_empty_string() {
    reset_containers();
    add_string("test", None);
    assert_eq!(try_get_string("").as_deref(), Some("test"));
}

#[test]
fn add_multiple_to_same_contract_returns_latest() {
    reset_containers();
    add_string(FIRST, Some(CONTRACT1));
    add_string(SECOND, Some(CONTRACT1));
    add_string(THIRD, Some(CONTRACT1));
    assert_eq!(try_get_string(CONTRACT1).as_deref(), Some(THIRD));
}

#[test]
fn add_to_different_contracts_are_isolated() {
    reset_containers();
    add_string("contract1 value", Some(CONTRACT1));
    add_string("contract2 value", Some(CONTRACT2));
    assert_eq!(try_get_string(CONTRACT1).as_deref(), Some("contract1 value"));
    assert_eq!(try_get_string(CONTRACT2).as_deref(), Some("contract2 value"));
}

#[test]
fn try_get_when_empty_returns_false() {
    reset_containers();
    assert!(ContractContainer::<String>::try_get(Some(CONTRACT1)).is_none());
}

#[test]
fn try_get_with_wrong_contract_returns_false() {
    reset_containers();
    add_string("test", Some(CONTRACT1));
    assert!(ContractContainer::<String>::try_get(Some(CONTRACT2)).is_none());
}

#[test]
fn try_get_with_factory_returning_null_returns_false() {
    reset_containers();
    ContractContainer::<String>::add_factory(|| None, Some(CONTRACT1));
    assert!(ContractContainer::<String>::try_get(Some(CONTRACT1)).is_none());
}

#[test]
fn try_get_invokes_factory_each_time() {
    reset_containers();
    let invocation_count = Rc::new(Cell::new(0));
    let count = invocation_count.clone();
    ContractContainer::<i32>::add_factory(
        move || {
            count.set(count.get() + 1);
            Some(Rc::new(count.get()))
        },
        Some(CONTRACT1),
    );
    let result1 = ContractContainer::<i32>::try_get(Some(CONTRACT1)).map(|value| *value);
    let result2 = ContractContainer::<i32>::try_get(Some(CONTRACT1)).map(|value| *value);
    assert_eq!(invocation_count.get(), 2);
    assert_eq!(result1, Some(FIRST_VALUE));
    assert_eq!(result2, Some(SECOND_VALUE));
}

#[test]
fn get_all_when_empty_returns_empty_array() {
    reset_containers();
    assert_eq!(ContractContainer::<String>::get_all(Some(CONTRACT1)).len(), 0);
}

#[test]
fn get_all_with_multiple_registrations_returns_all_values() {
    reset_containers();
    add_string(FIRST, Some(CONTRACT1));
    add_string(SECOND, Some(CONTRACT1));
    add_string(THIRD, Some(CONTRACT1));
    let result = ContractContainer::<String>::get_all(Some(CONTRACT1));
    assert_eq!(result.len(), THREE_ITEMS);
    assert_eq!(*result[0], FIRST);
    assert_eq!(*result[SECOND_INDEX], SECOND);
    assert_eq!(*result[THIRD_INDEX], THIRD);
}

#[test]
fn get_all_only_returns_matching_contract() {
    reset_containers();
    add_string("contract1-1", Some(CONTRACT1));
    add_string("contract1-2", Some(CONTRACT1));
    add_string("contract2-1", Some(CONTRACT2));
    let result1 = ContractContainer::<String>::get_all(Some(CONTRACT1));
    let result2 = ContractContainer::<String>::get_all(Some(CONTRACT2));
    assert_eq!(result1.len(), TWO_ITEMS);
    assert_eq!(*result1[0], "contract1-1");
    assert_eq!(*result1[1], "contract1-2");
    assert_eq!(result2.len(), 1);
    assert_eq!(*result2[0], "contract2-1");
}

#[test]
fn remove_current_when_empty_does_not_throw() {
    reset_containers();
    ContractContainer::<String>::remove_current(Some(CONTRACT1));
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
}

#[test]
fn remove_current_with_single_item_clears_contract() {
    reset_containers();
    add_string("test", Some(CONTRACT1));
    ContractContainer::<String>::remove_current(Some(CONTRACT1));
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
    assert!(ContractContainer::<String>::try_get(Some(CONTRACT1)).is_none());
}

#[test]
fn remove_current_with_multiple_items_removes_latest() {
    reset_containers();
    add_string(FIRST, Some(CONTRACT1));
    add_string(SECOND, Some(CONTRACT1));
    add_string(THIRD, Some(CONTRACT1));
    ContractContainer::<String>::remove_current(Some(CONTRACT1));
    assert_eq!(try_get_string(CONTRACT1).as_deref(), Some(SECOND));
}

#[test]
fn remove_current_only_affects_specified_contract() {
    reset_containers();
    add_string(CONTRACT1_VALUE, Some(CONTRACT1));
    add_string(CONTRACT2_VALUE, Some(CONTRACT2));
    ContractContainer::<String>::remove_current(Some(CONTRACT1));
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
    assert!(ContractContainer::<String>::has_registrations(Some(CONTRACT2)));
}

#[test]
fn clear_removes_all_registrations_for_contract() {
    reset_containers();
    add_string(FIRST, Some(CONTRACT1));
    add_string(SECOND, Some(CONTRACT1));
    add_string(THIRD, Some(CONTRACT1));
    ContractContainer::<String>::clear(Some(CONTRACT1));
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
    assert_eq!(ContractContainer::<String>::get_all(Some(CONTRACT1)).len(), 0);
}

#[test]
fn clear_only_affects_specified_contract() {
    reset_containers();
    add_string(CONTRACT1_VALUE, Some(CONTRACT1));
    add_string(CONTRACT2_VALUE, Some(CONTRACT2));
    ContractContainer::<String>::clear(Some(CONTRACT1));
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
    assert!(ContractContainer::<String>::has_registrations(Some(CONTRACT2)));
}

#[test]
fn clear_all_removes_all_contracts() {
    reset_containers();
    add_string(CONTRACT1_VALUE, Some(CONTRACT1));
    add_string(CONTRACT2_VALUE, Some(CONTRACT2));
    ContractContainer::<String>::clear_all();
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT2)));
}

#[test]
fn clear_all_when_empty_does_not_throw() {
    reset_containers();
    ContractContainer::<String>::clear_all();
    assert!(!ContractContainer::<String>::has_registrations(Some(CONTRACT1)));
}

#[test]
fn contract_container_different_types_are_isolated() {
    reset_containers();
    add_string("string value", Some(CONTRACT1));
    ContractContainer::<i32>::add(Some(Rc::new(SAMPLE_VALUE)), Some(CONTRACT1));
    assert_eq!(try_get_string(CONTRACT1).as_deref(), Some("string value"));
    assert_eq!(ContractContainer::<i32>::try_get(Some(CONTRACT1)).map(|value| *value), Some(SAMPLE_VALUE));
}

#[test]
fn add_with_many_contracts_works_correctly() {
    reset_containers();
    let contract_count = 50;
    for i in 0..contract_count {
        ContractContainer::<i32>::add(Some(Rc::new(i)), Some(&format!("Contract{i}")));
    }
    for i in 0..contract_count {
        let contract = format!("Contract{i}");
        assert_eq!(ContractContainer::<i32>::try_get(Some(&contract)).map(|value| *value), Some(i));
    }
}

// Not ported: ContractContainer_ThreadSafety_ConcurrentAdds - real threads; the containers are per thread in the port
