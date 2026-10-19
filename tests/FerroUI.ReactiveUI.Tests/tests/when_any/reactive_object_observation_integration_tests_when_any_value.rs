//! `ReactiveObjectObservationIntegrationTests` (`ReactiveUI.Tests/WhenAny/ReactiveObjectObservationIntegrationTests.WhenAnyValue.cs`):
//! `WhenAnyValue` over up to twelve properties of a reactive object.
//!
//! Upstream's overloads by arity are one method taking a tuple of paths here; the values of
//! several paths are a tuple, and the projector takes them as one tuple. The odd properties of
//! [`WhenAnyTestFixture`] are strings and the even ones are unset optional strings, which
//! concatenate as the empty string, as C#'s `+` concatenates null.

use crate::mocks::WhenAnyTestFixture;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;
use std::cell::RefCell;
use std::rc::Rc;

/// Test value "A".
const A_TEXT: &str = "A";
/// Test value "B".
const B_TEXT: &str = "B";
/// Test value "1".
const ONE_TEXT: &str = "1";
/// Test value "13".
const ONE_THREE_TEXT: &str = "13";
/// Test value "135".
const ONE_THREE_FIVE_TEXT: &str = "135";
/// Test value "1357".
const ONE_THREE_FIVE_SEVEN_TEXT: &str = "1357";

/// The text of an optional string, empty when there is none (C#'s concatenation of null).
fn text_of(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("")
}

#[test]
fn when_any_value_with10_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
                prop!(WhenAnyTestFixture => value7),
                prop!(WhenAnyTestFixture => value8),
                prop!(WhenAnyTestFixture => value9),
                prop!(WhenAnyTestFixture => value10),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str(), text_of(&value8), value9.as_str(), text_of(&value10)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some("13579"));
}

#[test]
fn when_any_value_with11_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
                prop!(WhenAnyTestFixture => value7),
                prop!(WhenAnyTestFixture => value8),
                prop!(WhenAnyTestFixture => value9),
                prop!(WhenAnyTestFixture => value10),
                prop!(WhenAnyTestFixture => value11),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str(), text_of(&value8), value9.as_str(), text_of(&value10), value11.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some("1357911"));
}

#[test]
fn when_any_value_with12_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
                prop!(WhenAnyTestFixture => value7),
                prop!(WhenAnyTestFixture => value8),
                prop!(WhenAnyTestFixture => value9),
                prop!(WhenAnyTestFixture => value10),
                prop!(WhenAnyTestFixture => value11),
                prop!(WhenAnyTestFixture => value12),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6, value7, value8, value9, value10, value11, value12)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str(), text_of(&value8), value9.as_str(), text_of(&value10), value11.as_str(), text_of(&value12)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some("1357911"));
}

#[test]
fn when_any_value_with1_paramerters() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture.when_any_value(prop!(WhenAnyTestFixture => value1)).subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_TEXT));
}

// `Value1` is a `String` here, which cannot be null; the null that upstream forces into it
// (`null!`) is the empty string.
#[test]
fn when_any_value_with1_paramerters_sequential_check() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    fixture.set_value1(String::new());

    let slot = result.clone();
    let _ = fixture.when_any_value(prop!(WhenAnyTestFixture => value1)).subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(""));

    fixture.set_value1(A_TEXT.to_owned());

    assert_eq!(result.borrow().as_deref(), Some(A_TEXT));

    fixture.set_value1(B_TEXT.to_owned());

    assert_eq!(result.borrow().as_deref(), Some(B_TEXT));

    fixture.set_value1(String::new());

    assert_eq!(result.borrow().as_deref(), Some(""));
}

#[test]
fn when_any_value_with1_paramerters_sequential_check_nullable() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(Some(String::new())));

    let slot = result.clone();
    let _ = fixture.when_any_value(prop!(WhenAnyTestFixture => value2)).subscribe_next(move |value| *slot.borrow_mut() = value);

    assert!(result.borrow().is_none());

    fixture.set_value2(Some(A_TEXT.to_owned()));

    assert_eq!(result.borrow().as_deref(), Some(A_TEXT));

    fixture.set_value2(Some(B_TEXT.to_owned()));

    assert_eq!(result.borrow().as_deref(), Some(B_TEXT));

    fixture.set_value2(None);

    assert!(result.borrow().is_none());
}

#[test]
fn when_any_value_with2_paramerters_returns_tuple() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((
            prop!(WhenAnyTestFixture => value1),
            prop!(WhenAnyTestFixture => value2),
        ))
        .select(|(value1, value2)| [value1.as_str(), text_of(&value2)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_TEXT));
}

#[test]
fn when_any_value_with2_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
            ),
            |values| values,
        )
        .select(|(value1, value2)| [value1.as_str(), text_of(&value2)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_TEXT));
}

#[test]
fn when_any_value_with3_paramerters_returns_tuple() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((
            prop!(WhenAnyTestFixture => value1),
            prop!(WhenAnyTestFixture => value2),
            prop!(WhenAnyTestFixture => value3),
        ))
        .select(|(value1, value2, value3)| [value1.as_str(), text_of(&value2), value3.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_TEXT));
}

#[test]
fn when_any_value_with3_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3)| [value1.as_str(), text_of(&value2), value3.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_TEXT));
}

#[test]
fn when_any_value_with4_paramerters_returns_tuple() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((
            prop!(WhenAnyTestFixture => value1),
            prop!(WhenAnyTestFixture => value2),
            prop!(WhenAnyTestFixture => value3),
            prop!(WhenAnyTestFixture => value4),
        ))
        .select(|(value1, value2, value3, value4)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_TEXT));
}

#[test]
fn when_any_value_with4_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_TEXT));
}

#[test]
fn when_any_value_with5_paramerters_returns_tuple() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((
            prop!(WhenAnyTestFixture => value1),
            prop!(WhenAnyTestFixture => value2),
            prop!(WhenAnyTestFixture => value3),
            prop!(WhenAnyTestFixture => value4),
            prop!(WhenAnyTestFixture => value5),
        ))
        .select(|(value1, value2, value3, value4, value5)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_TEXT));
}

#[test]
fn when_any_value_with5_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_TEXT));
}

#[test]
fn when_any_value_with6_paramerters_returns_tuple() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((
            prop!(WhenAnyTestFixture => value1),
            prop!(WhenAnyTestFixture => value2),
            prop!(WhenAnyTestFixture => value3),
            prop!(WhenAnyTestFixture => value4),
            prop!(WhenAnyTestFixture => value5),
            prop!(WhenAnyTestFixture => value6),
        ))
        .select(|(value1, value2, value3, value4, value5, value6)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_TEXT));
}

#[test]
fn when_any_value_with6_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_TEXT));
}

#[test]
fn when_any_value_with7_paramerters_returns_tuple() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((
            prop!(WhenAnyTestFixture => value1),
            prop!(WhenAnyTestFixture => value2),
            prop!(WhenAnyTestFixture => value3),
            prop!(WhenAnyTestFixture => value4),
            prop!(WhenAnyTestFixture => value5),
            prop!(WhenAnyTestFixture => value6),
            prop!(WhenAnyTestFixture => value7),
        ))
        .select(|(value1, value2, value3, value4, value5, value6, value7)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_SEVEN_TEXT));
}

#[test]
fn when_any_value_with7_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
                prop!(WhenAnyTestFixture => value7),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6, value7)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_SEVEN_TEXT));
}

#[test]
fn when_any_value_with8_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
                prop!(WhenAnyTestFixture => value7),
                prop!(WhenAnyTestFixture => value8),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6, value7, value8)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str(), text_of(&value8)].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some(ONE_THREE_FIVE_SEVEN_TEXT));
}

#[test]
fn when_any_value_with9_paramerters_returns_values() {
    let fixture = WhenAnyTestFixture::new();

    let result = Rc::new(RefCell::new(None::<String>));

    let slot = result.clone();
    let _ = fixture
        .when_any_value_select(
            (
                prop!(WhenAnyTestFixture => value1),
                prop!(WhenAnyTestFixture => value2),
                prop!(WhenAnyTestFixture => value3),
                prop!(WhenAnyTestFixture => value4),
                prop!(WhenAnyTestFixture => value5),
                prop!(WhenAnyTestFixture => value6),
                prop!(WhenAnyTestFixture => value7),
                prop!(WhenAnyTestFixture => value8),
                prop!(WhenAnyTestFixture => value9),
            ),
            |values| values,
        )
        .select(|(value1, value2, value3, value4, value5, value6, value7, value8, value9)| [value1.as_str(), text_of(&value2), value3.as_str(), text_of(&value4), value5.as_str(), text_of(&value6), value7.as_str(), text_of(&value8), value9.as_str()].concat())
        .subscribe_next(move |value| *slot.borrow_mut() = Some(value));

    assert_eq!(result.borrow().as_deref(), Some("13579"));
}
