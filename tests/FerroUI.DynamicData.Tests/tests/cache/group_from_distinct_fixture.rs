//! `GroupFromDistinctFixture` (`DynamicData.Tests/Cache/GroupFromDistinctFixture.cs`).

use crate::domain::{Person, PersonEmpKey, PersonEmployment, PersonWithEmployment, Randomizer};
use ferroui_dynamic_data::cache::{Group, IConnectableCache, IObservableCache, ObservableCacheEx, SourceCache};
use ferroui_system::IDisposable;
use std::rc::Rc;

#[test]
fn group_from_distinct() {
    let person_cache = SourceCache::new(|p: &Rc<Person>| p.key());
    let employment_cache = SourceCache::new(|e: &Rc<PersonEmployment>| e.key());

    const NUMBER_OF_PEOPLE: i32 = 1000;
    let random = Randomizer::new_unseeded();
    let companies = ["Company A", "Company B", "Company C"];

    // Create the people.
    let people: Vec<Rc<Person>> = (1..=NUMBER_OF_PEOPLE).map(|i| Person::new(format!("Person{i}"), i)).collect();

    // Create 0-3 jobs for each person, from the companies.
    let emphistory: Vec<Rc<PersonEmployment>> = (1..=NUMBER_OF_PEOPLE)
        .flat_map(|i| {
            let companiestogenrate = random.next(0, 4) as usize;
            (0..companiestogenrate).map(move |c| PersonEmployment::new(format!("Person{i}"), companies[c])).collect::<Vec<_>>()
        })
        .collect();

    // Cache the results.
    let allpeople_with_emp_history = employment_cache
        .connect()
        .group_with_result_groups(|e: &Rc<PersonEmployment>| e.name(), person_cache.connect().distinct_values(|p: &Rc<Person>| p.name()))
        .transform(|x: &Group<Rc<PersonEmployment>, PersonEmpKey, String>| PersonWithEmployment::new(x.key().clone(), x.cache()))
        .as_observable_cache();

    person_cache.edit(|updater| updater.add_or_update_items(&mut people.clone().into_iter()));
    employment_cache.edit(|updater| updater.add_or_update_items(&mut emphistory.clone().into_iter()));

    assert_eq!(allpeople_with_emp_history.count(), NUMBER_OF_PEOPLE as usize);
    let employments: usize = allpeople_with_emp_history.items().iter().map(|d| d.employment_data().items().len()).sum();
    assert_eq!(employments, emphistory.len());

    // The grouped items have the key of their parent.
    for p in allpeople_with_emp_history.items() {
        assert!(p.employment_data().items().iter().all(|emph| emph.name() == p.person()));
    }

    person_cache.edit(|updater| updater.remove_key(&"Person1".to_owned()));
    assert_eq!(allpeople_with_emp_history.count(), (NUMBER_OF_PEOPLE - 1) as usize);
    employment_cache.edit(|updater| updater.remove_items(&mut emphistory.clone().into_iter()));
    allpeople_with_emp_history.dispose();

    person_cache.dispose();
    employment_cache.dispose();
}
