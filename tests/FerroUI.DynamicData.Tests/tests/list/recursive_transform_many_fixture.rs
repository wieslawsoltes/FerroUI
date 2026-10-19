//! `RecursiveTransformManyFixture` (`DynamicData.Tests/List/RecursiveTransformManyFixture.cs`): the
//! relations of people, recursively, as one list.

use crate::domain::PersonWithRelations;
use crate::utilities::SelectManyExtensions;
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{IObservableList, ObservableListEx, SourceList};
use ferroui_system::IDisposable;
use std::rc::Rc;

struct Fixture {
    results: ChangeSetAggregator<Rc<PersonWithRelations>>,
    source: SourceList<Rc<PersonWithRelations>>,
}

impl Fixture {
    fn new() -> Self {
        let source = SourceList::new();

        let results = source.connect().transform_many(|p: &Rc<PersonWithRelations>| p.relations().recursive_select(|r| r.relations())).as_aggregator();
        Self { results, source }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

#[test]
fn children_are_removed_when_parent_is_removed() {
    let fixture = Fixture::new();
    let frientofchild1 = PersonWithRelations::new("Friend1", 10);
    let child1 = PersonWithRelations::with_relations("Child1", 10, vec![frientofchild1]);
    let child2 = PersonWithRelations::new("Child2", 8);
    let child3 = PersonWithRelations::new("Child3", 8);
    let mother = PersonWithRelations::with_relations("Mother", 35, vec![child1, child2, child3]);
    //  var father = new PersonWithRelations("Father", 35, new[] {child1, child2, child3, mother});

    fixture.source.edit(|list| list.add(mother.clone()));
    fixture.source.edit(|list| {
        list.remove(&mother);
    });
    assert_eq!(fixture.results.data().count(), 0, "Should be 4 in the cache");
}

#[test]
fn recursive_children_can_be_added() {
    let fixture = Fixture::new();
    let frientofchild1 = PersonWithRelations::new("Friend1", 10);
    let child1 = PersonWithRelations::with_relations("Child1", 10, vec![frientofchild1.clone()]);
    let child2 = PersonWithRelations::new("Child2", 8);
    let child3 = PersonWithRelations::new("Child3", 8);
    let mother = PersonWithRelations::with_relations("Mother", 35, vec![child1.clone(), child2.clone(), child3.clone()]);
    //  var father = new PersonWithRelations("Father", 35, new[] {child1, child2, child3, mother});

    fixture.source.edit(|list| list.add(mother));

    let items = fixture.results.data().items();
    assert_eq!(fixture.results.data().count(), 4, "Should be 4 in the cache");
    assert!(items.contains(&child1));
    assert!(items.contains(&child2));
    assert!(items.contains(&child3));
    assert!(items.contains(&frientofchild1));
}
