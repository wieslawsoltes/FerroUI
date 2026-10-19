//! `OnItemFixture` (`DynamicData.Tests/Cache/OnItemFixture.cs`): the actions of the items added,
//! refreshed, removed and updated in a cache, and the same behaviour of a list and a cache.

use crate::domain::Person;
use ferroui_dynamic_data::binding::{ObservableCollectionEx, ObservableCollectionExtended};
use ferroui_dynamic_data::cache::{IConnectableCache, ObservableCacheEx, SourceCache};
use ferroui_reactive::ObservableExt;
use ferroui_system::collections::object_model::ObservableCollection;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn on_item_add_called() {
    let called = Rc::new(Cell::new(false));
    let source = SourceCache::new(|x: &Rc<Person>| x.age());

    let on_called = called.clone();
    let _subscription = source.connect().on_item_added(move |_: &Rc<Person>| on_called.set(true)).subscribe_ignore();

    let person = Person::new("A", 1);

    source.edit(move |updater| updater.add_or_update(person));
    assert!(called.get());
}

#[test]
fn on_item_refreshed_called() {
    let called = Rc::new(Cell::new(false));
    let source = SourceCache::new(|x: &Rc<Person>| x.age());

    let person = Person::new("A", 1);
    let added = person.clone();
    source.edit(move |updater| updater.add_or_update(added));

    let on_called = called.clone();
    let _subscription = source
        .connect()
        .auto_refresh(ferroui_system::prop!(Person => age))
        .on_item_refreshed(move |_: &Rc<Person>| on_called.set(true))
        .subscribe_ignore();

    person.set_age(person.age() + 1);

    assert!(called.get());
}

#[test]
fn on_item_removed_called() {
    let called = Rc::new(Cell::new(false));
    let source = SourceCache::new(|x: &Rc<Person>| x.age());

    let on_called = called.clone();
    let _subscription = source.connect().on_item_removed(move |_: &Rc<Person>| on_called.set(true)).subscribe_ignore();

    let person = Person::new("A", 1);
    let added = person.clone();
    source.edit(move |updater| updater.add_or_update(added));
    source.edit(|updater| updater.remove_item(&person));
    assert!(called.get());
}

// Test for https://github.com/reactivemarbles/DynamicData/issues/613
#[test]
fn on_item_removed_not_called_for_update() {
    let called = Rc::new(Cell::new(false));
    let source = SourceCache::new(|x: &Rc<Person>| x.age());

    let on_called = called.clone();
    let _subscription = source.connect().on_item_removed(move |_: &Rc<Person>| on_called.set(true)).subscribe_ignore();

    source.edit(|updater| updater.add_or_update(Person::new("A", 1)));
    source.edit(|updater| updater.add_or_update(Person::new("A", 2)));

    assert!(!called.get());
}

#[test]
fn on_item_updated_called() {
    let called = Rc::new(Cell::new(false));
    let source = SourceCache::new(|x: &Rc<Person>| x.age());

    let on_called = called.clone();
    let _subscription = source.connect().on_item_updated(move |_: &Rc<Person>, _: &Rc<Person>| on_called.set(true)).subscribe_ignore();

    let person = Person::new("A", 1);
    source.edit(move |updater| updater.add_or_update(person));
    let update = Person::new("B", 1);
    source.edit(move |updater| updater.add_or_update(update));
    assert!(called.get());
}

/// `Item`: a class without `Equals`, so equal only to itself.
struct Item {
    id: i32,
}

impl Item {
    fn new(id: i32) -> Rc<Self> {
        Rc::new(Self { id })
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `Proxy`: a class without `Equals`, so equal only to itself.
struct Proxy {
    item: Rc<Item>,
    active: Cell<Option<bool>>,
}

impl Proxy {
    fn new(item: &Rc<Item>) -> Rc<Self> {
        Rc::new(Self { item: item.clone(), active: Cell::new(None) })
    }
}

impl PartialEq for Proxy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// `ProxyEqualityComparer.Equals`.
fn proxy_equals(x: &Rc<Proxy>, y: &Rc<Proxy>) -> bool {
    x.item.id == y.item.id && x.active.get() == y.active.get()
}

/// `Assert.Equal(expected, actual, new ProxyEqualityComparer())`: the same length, and equal
/// proxies in the same order.
#[track_caller]
fn assert_proxies_equal(expected: &[Rc<Proxy>], actual: &[Rc<Proxy>]) {
    assert_eq!(expected.len(), actual.len(), "the collections should have the same number of proxies");
    for (index, (x, y)) in expected.iter().zip(actual.iter()).enumerate() {
        assert!(
            proxy_equals(x, y),
            "the proxies at {index} differ: item {} active {:?}, item {} active {:?}",
            x.item.id,
            x.active.get(),
            y.item.id,
            y.active.get()
        );
    }
}

/// The list half of `ListAndCacheShouldHaveEquivalentBehaviour`, in a module of its own so that
/// only the operators of lists are in scope.
mod list_side {
    use super::{Item, Proxy};
    use ferroui_dynamic_data::binding::{ObservableCollectionEx, ObservableCollectionExtended};
    use ferroui_dynamic_data::list::ObservableListEx;
    use ferroui_reactive::ObservableExt;
    use ferroui_system::collections::object_model::ObservableCollection;
    use ferroui_system::IDisposable;
    use std::rc::Rc;

    pub(super) fn bind(source: &ObservableCollection<Rc<Item>>, output: &ObservableCollectionExtended<Rc<Proxy>>) -> Rc<dyn IDisposable> {
        source
            .to_observable_change_set()
            .transform(|item: &Rc<Item>| Proxy::new(item))
            .on_item_added(|proxy: &Rc<Proxy>| proxy.active.set(Some(true)))
            .on_item_removed(|proxy: &Rc<Proxy>| proxy.active.set(Some(false)))
            .bind(output)
            .subscribe_ignore()
    }
}

// Test for https://github.com/reactivemarbles/DynamicData/issues/268
//
// Upstream binds to the read-only collections that `Bind(out ...)` makes; here each binds to an
// `ObservableCollectionExtended` of its own.
#[test]
fn list_and_cache_should_have_equivalent_behaviour() {
    let source = ObservableCollection::from_items(vec![Item::new(1), Item::new(2)]);

    let list_output = ObservableCollectionExtended::<Rc<Proxy>>::new();
    let list = list_side::bind(&source, &list_output);

    let cache_output = ObservableCollectionExtended::<Rc<Proxy>>::new();
    let cache = source
        .to_observable_change_set_with_key(|item: &Rc<Item>| item.id)
        .transform(|item: &Rc<Item>| Proxy::new(item))
        .on_item_added(|proxy: &Rc<Proxy>| proxy.active.set(Some(true)))
        .on_item_removed(|proxy: &Rc<Proxy>| proxy.active.set(Some(false)))
        .bind(&cache_output)
        .subscribe_ignore();

    assert_proxies_equal(&list_output.to_vec(), &cache_output.to_vec());

    list.dispose();
    cache.dispose();

    assert_proxies_equal(&list_output.to_vec(), &cache_output.to_vec());
}
