//! `MergeManyItemsFixture` (`DynamicData.Tests/Cache/MergeManyItemsFixture.cs`): the values of the
//! sequences of the items of a cache, merged and paired with their items.

use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_dynamic_data::kernel::ItemWithValue;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// `ObjectWithObservable`: an item with a sequence of its own. Upstream's private `_value`, which
/// is written and never read, is left out.
struct ObjectWithObservable {
    id: i32,
    changed: Rc<Subject<bool>>,
}

impl ObjectWithObservable {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id, changed: Subject::new() })
    }

    fn id(&self) -> i32 {
        self.id
    }

    /// `Observable`.
    fn observable(&self) -> Rc<dyn IObservable<bool>> {
        self.changed.as_observable()
    }

    /// `InvokeObservable`.
    fn invoke_observable(&self, value: bool) {
        self.changed.on_next(value);
    }
}

/// The fixture: the source, disposed with the fixture (`Dispose`).
struct Fixture {
    source: SourceCache<Rc<ObjectWithObservable>, i32>,
}

impl Fixture {
    fn new() -> Self {
        Self { source: SourceCache::new(|p: &Rc<ObjectWithObservable>| p.id()) }
    }

    fn add_or_update(&self, item: &Rc<ObjectWithObservable>) {
        let item = item.clone();
        self.source.edit(move |updater| updater.add_or_update(item));
    }

    fn remove(&self, item: &Rc<ObjectWithObservable>) {
        self.source.edit(|updater| updater.remove_item(item));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.dispose();
    }
}

#[test]
fn everything_is_unsubscribed_when_stream_is_disposed() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let stream = fixture.source.connect().merge_many_items(|o: &Rc<ObjectWithObservable>| o.observable()).subscribe_next(move |o: ItemWithValue<Rc<ObjectWithObservable>, bool>| {
        flag.set(true);
        assert!(o.item().id() == 1);
    });

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    stream.dispose();

    item.invoke_observable(true);
    assert!(!invoked.get());
}

#[test]
fn invocation_only_when_child_is_invoked() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let stream = fixture.source.connect().merge_many_items(|o: &Rc<ObjectWithObservable>| o.observable()).subscribe_next(move |o: ItemWithValue<Rc<ObjectWithObservable>, bool>| {
        flag.set(true);
        assert!(o.item().id() == 1);
    });

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);

    assert!(!invoked.get());

    item.invoke_observable(true);
    assert!(invoked.get());
    stream.dispose();
}

#[test]
fn removed_item_will_not_cause_invocation() {
    let fixture = Fixture::new();
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let stream = fixture.source.connect().merge_many_items(|o: &Rc<ObjectWithObservable>| o.observable()).subscribe_next(move |o: ItemWithValue<Rc<ObjectWithObservable>, bool>| {
        flag.set(true);
        assert!(o.item().id() == 1);
    });

    let item = ObjectWithObservable::new(1);
    fixture.add_or_update(&item);
    fixture.remove(&item);
    assert!(!invoked.get());

    item.invoke_observable(true);
    assert!(!invoked.get());
    stream.dispose();
}
