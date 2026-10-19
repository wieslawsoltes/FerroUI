//! `AutoPersistCollectionTest` (`ReactiveUI.Tests/AutoPersist/AutoPersistCollectionTest.cs`).
//!
//! Upstream's `ResettableCollection<T>` raises a reset of an `ObservableCollection<T>`; the
//! collection of `ferroui-system` raises it with `on_collection_reset`.

use super::{counting_save, ms, with_virtual_time};
use crate::mocks::TestFixture;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::Observable;
use ferroui_reactiveui::*;
use ferroui_system::collections::object_model::ObservableCollection;
use ferroui_system::IObserver;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

const INITIAL_ADVANCE_MILLISECONDS: u64 = 10;
const THROTTLE_MILLISECONDS: u64 = 100;
const PAST_THROTTLE_MILLISECONDS: u64 = 110;

fn throttle() -> Option<Duration> {
    Some(Duration::from_millis(THROTTLE_MILLISECONDS))
}

#[test]
fn auto_persist_collection_dispose_stops_all_persistence() {
    with_virtual_time(|scheduler| {
        let manual_save = Subject::<()>::new();
        let item = TestFixture::new();
        let fixture = ObservableCollection::from_items(vec![item.clone()]);
        let times_saved = Rc::new(Cell::new(0));
        let disp = fixture.auto_persist_collection_with(counting_save(&times_saved), manual_save.as_observable(), throttle());

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        assert_eq!(times_saved.get(), 0);

        item.set_is_not_null_string(Some("Foo".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        disp.dispose();
        fixture.clear();
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        item.set_is_not_null_string(Some("Bar".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        fixture.add(item.clone());
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Baz".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        fixture.on_collection_reset();
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Bamf".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        fixture.remove_at(0);
        item.set_is_not_null_string(Some("Blomf".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);
    });
}

#[test]
fn auto_persist_collection_duplicate_add_no_double_save() {
    with_virtual_time(|scheduler| {
        let item = TestFixture::new();
        let fixture = ObservableCollection::<Rc<TestFixture>>::new();
        let times_saved = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_collection_with_interval(counting_save(&times_saved), throttle());

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        fixture.add(item.clone());
        fixture.add(item.clone());
        scheduler.advance_by(ms(1));
        item.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(times_saved.get(), 1);
    });
}

#[test]
fn auto_persist_collection_lifecycle_manages_persistence() {
    with_virtual_time(|scheduler| {
        const EXPECTED_SAVES_AFTER_RE_ADD: i32 = 2;
        const EXPECTED_SAVES_AFTER_RESET: i32 = 3;
        let manual_save = Subject::<()>::new();
        let item = TestFixture::new();
        let fixture = ObservableCollection::from_items(vec![item.clone()]);
        let times_saved = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_collection_with(counting_save(&times_saved), manual_save.as_observable(), throttle());

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        assert_eq!(times_saved.get(), 0);

        item.set_is_not_null_string(Some("Foo".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        fixture.clear();
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Bar".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        fixture.add(item.clone());
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Baz".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), EXPECTED_SAVES_AFTER_RE_ADD);

        fixture.on_collection_reset();
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Bamf".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), EXPECTED_SAVES_AFTER_RESET);

        fixture.remove_at(0);
        item.set_is_not_null_string(Some("Blomf".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), EXPECTED_SAVES_AFTER_RESET);
    });
}

#[test]
fn auto_persist_collection_manual_save_triggers_immediate_save() {
    with_virtual_time(|scheduler| {
        let manual_save = Subject::<()>::new();
        let item = TestFixture::new();
        let fixture = ObservableCollection::from_items(vec![item]);
        let times_saved = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_collection_with(counting_save(&times_saved), manual_save.as_observable(), throttle());

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        assert_eq!(times_saved.get(), 0);

        manual_save.on_next(());
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);
    });
}

#[test]
fn auto_persist_collection_metadata_provider_works_correctly() {
    with_virtual_time(|scheduler| {
        let item = TestFixture::new();
        let fixture = ObservableCollection::from_items(vec![item.clone()]);
        let metadata_provider = AutoPersistMetadata::provider_for_type::<TestFixture>();
        let times_saved = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_collection_metadata_provider(counting_save(&times_saved), Observable::never::<()>(), metadata_provider, throttle());

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Test".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));

        assert_eq!(times_saved.get(), 1);
    });
}

#[test]
fn auto_persist_collection_reset_continues_persistence() {
    with_virtual_time(|scheduler| {
        const EXPECTED_SAVES_AFTER_RESET: i32 = 2;
        let item = TestFixture::new();
        let fixture = ObservableCollection::from_items(vec![item.clone()]);
        let times_saved = Rc::new(Cell::new(0));
        let _ = fixture.auto_persist_collection_with_interval(counting_save(&times_saved), throttle());

        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("Before".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), 1);

        fixture.on_collection_reset();
        scheduler.advance_by(ms(INITIAL_ADVANCE_MILLISECONDS));
        item.set_is_not_null_string(Some("After".to_owned()));
        scheduler.advance_by(ms(PAST_THROTTLE_MILLISECONDS));
        assert_eq!(times_saved.get(), EXPECTED_SAVES_AFTER_RESET);
    });
}
