//! `GroupControllerForFilteredItemsFixture`
//! (`DynamicData.Tests/Cache/GroupControllerForFilteredItemsFixture.cs`).

use crate::domain::Person;
use ferroui_dynamic_data::cache::{Group, IConnectableCache, IObservableCache, ObservableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::subjects::Subject;
use ferroui_system::{IDisposable, IObserver};
use std::rc::Rc;

/// `AgeBracket`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AgeBracket {
    Under20,
    Adult,
    Pensioner,
}

/// `_grouper`.
fn grouper(p: &Rc<Person>) -> AgeBracket {
    if p.age() <= 19 {
        return AgeBracket::Under20;
    }
    if p.age() <= 60 {
        AgeBracket::Adult
    } else {
        AgeBracket::Pensioner
    }
}

struct Fixture {
    grouped: ObservableCache<Group<Rc<Person>, String, AgeBracket>, AgeBracket>,
    refresh_subject: Rc<Subject<()>>,
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceCache::new(|p: &Rc<Person>| p.name());
        let refresh_subject = Subject::new();
        let grouped = source.connect_filtered(|p: &Rc<Person>| grouper(p) != AgeBracket::Pensioner).group_with(grouper, refresh_subject.as_observable()).as_observable_cache();
        Self { grouped, refresh_subject, source }
    }

    fn is_contained_in(&self, name: &str, bracket: AgeBracket) -> bool {
        match self.grouped.lookup(&bracket) {
            None => false,
            Some(group) => group.cache().lookup(&name.to_owned()).is_some(),
        }
    }

    fn people_named(&self, name: &str) -> Vec<Rc<Person>> {
        self.grouped.items().iter().flat_map(|g| g.cache().items().into_iter().filter(|s| s.name() == name)).collect()
    }

    fn is_contained_only_in_one_group(&self, name: &str) -> bool {
        self.people_named(name).len() == 1
    }

    fn is_not_contained_any_where(&self, name: &str) -> bool {
        self.people_named(name).is_empty()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.grouped.dispose();
    }
}

#[test]
fn regroup_recaluates_groupings() {
    let fixture = Fixture::new();
    let p1 = Person::new("P1", 10);
    let p2 = Person::new("P2", 15);
    let p3 = Person::new("P3", 30);
    let p4 = Person::new("P4", 70);
    let people = vec![p1.clone(), p2.clone(), p3.clone(), p4.clone()];

    fixture.source.edit(|updater| updater.add_or_update_items(&mut people.into_iter()));

    assert!(fixture.is_contained_in("P1", AgeBracket::Under20));
    assert!(fixture.is_contained_in("P2", AgeBracket::Under20));
    assert!(fixture.is_contained_in("P3", AgeBracket::Adult));

    p1.set_age(60);
    p2.set_age(80);
    p3.set_age(15);
    p4.set_age(30);

    fixture.refresh_subject.on_next(());

    assert!(fixture.is_contained_in("P1", AgeBracket::Adult));
    assert!(fixture.is_contained_in("P3", AgeBracket::Under20));

    assert!(fixture.is_contained_only_in_one_group("P1"));
    assert!(fixture.is_contained_only_in_one_group("P2"));
}

#[test]
fn regroup_recaluates_groupings2() {
    let fixture = Fixture::new();
    let p1 = Person::new("P1", 10);
    let p2 = Person::new("P2", 15);
    let p3 = Person::new("P3", 30);
    let p4 = Person::new("P4", 70);
    let people = vec![p1.clone(), p2.clone(), p3.clone(), p4.clone()];

    fixture.source.edit(|updater| updater.add_or_update_items(&mut people.into_iter()));

    assert!(fixture.is_contained_in("P1", AgeBracket::Under20));
    assert!(fixture.is_contained_in("P2", AgeBracket::Under20));
    assert!(fixture.is_contained_in("P3", AgeBracket::Adult));
    assert!(!fixture.is_contained_in("P4", AgeBracket::Pensioner));

    p1.set_age(60);
    p2.set_age(80);
    p3.set_age(15);
    p4.set_age(30);

    let refreshed = vec![p1, p2, p3, p4];
    fixture.source.edit(|updater| updater.refresh_items(&mut refreshed.into_iter()));

    assert!(fixture.is_contained_in("P1", AgeBracket::Adult));
    assert!(!fixture.is_contained_in("P2", AgeBracket::Pensioner));
    assert!(fixture.is_contained_in("P3", AgeBracket::Under20));
    assert!(fixture.is_contained_in("P4", AgeBracket::Adult));

    assert!(fixture.is_contained_only_in_one_group("P1"));
    assert!(fixture.is_not_contained_any_where("P2"));
    assert!(fixture.is_contained_only_in_one_group("P3"));
    assert!(fixture.is_contained_only_in_one_group("P4"));
}
