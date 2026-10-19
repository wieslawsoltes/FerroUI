//! `AutoPersistHelperTest.HandleLifetime` (`ReactiveUI.Tests/AutoPersist/AutoPersistHelperTest.HandleLifetime.cs`).

use super::{counting_save, ms, seconds, with_virtual_time};
use crate::mocks::TestFixture;
use ferroui_reactive::Observable;
use ferroui_reactiveui::*;
use ferroui_system::collections::object_model::ObservableCollection;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const INITIAL_ADVANCE_MILLISECONDS: u64 = 10;
const THROTTLE_MILLISECONDS: u64 = 100;
const PAST_THROTTLE_MILLISECONDS: u64 = 150;
const DEFAULT_INTERVAL_SECONDS: u64 = 3;
const COLLECTION_ITEM_COUNT: usize = 3;
const CHANGED_VALUE: &str = "changed";

#[test]
fn auto_persist_handle_disposed_before_scheduled_start_never_saves() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let handle = fixture.auto_persist_with_interval(counting_save(&save_count), Some(Duration::from_secs(1)));

        // The start is still queued on the main-thread scheduler: the handle exists, the driver
        // does not.
        handle.dispose();
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some(CHANGED_VALUE.to_owned()));
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_with_metadata_handle_disposed_before_scheduled_start_never_saves() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let handle = fixture.auto_persist_metadata_with_interval(counting_save(&save_count), AutoPersistMetadata::for_type::<TestFixture>(), Some(Duration::from_secs(1)));

        handle.dispose();
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some(CHANGED_VALUE.to_owned()));
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_with_metadata_handle_disposed_after_start_stops_saving() {
    with_virtual_time(|scheduler| {
        let fixture = TestFixture::new();
        let save_count = Rc::new(Cell::new(0));
        let handle = fixture.auto_persist_metadata_with_interval(counting_save(&save_count), AutoPersistMetadata::for_type::<TestFixture>(), Some(Duration::from_secs(1)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.set_is_not_null_string(Some("first".to_owned()));
        scheduler.advance_by(seconds(1));
        assert_eq!(save_count.get(), 1);

        handle.dispose();
        fixture.set_is_not_null_string(Some("second".to_owned()));
        scheduler.advance_by(seconds(DEFAULT_INTERVAL_SECONDS));
        assert_eq!(save_count.get(), 1);
    });
}

#[test]
fn auto_persist_collection_handle_disposed_stops_saving_every_item() {
    with_virtual_time(|scheduler| {
        let collection = ObservableCollection::<Rc<TestFixture>>::new();
        for _ in 0..COLLECTION_ITEM_COUNT {
            collection.add(TestFixture::new());
        }
        let save_count = Rc::new(Cell::new(0));
        let handle = collection.auto_persist_collection_with_interval(counting_save(&save_count), Some(Duration::from_millis(THROTTLE_MILLISECONDS)));

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        handle.dispose();
        for item in collection.to_vec() {
            item.set_is_not_null_string(Some(CHANGED_VALUE.to_owned()));
        }
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(save_count.get(), 0);
    });
}

#[test]
fn auto_persist_collection_item_removed_stops_saving_only_that_item() {
    with_virtual_time(|scheduler| {
        let removed = TestFixture::new();
        let kept = TestFixture::new();
        let collection = ObservableCollection::from_items(vec![removed.clone(), kept.clone()]);
        let saved_items: Rc<RefCell<Vec<Rc<TestFixture>>>> = Rc::new(RefCell::new(Vec::new()));
        let log = saved_items.clone();
        let handle = collection.auto_persist_collection_with_interval(
            move |item| {
                log.borrow_mut().push(item);
                Observable::return_(())
            },
            Some(Duration::from_millis(THROTTLE_MILLISECONDS)),
        );

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        let _ = collection.remove(&removed);
        removed.set_is_not_null_string(Some(CHANGED_VALUE.to_owned()));
        kept.set_is_not_null_string(Some(CHANGED_VALUE.to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        let saved_items = saved_items.borrow();
        assert_eq!(saved_items.len(), 1);
        assert!(Rc::ptr_eq(&saved_items[0], &kept));
        handle.dispose();
    });
}

// Not ported: AutoPersist_StartedAndDisposedFromManyThreads_NeverSavesAfterDisposal - real threads
