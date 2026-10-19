//! `ReactiveRecordTests` (`ReactiveUI.Tests/ReactiveObjects/ReactiveRecordTests.cs`): records
//! whose notification state is not part of their value.

use crate::mocks::collect_items;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A record with two members (`TestRecord`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
struct TestRecord {
    reactive: ReactiveRecord,
    name: Option<String>,
    age: i32,
}
reactive_object!(TestRecord);

reactive_properties! {
    /// A record with a notifying property (`TestMutableRecord`).
    #[derive(Default)]
    struct TestMutableRecord {
        reactive: ReactiveRecord,
        #[reactive]
        value: i32,
    }
}
reactive_object!(TestMutableRecord);

/// Asserts that every count is `expected` (`AssertCount`).
fn assert_count(expected: usize, counts: &[usize]) {
    for count in counts {
        assert_eq!(*count, expected);
    }
}

fn hash_of(record: &TestRecord) -> u64 {
    let mut hasher = DefaultHasher::new();
    record.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn changing_observable_should_fire_before_property_changes() {
    const NEW_VALUE: i32 = 42;
    let fixture = Rc::new(TestMutableRecord::default());
    let changing_fired = Rc::new(Cell::new(false));
    let changed_fired = Rc::new(Cell::new(false));

    let fired = changing_fired.clone();
    let sub1 = fixture.changing().observe_on(ImmediateScheduler::instance()).subscribe_next(move |_| fired.set(true));
    let fired = changed_fired.clone();
    let sub2 = fixture.changed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |_| fired.set(true));

    fixture.set_value(NEW_VALUE);

    // Verify both events fired
    assert!(changing_fired.get());
    assert!(changed_fired.get());
    sub1.dispose();
    sub2.dispose();
}

#[test]
fn delay_change_notifications_should_defer_notifications() {
    const SECOND_VALUE: i32 = 2;
    const THIRD_VALUE: i32 = 3;
    const EXPECTED_COUNT_AFTER_DISPOSE: usize = 2;
    let fixture = Rc::new(TestMutableRecord::default());
    let changed = collect_items(&fixture.changed());
    let changing = collect_items(&fixture.changing());
    let counts = || [changing.borrow().len(), changed.borrow().len()];

    assert_count(0, &counts());

    fixture.set_value(1);
    assert_count(1, &counts());

    let delay = fixture.delay_change_notifications();
    fixture.set_value(SECOND_VALUE);
    assert_count(1, &counts());

    fixture.set_value(THIRD_VALUE);
    assert_count(1, &counts());
    delay.dispose();

    // After disposing, delayed notifications should fire
    assert_count(EXPECTED_COUNT_AFTER_DISPOSE, &counts());
}

#[test]
fn exceptions_should_marshal_to_thrown_exceptions() {
    const SECOND_VALUE: i32 = 2;
    let fixture = Rc::new(TestMutableRecord::default());
    fixture.set_value(1);

    let sub1 = fixture.changed().subscribe_next(|_| panic!("Test exception"));
    let exceptions = collect_items(&fixture.thrown_exceptions());

    fixture.set_value(SECOND_VALUE);

    assert!(!exceptions.borrow().is_empty());
    sub1.dispose();
}

#[test]
fn nested_delay_change_notifications_should_work() {
    const SECOND_VALUE: i32 = 2;
    let fixture = Rc::new(TestMutableRecord::default());
    let changed = collect_items(&fixture.changed());
    let changing = collect_items(&fixture.changing());
    let counts = || [changing.borrow().len(), changed.borrow().len()];

    assert_count(0, &counts());

    let outer = fixture.delay_change_notifications();
    fixture.set_value(1);
    assert_count(0, &counts());

    let inner = fixture.delay_change_notifications();
    fixture.set_value(SECOND_VALUE);
    assert_count(0, &counts());

    outer.dispose();
    assert_count(0, &counts()); // Still delayed by inner

    inner.dispose();
    assert_count(1, &counts()); // Now notifications fire
}

#[test]
fn property_changed_event_should_fire() {
    const NEW_VALUE: i32 = 42;
    let fixture = Rc::new(TestMutableRecord::default());
    let fired = Rc::new(Cell::new(false));
    let property_name = Rc::new(RefCell::new(String::new()));

    let (was_fired, name) = (fired.clone(), property_name.clone());
    fixture.property_changed().add(Rc::new(move |args: &str| {
        was_fired.set(true);
        *name.borrow_mut() = args.to_owned();
    }));

    fixture.set_value(NEW_VALUE);

    assert!(fired.get());
    assert_eq!(*property_name.borrow(), "Value");
}

#[test]
fn property_changing_event_should_fire() {
    const NEW_VALUE: i32 = 42;
    let fixture = Rc::new(TestMutableRecord::default());
    let fired = Rc::new(Cell::new(false));
    let property_name = Rc::new(RefCell::new(String::new()));

    let (was_fired, name) = (fired.clone(), property_name.clone());
    fixture.property_changing().add(Rc::new(move |args: &str| {
        was_fired.set(true);
        *name.borrow_mut() = args.to_owned();
    }));

    fixture.set_value(NEW_VALUE);

    assert!(fired.get());
    assert_eq!(*property_name.borrow(), "Value");
}

// Not ported: ReactiveRecordShouldNotSerializeInternalReactiveProperties - serialization

#[test]
fn reactive_record_should_raise_property_change_notifications() {
    let fixture = Rc::new(TestRecord { name: Some("Initial".to_owned()), ..TestRecord::default() });
    let changing_events = Rc::new(RefCell::new(Vec::<String>::new()));
    let changed_events = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = changing_events.clone();
    let sub1 = fixture.changing().select(|x| x.property_name).subscribe_next(move |name| sink.borrow_mut().push(name));
    let sink = changed_events.clone();
    let sub2 = fixture.changed().select(|x| x.property_name).subscribe_next(move |name| sink.borrow_mut().push(name));

    let updated = Rc::new(TestRecord { name: Some("Updated".to_owned()), ..(*fixture).clone() });

    // Records create new instances, so we need to subscribe to the new instance
    let sink = changing_events.clone();
    let sub3 = updated.changing().select(|x| x.property_name).subscribe_next(move |name| sink.borrow_mut().push(name));
    let sink = changed_events.clone();
    let sub4 = updated.changed().select(|x| x.property_name).subscribe_next(move |name| sink.borrow_mut().push(name));

    assert_eq!(updated.name.as_deref(), Some("Updated"));
    for subscription in [sub1, sub2, sub3, sub4] {
        subscription.dispose();
    }
}

// Not ported: ReactiveRecordShouldSerializeCorrectly - serialization

#[test]
fn removing_property_changed_handler_should_work() {
    let fixture = Rc::new(TestMutableRecord::default());
    let call_count = Rc::new(Cell::new(0));

    const SECOND_VALUE: i32 = 2;
    let calls = call_count.clone();
    let handler = fixture.property_changed().add(Rc::new(move |_: &str| calls.set(calls.get() + 1)));
    fixture.set_value(1);
    assert_eq!(call_count.get(), 1);

    fixture.property_changed().remove(handler);
    fixture.set_value(SECOND_VALUE);
    assert_eq!(call_count.get(), 1); // Should not have incremented
}

#[test]
fn removing_property_changing_handler_should_work() {
    let fixture = Rc::new(TestMutableRecord::default());
    let call_count = Rc::new(Cell::new(0));

    const SECOND_VALUE: i32 = 2;
    let calls = call_count.clone();
    let handler = fixture.property_changing().add(Rc::new(move |_: &str| calls.set(calls.get() + 1)));
    fixture.set_value(1);
    assert_eq!(call_count.get(), 1);

    fixture.property_changing().remove(handler);
    fixture.set_value(SECOND_VALUE);
    assert_eq!(call_count.get(), 1); // Should not have incremented
}

#[test]
fn suppress_change_notifications_should_work() {
    let fixture = Rc::new(TestMutableRecord::default());

    assert!(fixture.are_change_notifications_enabled());

    let suppression = fixture.suppress_change_notifications();
    assert!(!fixture.are_change_notifications_enabled());
    suppression.dispose();

    assert!(fixture.are_change_notifications_enabled());
}

/// Upstream's `ToString` prints the members of the record and none of its base. A derived
/// `Debug` prints every field; the part prints its name and no member.
#[test]
fn to_string_should_print_only_derived_members() {
    const AGE_VALUE: i32 = 25;
    let fixture = TestRecord { name: Some("Test".to_owned()), age: AGE_VALUE, ..TestRecord::default() };

    assert_eq!(format!("{fixture:?}"), r#"TestRecord { reactive: ReactiveRecord, name: Some("Test"), age: 25 }"#);
}

#[test]
fn records_with_equal_members_should_be_equal_after_reactive_state_is_used() {
    const AGE_VALUE: i32 = 25;
    let first = Rc::new(TestRecord { name: Some("Test".to_owned()), age: AGE_VALUE, ..TestRecord::default() });
    let second = Rc::new(TestRecord { name: Some("Test".to_owned()), age: AGE_VALUE, ..TestRecord::default() });

    let _ = first.changing();
    let _ = first.changed();
    let _ = first.thrown_exceptions();
    first.property_changed().add(Rc::new(|_: &str| {}));
    first.property_changing().add(Rc::new(|_: &str| {}));

    assert!(first.eq(&second));
    assert!(*first == *second);
    assert_eq!(hash_of(&first), hash_of(&second));
}

#[test]
fn records_with_different_members_should_not_be_equal() {
    const AGE_VALUE: i32 = 25;
    let first = TestRecord { name: Some("Test".to_owned()), age: AGE_VALUE, ..TestRecord::default() };
    let second = TestRecord { name: Some("Other".to_owned()), age: AGE_VALUE, ..TestRecord::default() };

    assert!(!first.eq(&second));
}

#[test]
fn thrown_exceptions_observable_should_be_initialized() {
    let fixture = TestRecord::default();
    let errors = collect_items(&fixture.thrown_exceptions());
    assert!(errors.borrow().is_empty());
}
