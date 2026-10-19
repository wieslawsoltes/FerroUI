//! `RefCountFixture` (`DynamicData.Tests/Cache/RefCountFixture.cs`): a chain of operators of a cache
//! shared by its subscribers, made when the first one subscribes and ended when the last one leaves.

use crate::domain::Person;
use ferroui_dynamic_data::cache::{ChangeSet, IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

struct Fixture {
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceCache::new(|p: &Rc<Person>| p.key()) }
    }

    /// `_source.Connect().Transform(p => p).Do(_ => created++).Finally(() => disposals++).RefCount()`.
    fn long_chain(&self, created: &Rc<Cell<i32>>, disposals: &Rc<Cell<i32>>) -> Rc<dyn ferroui_system::IObservable<ChangeSet<Rc<Person>, String>>> {
        let created = created.clone();
        let disposals = disposals.clone();
        self.source
            .connect()
            .transform(|p: &Rc<Person>| p.clone())
            .do_(move |_: &ChangeSet<Rc<Person>, String>| created.set(created.get() + 1))
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
    fixture.source.edit(|updater| updater.add_or_update(Person::new("Name", 10)));

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

    let subscriber1 = long_chain.subscribe_ignore();
    let subscriber2 = long_chain.subscribe_ignore();
    let subscriber3 = long_chain.subscribe_ignore();

    fixture.source.edit(|updater| updater.add_or_update(Person::new("Name", 10)));
    subscriber1.dispose();
    subscriber2.dispose();
    subscriber3.dispose();

    assert_eq!(created.get(), 1);
    assert_eq!(disposals.get(), 1);
}

// Not ported: IsHopefullyThreadSafe - real threads (upstream's [Fact] of it is commented out)
