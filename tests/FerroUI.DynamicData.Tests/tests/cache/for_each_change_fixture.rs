//! `ForEachChangeFixture` (`DynamicData.Tests/Cache/ForEachChangeFixture.cs`): an action run for
//! each change of a cache.

use crate::domain::{Person, RandomPersonGenerator};
use ferroui_dynamic_data::cache::{Change, IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

struct Fixture {
    source: SourceCache<Rc<Person>, String>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceCache::new(|p: &Rc<Person>| p.name()) }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn test() {
    let fixture = Fixture::new();
    let messages: Rc<RefCell<Vec<Change<Rc<Person>, String>>>> = Rc::new(RefCell::new(Vec::new()));
    let recorder = messages.clone();
    let message_writer = fixture
        .source
        .connect()
        .for_each_change(move |change: &Change<Rc<Person>, String>| recorder.borrow_mut().push(change.clone()))
        .subscribe_ignore();

    let people = RandomPersonGenerator::new().take(100);
    fixture.source.edit(move |updater| updater.add_or_update_items(&mut people.into_iter()));
    message_writer.dispose();

    assert_eq!(messages.borrow().len(), 100);
}
