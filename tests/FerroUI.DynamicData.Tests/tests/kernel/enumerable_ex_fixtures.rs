//! `EnumerableExFixtures` (`DynamicData.Tests/EnumerableExFixtures.cs`): a collection as a
//! sequence of change sets (`AsObservableChangeSet`).

use crate::domain::Person;
use crate::utilities::assert_equivalent;
use ferroui_dynamic_data::cache::{IObservableCache, ObservableCacheEx};
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx};
use ferroui_dynamic_data::EnumerableEx;
use ferroui_reactive::ObservableExt;
use std::cell::Cell;
use std::rc::Rc;

/// Upstream's `_person1`, `_person2` and `_person3`.
fn source() -> Vec<Rc<Person>> {
    vec![Person::new("One", 1), Person::new("Two", 2), Person::new("Three", 3)]
}

#[test]
fn can_convert_to_observable_change_set_cache() {
    let source = source();
    let change_set = source.clone().as_observable_change_set().as_observable_list();
    assert_equivalent(&change_set.items(), &source);
}

#[test]
fn can_convert_to_observable_change_set_list() {
    let source = source();
    let change_set = source.clone().as_observable_change_set_cache(|x: &Rc<Person>| x.age()).as_observable_cache();
    assert_equivalent(&change_set.items(), &source);
}

fn respects_complete_configuration_for_cache(should_complete: bool) {
    let completed = Rc::new(Cell::new(false));
    let source = source();
    let target = completed.clone();
    let subscription = source.as_observable_change_set_cache_with(|x: &Rc<Person>| x.age(), should_complete).subscribe_next_completed(|_| {}, move || target.set(true));
    assert_eq!(completed.get(), should_complete);
    subscription.dispose();
}

#[test]
fn respects_complete_configuration_for_cache_case1() {
    respects_complete_configuration_for_cache(true);
}

#[test]
fn respects_complete_configuration_for_cache_case2() {
    respects_complete_configuration_for_cache(false);
}

fn respects_complete_configuration_for_list(should_complete: bool) {
    let completed = Rc::new(Cell::new(false));
    let source = source();
    let target = completed.clone();
    let subscription = source.as_observable_change_set_with(should_complete).subscribe_next_completed(|_| {}, move || target.set(true));
    assert_eq!(completed.get(), should_complete);
    subscription.dispose();
}

#[test]
fn respects_complete_configuration_for_list_case1() {
    respects_complete_configuration_for_list(true);
}

#[test]
fn respects_complete_configuration_for_list_case2() {
    respects_complete_configuration_for_list(false);
}
