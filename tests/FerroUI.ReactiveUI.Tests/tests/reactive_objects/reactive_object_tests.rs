//! `ReactiveObjectTests` (`ReactiveUI.Tests/ReactiveObjects/ReactiveObjectTests.cs`): the change
//! notifications of a reactive object.

use crate::mocks::{collect_items, TestFixture};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::{INotifyPropertyChanged, INotifyPropertyChanging};
use ferroui_system::prop;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

/// The "Foo" text value used in property change tests.
const FOO_TEXT: &str = "Foo";
/// The "Bar" text value used in property change tests.
const BAR_TEXT: &str = "Bar";
/// The "Baz" text value used in property change tests.
const BAZ_TEXT: &str = "Baz";
/// The property name for the IsNotNullString property.
const IS_NOT_NULL_STRING_NAME: &str = "IsNotNullString";
/// The property name for the IsOnlyOneWord property.
const IS_ONLY_ONE_WORD_NAME: &str = "IsOnlyOneWord";
/// The property name for the NullableInt property.
const NULLABLE_INT_NAME: &str = "NullableInt";

fn text(value: &str) -> Option<String> {
    Some(value.to_owned())
}

/// Asserts that every count is `expected` (`AssertCount`).
fn assert_count(expected: usize, counts: &[usize]) {
    for count in counts {
        assert_eq!(*count, expected);
    }
}

#[test]
fn changing_should_always_arrive_before_changed() {
    let before_set = FOO_TEXT;
    let after_set = BAR_TEXT;

    let fixture = TestFixture::new();
    fixture.set_is_only_one_word(text(before_set));

    let before_fired = Rc::new(Cell::new(false));
    let changing_property_name = Rc::new(RefCell::new(None::<String>));
    let changing_value = Rc::new(RefCell::new(None::<String>));
    let (fired, name, value) = (before_fired.clone(), changing_property_name.clone(), changing_value.clone());
    let _ = fixture.changing().subscribe_next(move |x| {
        *name.borrow_mut() = Some(x.property_name.clone());
        *value.borrow_mut() = x.sender.is_only_one_word();
        fired.set(true);
    });

    let after_fired = Rc::new(Cell::new(false));
    let changed_property_name = Rc::new(RefCell::new(None::<String>));
    let changed_value = Rc::new(RefCell::new(None::<String>));
    let (fired, name, value) = (after_fired.clone(), changed_property_name.clone(), changed_value.clone());
    let _ = fixture.changed().subscribe_next(move |x| {
        *name.borrow_mut() = Some(x.property_name.clone());
        *value.borrow_mut() = x.sender.is_only_one_word();
        fired.set(true);
    });

    fixture.set_is_only_one_word(text(after_set));

    assert!(before_fired.get());
    assert!(after_fired.get());
    assert_eq!(changing_property_name.borrow().as_deref(), Some(IS_ONLY_ONE_WORD_NAME));
    assert_eq!(changing_value.borrow().as_deref(), Some(before_set));
    assert_eq!(changed_property_name.borrow().as_deref(), Some(IS_ONLY_ONE_WORD_NAME));
    assert_eq!(changed_value.borrow().as_deref(), Some(after_set));
}

#[test]
fn deferred_notifications_dont_show_up_until_undeferred() {
    const FIRST_INT: i32 = 4;
    const SECOND_INT: i32 = 5;
    const THIRD_INT: i32 = 6;
    const EXPECTED_COUNT_AFTER_UNDEFER: usize = 3;
    let fixture = TestFixture::new();
    let changing = collect_items(&fixture.changing());
    let changed = collect_items(&fixture.changed());
    let property_changing_events = Rc::new(RefCell::new(Vec::<String>::new()));
    let events = property_changing_events.clone();
    fixture.property_changing().add(Rc::new(move |name: &str| events.borrow_mut().push(name.to_owned())));
    let property_changed_events = Rc::new(RefCell::new(Vec::<String>::new()));
    let events = property_changed_events.clone();
    fixture.property_changed().add(Rc::new(move |name: &str| events.borrow_mut().push(name.to_owned())));

    let counts = || {
        [changing.borrow().len(), changed.borrow().len(), property_changing_events.borrow().len(), property_changed_events.borrow().len()]
    };

    assert_count(0, &counts());
    fixture.set_nullable_int(Some(FIRST_INT));
    assert_count(1, &counts());

    let stop_delaying = fixture.delay_change_notifications();

    fixture.set_nullable_int(Some(SECOND_INT));
    assert_count(1, &counts());

    fixture.set_is_not_null_string(text(BAR_TEXT));
    assert_count(1, &counts());

    fixture.set_nullable_int(Some(THIRD_INT));
    assert_count(1, &counts());

    fixture.set_is_not_null_string(text(BAZ_TEXT));
    assert_count(1, &counts());

    let stop_delaying_more = fixture.delay_change_notifications();

    fixture.set_is_not_null_string(text("Bamf"));
    assert_count(1, &counts());

    stop_delaying.dispose();

    fixture.set_is_not_null_string(text("Blargh"));
    assert_count(1, &counts());

    // NB: Because we debounce queued up notifications, we should only see a notification from
    // the latest NullableInt and the latest IsNotNullableString.
    stop_delaying_more.dispose();

    assert_count(EXPECTED_COUNT_AFTER_UNDEFER, &counts());

    let expected_event_properties = [NULLABLE_INT_NAME, NULLABLE_INT_NAME, IS_NOT_NULL_STRING_NAME];
    let names = |events: &Vec<ReactivePropertyChangedEventArgs<Rc<TestFixture>>>| events.iter().map(|e| e.property_name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&*changing.borrow()), expected_event_properties);
    assert_eq!(names(&*changed.borrow()), expected_event_properties);
    assert_eq!(*property_changing_events.borrow(), expected_event_properties);
    assert_eq!(*property_changed_events.borrow(), expected_event_properties);
}

#[test]
fn exceptions_thrown_in_subscribers_should_marshal_to_thrown_exceptions() {
    let fixture = TestFixture::new();
    fixture.set_is_only_one_word(text(FOO_TEXT));

    let _ = fixture.changed().subscribe_next(|_| panic!("Die!"));
    let exception_list = collect_items(&fixture.thrown_exceptions());

    fixture.set_is_only_one_word(text(BAR_TEXT));
    assert_eq!(exception_list.borrow().len(), 1);
    assert_eq!(exception_list.borrow()[0].to_string(), "Die!");
}

#[test]
fn when_any_using_expression() {
    const EXPECTED_COUNT: usize = 2;
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(text(FOO_TEXT));
    fixture.set_is_only_one_word(text(BAZ_TEXT));
    let output = Rc::new(RefCell::new(Vec::new()));
    let sink = output.clone();
    let subscription = fixture
        .when_any(prop!(TestFixture => is_not_null_string), |change| change)
        .skip(1)
        .subscribe_next(move |change| sink.borrow_mut().push(change));

    fixture.set_is_not_null_string(text(BAR_TEXT));
    fixture.set_is_not_null_string(text(BAZ_TEXT));
    fixture.set_is_not_null_string(text(BAZ_TEXT));

    fixture.set_is_only_one_word(text("Bamf"));

    let output = output.borrow();
    assert_eq!(output.len(), EXPECTED_COUNT);

    // Upstream's generated observation leaves `Expression` null; here it is the text of the path,
    // which the tests of the observation check.
    assert!(Rc::ptr_eq(&output[0].sender, &fixture));
    assert_eq!(output[0].value, text(BAR_TEXT));

    assert!(Rc::ptr_eq(&output[1].sender, &fixture));
    assert_eq!(output[1].value, text(BAZ_TEXT));
    subscription.dispose();
}

#[test]
fn raise_and_set_using_expression() {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(text(FOO_TEXT));
    fixture.set_is_only_one_word(text(BAZ_TEXT));
    let output = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = output.clone();
    let _ = fixture
        .changed()
        .select(|x| x.property_name)
        .subscribe_next(move |name| sink.borrow_mut().push(name));

    fixture.set_uses_expr_raise_set(text(FOO_TEXT));
    fixture.set_uses_expr_raise_set(text(FOO_TEXT)); // This one shouldn't raise a change notification

    assert_eq!(fixture.uses_expr_raise_set(), text(FOO_TEXT));
    assert_eq!(output.borrow().len(), 1);
    assert_eq!(output.borrow()[0], "UsesExprRaiseSet");
}

#[test]
fn reactive_object_can_suppress_change_notifications() {
    let fixture = TestFixture::new();
    let suppression = fixture.suppress_change_notifications();
    assert!(!fixture.are_change_notifications_enabled());
    suppression.dispose();

    assert!(fixture.are_change_notifications_enabled());

    // Not ported: the second half of the test serializes the fixture with System.Text.Json and
    // checks the suppression of the copy - serialization.
}

// Not ported: ReactiveObjectShouldntSerializeAnythingExtra - serialization

#[test]
fn reactive_object_should_rethrow_exception() {
    let fixture = TestFixture::new();
    let observable = fixture.when_any_value(prop!(TestFixture => is_only_one_word)).skip(1);
    let _ = observable.subscribe_next(|_| panic!("This is a test."));

    let result = catch_unwind(AssertUnwindSafe(|| fixture.set_is_only_one_word(text("Two Words"))));
    assert!(result.is_err());
}

#[test]
fn reactive_object_smoke_test() {
    let output_changing = Rc::new(RefCell::new(Vec::<String>::new()));
    let output = Rc::new(RefCell::new(Vec::<String>::new()));
    let fixture = TestFixture::new();

    let sink = output_changing.clone();
    let _ = fixture.changing().select(|x| x.property_name).subscribe_next(move |name| sink.borrow_mut().push(name));
    let sink = output.clone();
    let _ = fixture.changed().select(|x| x.property_name).subscribe_next(move |name| sink.borrow_mut().push(name));

    fixture.set_is_not_null_string(text("Foo Bar Baz"));
    fixture.set_is_only_one_word(text(FOO_TEXT));
    fixture.set_is_only_one_word(text(BAR_TEXT));
    fixture.set_is_not_null_string(None); // Sorry.
    fixture.set_is_not_null_string(None);

    let results = [IS_NOT_NULL_STRING_NAME, IS_ONLY_ONE_WORD_NAME, IS_ONLY_ONE_WORD_NAME, IS_NOT_NULL_STRING_NAME];

    assert_eq!(output.borrow().len(), results.len());

    assert_eq!(*output.borrow(), *output_changing.borrow());
    assert_eq!(*output.borrow(), results);
}
