//! `RefCountFixture` (`DynamicData.Tests/List/RefCountFixture.cs`): a chain of operators of a list
//! shared by its subscribers, made when the first one subscribes and ended when the last one leaves.

use crate::domain::Person;
use ferroui_dynamic_data::list::{ChangeSet, IObservableList, ObservableListEx, SourceList};
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable};
use std::cell::Cell;
use std::rc::Rc;

struct Fixture {
    source: SourceList<Rc<Person>>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceList::new() }
    }

    /// `_source.Connect().Transform(p => p).Do(_ => created++).Finally(() => disposals++).RefCount()`.
    fn long_chain(&self, created: &Rc<Cell<i32>>, disposals: &Rc<Cell<i32>>) -> Rc<dyn IObservable<ChangeSet<Rc<Person>>>> {
        let created = created.clone();
        let disposals = disposals.clone();
        self.source
            .connect()
            .transform(|p: &Rc<Person>| p.clone())
            .do_(move |_: &ChangeSet<Rc<Person>>| created.set(created.get() + 1))
            .finally(move || disposals.set(disposals.get() + 1))
            .ref_count()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn can_resubscribe() {
    let fixture = Fixture::new();
    let created = Rc::new(Cell::new(0));
    let disposals = Rc::new(Cell::new(0));

    // must have data so transform is invoked
    fixture.source.edit(|list| list.add(Person::new("Name", 10)));

    // Some expensive transform (or chain of operations)
    let long_chain = fixture.long_chain(&created, &disposals);

    let subscriber = long_chain.subscribe_ignore();
    subscriber.dispose();

    let subscriber = long_chain.subscribe_ignore();
    subscriber.dispose();

    assert_eq!(created.get(), 2);
    assert_eq!(disposals.get(), 2);
}

#[test]
fn chain_is_invoked_once_for_multiple_subscribers() {
    let fixture = Fixture::new();
    let created = Rc::new(Cell::new(0));
    let disposals = Rc::new(Cell::new(0));

    // Some expensive transform (or chain of operations)
    let long_chain = fixture.long_chain(&created, &disposals);

    let suscriber1 = long_chain.subscribe_ignore();
    let suscriber2 = long_chain.subscribe_ignore();
    let suscriber3 = long_chain.subscribe_ignore();

    fixture.source.edit(|list| list.add(Person::new("Name", 10)));
    suscriber1.dispose();
    suscriber2.dispose();
    suscriber3.dispose();

    assert_eq!(created.get(), 1);
    assert_eq!(disposals.get(), 1);
}

// Not ported: IsHopefullyThreadSafe - real threads (upstream's [Fact] of it is commented out)
