//! `TransformManyFixture` (`DynamicData.Tests/List/TransformManyFixture.cs`): the relations of
//! people, recursively, as one list.

use crate::domain::PersonWithRelations;
use crate::utilities::{assert_equivalent, SelectManyExtensions};
use ferroui_dynamic_data::list::testing::ChangeSetAggregator;
use ferroui_dynamic_data::list::{replace, IObservableList, ObservableListEx, SourceList};
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

    fn add(&self, person: &Rc<PersonWithRelations>) {
        self.source.edit(|list| list.add(person.clone()));
    }

    fn add_range(&self, people: &[Rc<PersonWithRelations>]) {
        self.source.edit(|list| list.add_range(people.to_vec()));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
        self.results.dispose();
    }
}

/// The family of most tests: a mother, her three children and the friend of the first.
fn mother_and_children() -> (Rc<PersonWithRelations>, [Rc<PersonWithRelations>; 4]) {
    let frientofchild1 = PersonWithRelations::new("Friend1", 10);
    let child1 = PersonWithRelations::with_relations("Child1", 10, vec![frientofchild1.clone()]);
    let child2 = PersonWithRelations::new("Child2", 8);
    let child3 = PersonWithRelations::new("Child3", 8);
    let mother = PersonWithRelations::with_relations("Mother", 35, vec![child1.clone(), child2.clone(), child3.clone()]);
    (mother, [child1, child2, child3, frientofchild1])
}

#[test]
fn add() {
    let fixture = Fixture::new();
    let (mother, [child1, child2, child3, frientofchild1]) = mother_and_children();
    //  var father = new PersonWithRelations("Father", 35, new[] {child1, child2, child3, mother});

    fixture.add(&mother);

    assert_eq!(fixture.results.data().count(), 4);
    assert_equivalent(&fixture.results.data().items(), &[child1, child2, child3, frientofchild1]);
}

#[test]
fn add_range() {
    let fixture = Fixture::new();
    let (mother, [child1, child2, child3, frientofchild1]) = mother_and_children();

    fixture.add(&mother);

    let child4 = PersonWithRelations::new("Child4", 1);
    let child5 = PersonWithRelations::new("Child5", 2);
    let another_relative1 = PersonWithRelations::with_relations("Another1", 2, vec![child4.clone(), child5.clone()]);

    let child6 = PersonWithRelations::new("Child6", 1);
    let child7 = PersonWithRelations::new("Child7", 2);
    let another_relative2 = PersonWithRelations::with_relations("Another2", 2, vec![child6.clone(), child7.clone()]);

    fixture.add_range(&[another_relative1, another_relative2]);
    assert_eq!(fixture.results.data().count(), 8);
    assert_equivalent(&fixture.results.data().items(), &[child1, child2, child3, frientofchild1, child4, child5, child6, child7]);
}

#[test]
fn clear() {
    let fixture = Fixture::new();
    let (mother, _) = mother_and_children();
    let child4 = PersonWithRelations::new("Child4", 1);
    let child5 = PersonWithRelations::new("Child5", 2);
    let another_relative1 = PersonWithRelations::with_relations("Another1", 2, vec![child4, child5]);
    let child6 = PersonWithRelations::new("Child6", 1);
    let child7 = PersonWithRelations::new("Child7", 2);
    let another_relative2 = PersonWithRelations::with_relations("Another2", 2, vec![child6, child7]);

    fixture.add_range(&[mother, another_relative1, another_relative2]);

    fixture.source.edit(|list| list.clear());
    assert_eq!(fixture.results.data().count(), 0);
}

#[test]
fn move_() {
    let fixture = Fixture::new();
    // Move should have no effect.

    let child4 = PersonWithRelations::new("Child4", 1);
    let child5 = PersonWithRelations::new("Child5", 2);
    let another_relative1 = PersonWithRelations::with_relations("Another1", 2, vec![child4, child5]);
    let child6 = PersonWithRelations::new("Child6", 1);
    let child7 = PersonWithRelations::new("Child7", 2);
    let another_relative2 = PersonWithRelations::with_relations("Another2", 2, vec![child6, child7]);

    fixture.add_range(&[another_relative1, another_relative2]);

    assert_eq!(fixture.results.messages().len(), 1);
    fixture.source.edit(|list| list.move_index(1, 0));
    assert_eq!(fixture.results.messages().len(), 1);
}

// Not ported: Remove - needs ObservableCollectionEx.ToObservableChangeSet (wave 2): the tours of a
// provider are an `ObservableCollection<Tour>`.

#[test]
fn remove_parent() {
    let fixture = Fixture::new();
    let (mother, _) = mother_and_children();

    fixture.add(&mother);
    fixture.source.edit(|list| {
        list.remove(&mother);
    });
    assert_eq!(fixture.results.data().count(), 0);
}

#[test]
fn remove_range() {
    let fixture = Fixture::new();
    let (mother, _) = mother_and_children();
    let child4 = PersonWithRelations::new("Child4", 1);
    let child5 = PersonWithRelations::new("Child5", 2);
    let another_relative1 = PersonWithRelations::with_relations("Another1", 2, vec![child4, child5]);
    let child6 = PersonWithRelations::new("Child6", 1);
    let child7 = PersonWithRelations::new("Child7", 2);
    let another_relative2 = PersonWithRelations::with_relations("Another2", 2, vec![child6.clone(), child7.clone()]);

    fixture.add_range(&[mother, another_relative1, another_relative2]);

    fixture.source.edit(|list| list.remove_range(0, 2));
    assert_eq!(fixture.results.data().count(), 2);
    assert_equivalent(&fixture.results.data().items(), &[child6, child7]);
}

#[test]
fn replace_() {
    let fixture = Fixture::new();
    let (mother, [child1, child2, _child3, frientofchild1]) = mother_and_children();

    fixture.add(&mother);

    let child4 = PersonWithRelations::new("Child4", 2);
    let updated_mother = PersonWithRelations::with_relations("Mother", 35, vec![child1.clone(), child2.clone(), child4.clone()]);

    fixture.source.edit(|list| replace(list, &mother, updated_mother.clone()));

    assert_eq!(fixture.results.data().count(), 4);
    assert_equivalent(&fixture.results.data().items(), &[child1, child2, frientofchild1, child4]);
}
