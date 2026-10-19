//! `AutoPersistHelperTest` (`ReactiveUI.Tests/AutoPersist/AutoPersistHelperTest.cs`).

use super::{counting_save, ms, with_virtual_time};
use crate::mocks::TestFixture;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactiveui::*;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::IObserver;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::time::Duration;

const INITIAL_ADVANCE_MILLISECONDS: u64 = 10;
const THROTTLE_MILLISECONDS: u64 = 100;
const PAST_THROTTLE_MILLISECONDS: u64 = 150;

/// A reactive object without a data contract (`ObjectWithoutDataContract`).
#[derive(Default)]
struct ObjectWithoutDataContract {
    reactive: ReactiveObject,
}
reactive_object!(ObjectWithoutDataContract);

fn collected() -> (Rc<RefCell<Vec<Rc<TestFixture>>>>, impl Fn(Rc<TestFixture>) + 'static) {
    let items = Rc::new(RefCell::new(Vec::new()));
    let log = items.clone();
    (items, move |item| log.borrow_mut().push(item))
}

fn contains(items: &[Rc<TestFixture>], item: &Rc<TestFixture>) -> bool {
    items.iter().any(|candidate| Rc::ptr_eq(candidate, item))
}

#[test]
fn act_on_every_object_add_new_item_calls_on_add() {
    let collection = ObservableCollection::<Rc<TestFixture>>::new();
    let (added_items, on_add) = collected();
    let _ = collection.act_on_every_object(on_add, |_| {});

    let item = TestFixture::new();
    collection.add(item.clone());

    let added_items = added_items.borrow();
    assert_eq!(added_items.len(), 1);
    assert!(Rc::ptr_eq(&added_items[0], &item));
}

#[test]
fn act_on_every_object_clear_collection_calls_on_remove() {
    let item1 = TestFixture::new();
    let item2 = TestFixture::new();
    let collection = ObservableCollection::from_items(vec![item1.clone(), item2.clone()]);
    let (removed_items, on_remove) = collected();
    let _ = collection.act_on_every_object(|_| {}, on_remove);

    collection.clear();

    let removed_items = removed_items.borrow();
    assert_eq!(removed_items.len(), 2);
    assert!(contains(&removed_items, &item1));
    assert!(contains(&removed_items, &item2));
}

#[test]
fn act_on_every_object_dispose_calls_on_remove_for_all() {
    let item1 = TestFixture::new();
    let item2 = TestFixture::new();
    let collection = ObservableCollection::from_items(vec![item1.clone(), item2.clone()]);
    let (removed_items, on_remove) = collected();
    let subscription = collection.act_on_every_object(|_| {}, on_remove);

    subscription.dispose();

    let removed_items = removed_items.borrow();
    assert_eq!(removed_items.len(), 2);
    assert!(contains(&removed_items, &item1));
    assert!(contains(&removed_items, &item2));
}

#[test]
fn act_on_every_object_existing_items_calls_on_add() {
    let item1 = TestFixture::new();
    let item2 = TestFixture::new();
    let collection = ObservableCollection::from_items(vec![item1.clone(), item2.clone()]);
    let (added_items, on_add) = collected();

    let _ = collection.act_on_every_object(on_add, |_| {});

    let added_items = added_items.borrow();
    assert_eq!(added_items.len(), 2);
    assert!(contains(&added_items, &item1));
    assert!(contains(&added_items, &item2));
}

// Not ported: ActOnEveryObject_NullCollection_ThrowsArgumentNullException - argument validation
// Not ported: ActOnEveryObject_NullOnAdd_ThrowsArgumentNullException - argument validation
// Not ported: ActOnEveryObject_NullOnRemove_ThrowsArgumentNullException - argument validation
// Not ported: ActOnEveryObject_ReadOnlyCollection_WorksCorrectly - needs ReadOnlyObservableCollection in ferroui-system

#[test]
fn act_on_every_object_remove_item_calls_on_remove() {
    let item = TestFixture::new();
    let collection = ObservableCollection::from_items(vec![item.clone()]);
    let (removed_items, on_remove) = collected();
    let _ = collection.act_on_every_object(|_| {}, on_remove);

    let _ = collection.remove(&item);

    let removed_items = removed_items.borrow();
    assert_eq!(removed_items.len(), 1);
    assert!(Rc::ptr_eq(&removed_items[0], &item));
}

#[test]
fn act_on_every_object_replace_item_calls_on_remove_and_on_add() {
    let item1 = TestFixture::new();
    let item2 = TestFixture::new();
    let collection = ObservableCollection::from_items(vec![item1.clone()]);
    let (added_items, on_add) = collected();
    let (removed_items, on_remove) = collected();
    let _ = collection.act_on_every_object(on_add, on_remove);

    collection.set(0, item2.clone());

    let added_items = added_items.borrow();
    let removed_items = removed_items.borrow();
    assert_eq!(added_items.len(), 2);
    assert!(Rc::ptr_eq(&added_items[0], &item1));
    assert!(Rc::ptr_eq(&added_items[1], &item2));
    assert_eq!(removed_items.len(), 1);
    assert!(Rc::ptr_eq(&removed_items[0], &item1));
}

#[test]
fn auto_persist_dispose_stops_saving() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let subscription = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(1)));
        const AFTER_DISPOSE_SECONDS: u64 = 2;

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some("First".to_owned()));
        scheduler.advance_by(ms(1000));
        assert_eq!(save_count.get(), 1);

        subscription.dispose();
        fixture.set_is_not_null_string(Some("Second".to_owned()));
        scheduler.advance_by(super::seconds(AFTER_DISPOSE_SECONDS));
        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_manual_save_signal_triggers_save() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let manual_save = Subject::<()>::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_with(counting_save(&save_count), manual_save.as_observable(), Some(Duration::from_secs(1)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        manual_save.on_next(());
        scheduler.advance_by(ms(1000));

        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_metadata_provider_works_correctly() {
    with_virtual_time(|_| {
        let metadata_provider = AutoPersistMetadata::provider_for_type::<TestFixture>();
        let fixture = TestFixture::new();

        let metadata = metadata_provider(&fixture);

        assert!(metadata.has_data_contract());
        assert!(metadata.persistable_property_names().contains("IsNotNullString"));
    });
}

/// Upstream reads by reflection that the object has no `[DataContract]`; a type without the
/// contract does not implement [`DataContract`], so the metadata says it explicitly.
#[test]
fn auto_persist_no_data_contract_throws_argument_exception() {
    let obj = Rc::new(ObjectWithoutDataContract::default());
    let metadata = Rc::new(AutoPersistMetadata::new(false, Vec::<String>::new()));

    let outcome = catch_unwind(AssertUnwindSafe(|| obj.auto_persist_metadata(|_| Observable::return_(()), metadata)));

    assert!(outcome.is_err());
}

#[test]
fn auto_persist_non_data_member_property_does_not_trigger_save() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(1)));
        const SMALL_ADVANCE_MILLISECONDS: u64 = 2;

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_poco_property(Some("NoSave".to_owned()));
        scheduler.advance_by(ms(SMALL_ADVANCE_MILLISECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

// Not ported: AutoPersist_NullDoPersist_ThrowsArgumentNullException - argument validation
// Not ported: AutoPersist_NullManualSaveSignal_ThrowsArgumentNullException - argument validation
// Not ported: AutoPersist_NullMetadata_ThrowsArgumentNullException - argument validation

#[test]
fn auto_persist_property_change_triggers_save() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(1)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(1000));

        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_throttle_respect_interval() {
    with_virtual_time(|scheduler| {
        const THROTTLE_SECONDS: u64 = 2;
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(THROTTLE_SECONDS)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some("First".to_owned()));
        scheduler.advance_by(ms(1000));
        assert_eq!(save_count.get(), 0);

        fixture.set_is_not_null_string(Some("Second".to_owned()));
        scheduler.advance_by(ms(1000));
        assert_eq!(save_count.get(), 0);

        scheduler.advance_by(ms(1000));
        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_with_metadata_saves_correctly() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let metadata = AutoPersistMetadata::for_type::<TestFixture>();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_metadata_with_interval(counting_save(&save_count), metadata, Some(Duration::from_secs(1)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(1000));

        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_collection_add_item_enables_persistence() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = collection.auto_persist_collection_with_interval(counting_save(&save_count), Some(Duration::from_millis(THROTTLE_MILLISECONDS)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let item = TestFixture::new();
        collection.add(item.clone());
        scheduler.advance_by(ms(1));
        item.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(save_count.get(), 1);
    });
}
