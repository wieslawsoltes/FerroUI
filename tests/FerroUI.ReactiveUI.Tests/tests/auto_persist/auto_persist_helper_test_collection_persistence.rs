//! `AutoPersistHelperTest.CollectionPersistence`
//! (`ReactiveUI.Tests/AutoPersist/AutoPersistHelperTest.CollectionPersistence.cs`).

use super::{counting_save, ms, seconds, with_virtual_time};
use crate::mocks::TestFixture;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactiveui::*;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::{IDisposable, IObserver};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;
use std::time::Duration;

const INITIAL_ADVANCE_MILLISECONDS: u64 = 10;
const THROTTLE_MILLISECONDS: u64 = 100;
const PAST_THROTTLE_MILLISECONDS: u64 = 150;
const DEFAULT_INTERVAL_SECONDS: u64 = 3;

// Not ported: AutoPersistCollection_NullCollection_ThrowsArgumentNullException - argument validation
// Not ported: AutoPersistCollection_NullDoPersist_ThrowsArgumentNullException - argument validation
// Not ported: AutoPersistCollection_NullMetadata_ThrowsArgumentNullException - argument validation
// Not ported: AutoPersistCollection_ReadOnlyCollection_WorksCorrectly - needs ReadOnlyObservableCollection in ferroui-system

#[test]
fn auto_persist_collection_remove_item_disables_persistence() {
    with_virtual_time(|scheduler| {
        let item = TestFixture::new();
        let collection = ObservableCollection::from_items(vec![item.clone()]);
        let save_count = Rc::new(Cell::new(0));
        let _ = collection.auto_persist_collection_with_interval(counting_save(&save_count), Some(Duration::from_millis(THROTTLE_MILLISECONDS)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let _ = collection.remove(&item);
        item.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_collection_with_metadata_saves_correctly() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        let metadata = AutoPersistMetadata::for_type::<TestFixture>();
        let save_count = Rc::new(Cell::new(0));
        let _ = collection.auto_persist_collection_metadata_with_interval(counting_save(&save_count), metadata, Some(Duration::from_millis(THROTTLE_MILLISECONDS)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let item = TestFixture::new();
        collection.add(item.clone());
        scheduler.advance_by(ms(1));
        item.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn create_metadata_returns_correct_metadata() {
    let metadata = AutoPersistMetadata::for_type::<TestFixture>();

    assert!(metadata.has_data_contract());
    assert!(metadata.persistable_property_names().contains("IsNotNullString"));
    assert!(metadata.persistable_property_names().contains("IsOnlyOneWord"));
    assert!(!metadata.persistable_property_names().contains("PocoProperty"));
}

// Not ported: CreateMetadata_WithoutDataContract_ReturnsMetadataWithoutContract - reflection (a type without the contract does not implement DataContract)

#[test]
fn create_metadata_caches_metadata() {
    let metadata1 = AutoPersistMetadata::for_type::<TestFixture>();
    let metadata2 = AutoPersistMetadata::for_type::<TestFixture>();

    assert!(Rc::ptr_eq(&metadata1, &metadata2));
}

#[test]
fn auto_persist_collection_manual_signal_and_metadata_no_interval_saves_correctly() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        let metadata = AutoPersistMetadata::for_type::<TestFixture>();
        let manual_save = Subject::<()>::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = collection.auto_persist_collection_metadata_with(counting_save(&save_count), manual_save.as_observable(), metadata, None);

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let item = TestFixture::new();
        collection.add(item);
        scheduler.advance_by(ms(1));
        manual_save.on_next(());
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_collection_manual_signal_no_interval_saves_correctly() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        let manual_save = Subject::<()>::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = collection.auto_persist_collection_with(counting_save(&save_count), manual_save.as_observable(), None);

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let item = TestFixture::new();
        collection.add(item);
        scheduler.advance_by(ms(1));
        manual_save.on_next(());
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 1);
    });
}

// Not ported: AutoPersistCollection_ReadOnlyManualSignalAndMetadataNoInterval_SavesCorrectly - needs ReadOnlyObservableCollection in ferroui-system
// Not ported: AutoPersistCollection_ReadOnlyManualSignalNoInterval_SavesCorrectly - needs ReadOnlyObservableCollection in ferroui-system

#[test]
fn auto_persist_collection_metadata_provider_no_interval_saves_correctly() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        let metadata_provider = AutoPersistMetadata::provider_for_type::<TestFixture>();
        let manual_save = Subject::<()>::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = collection.auto_persist_collection_metadata_provider(counting_save(&save_count), manual_save.as_observable(), metadata_provider, None);

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let item = TestFixture::new();
        collection.add(item.clone());
        scheduler.advance_by(ms(1));
        item.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_collection_metadata_provider_dispose_stops_persistence() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        let metadata_provider = AutoPersistMetadata::provider_for_type::<TestFixture>();
        let save_count = Rc::new(Cell::new(0));
        let subscription = collection.auto_persist_collection_metadata_provider(
            counting_save(&save_count),
            Subject::<()>::new().as_observable(),
            metadata_provider,
            Some(Duration::from_millis(THROTTLE_MILLISECONDS)),
        );

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let item = TestFixture::new();
        collection.add(item.clone());
        scheduler.advance_by(ms(1));
        subscription.dispose();
        item.set_is_not_null_string(Some("AfterDispose".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_collection_metadata_without_data_contract_throws_argument_exception() {
    let collection = ObservableCollection::<Rc<TestFixture>>::new();
    let metadata = Rc::new(AutoPersistMetadata::new(false, Vec::<String>::new()));

    let outcome = catch_unwind(AssertUnwindSafe(|| {
        collection.auto_persist_collection_metadata_with(|_| Observable::return_(()), Subject::<()>::new().as_observable(), metadata, None)
    }));

    assert!(outcome.is_err());
}

// Not ported: AutoPersistCollection_GenericMetadataNoInterval_SavesCorrectly - needs a typed change notification of collections other than ObservableCollection in ferroui-system (the port takes an ObservableCollection)

#[test]
fn auto_persist_dispose_with_pending_save_does_not_persist() {
    with_virtual_time(|scheduler| {
        const INTERVAL_SECONDS: u64 = 2;
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let subscription = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(INTERVAL_SECONDS)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some("Pending".to_owned()));
        scheduler.advance_by(seconds(1));
        subscription.dispose();
        scheduler.advance_by(seconds(INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_non_persistable_raised_property_does_not_trigger_save() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(1)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_not_serialized(Some("Changed".to_owned()));
        scheduler.advance_by(seconds(1));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_manual_signal_no_interval_saves_correctly() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let manual_save = Subject::<()>::new();
        let save_count = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_with(counting_save(&save_count), manual_save.as_observable(), None);

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        manual_save.on_next(());
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 1);
    });
}

// Not ported: AutoPersist_DerivedRuntimeType_UsesRuntimeMetadata - reflection (the metadata of the runtime type of a derived class)

/// The items `on_add` and `on_remove` received, and the subscription of `ActOnEveryObject` to
/// `change_sets`.
fn act_on_every_change_set(
    change_sets: &Rc<Subject<ReactiveChangeSet<Rc<TestFixture>>>>,
) -> (Rc<RefCell<Vec<Rc<TestFixture>>>>, Rc<RefCell<Vec<Rc<TestFixture>>>>, Rc<dyn IDisposable>) {
    let adds = Rc::new(RefCell::new(Vec::new()));
    let removes = Rc::new(RefCell::new(Vec::new()));
    let added = adds.clone();
    let removed = removes.clone();
    let subscription = change_sets
        .as_observable()
        .act_on_every_object(move |item| added.borrow_mut().push(item), move |item| removed.borrow_mut().push(item));
    (adds, removes, subscription)
}

/// Whether `items` holds `item` (`Contains`, by reference).
fn contains(items: &RefCell<Vec<Rc<TestFixture>>>, item: &Rc<TestFixture>) -> bool {
    items.borrow().iter().any(|candidate| Rc::ptr_eq(candidate, item))
}

#[test]
fn act_on_every_object_refresh_change_calls_remove_then_add() {
    let item = TestFixture::new();
    let change_sets = Subject::<ReactiveChangeSet<Rc<TestFixture>>>::new();
    let (adds, removes, subscription) = act_on_every_change_set(&change_sets);

    change_sets.on_next(ReactiveChangeSet::new(vec![ReactiveChange::new(ReactiveChangeReason::Refresh, item.clone(), None, 0, -1)]));

    assert!(contains(&removes, &item));
    assert!(contains(&adds, &item));
    subscription.dispose();
}

#[test]
fn act_on_every_object_replace_change_with_previous_calls_remove_and_add() {
    let previous = TestFixture::new();
    let current = TestFixture::new();
    let change_sets = Subject::<ReactiveChangeSet<Rc<TestFixture>>>::new();
    let (adds, removes, subscription) = act_on_every_change_set(&change_sets);

    change_sets.on_next(ReactiveChangeSet::new(vec![ReactiveChange::new(
        ReactiveChangeReason::Replace,
        current.clone(),
        Some(previous.clone()),
        0,
        -1,
    )]));

    assert!(contains(&removes, &previous));
    assert!(contains(&adds, &current));
    subscription.dispose();
}

// Not ported: AutoPersistCollection_ReplaceUntrackedPrevious_DisposeAndRemoveSkips - needs a typed change notification of collections other than ObservableCollection in ferroui-system (a replace of an item never added)
// Not ported: AutoPersistCollection_GenericReflectionManualSignalNoInterval_SavesCorrectly - needs a typed change notification of collections other than ObservableCollection in ferroui-system
