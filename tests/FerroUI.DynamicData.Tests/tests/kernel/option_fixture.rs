//! `OptionFixture` (`DynamicData.Tests/Kernal/OptionFixture.cs`): the extensions of an option.
//!
//! Upstream's `Optional<T>` is `Option<T>`: a null reference is `None`, the implicit conversion of
//! a value is `Some` (`From<T>`), `HasValue` is `is_some`, and `OrElse` is `Option::or_else`. A
//! nullable `Optional<T>?` is an `Option<Option<T>>`, which `flatten` makes an option.

use crate::domain::Person;
use ferroui_dynamic_data::cache::ChangeSet;
use ferroui_dynamic_data::kernel::OptionExtensions;
use std::cell::Cell;
use std::rc::Rc;

fn parse_int(input: &str) -> Option<i32> {
    input.parse::<i32>().ok()
}

/// `int.TryParse(input, NumberStyles.HexNumber, ..)`: eight hex digits are the bits of the number.
fn parse_hex(input: &str) -> Option<i32> {
    u32::from_str_radix(input, 16).ok().map(|value| value as i32)
}

#[test]
fn implict_cast_has_value() {
    let person = Person::new("Name", 20);
    let option: Option<Rc<Person>> = person.clone().into();

    assert!(option.is_some());
    assert!(Rc::ptr_eq(&person, option.as_ref().expect("the option has a value")));
}

#[test]
fn option_else_invoked_if_option_has_no_value() {
    let source: Option<Option<Rc<Person>>> = None;

    let ifactioninvoked = Cell::new(false);
    let elseactioninvoked = Cell::new(false);

    source.flatten().if_has_value(|_p| ifactioninvoked.set(true)).else_(|| elseactioninvoked.set(true));

    assert!(!ifactioninvoked.get());
    assert!(elseactioninvoked.get());
}

#[test]
fn option_if_has_value_invoked_if_option_has_value() {
    let source: Option<Rc<Person>> = Person::new("A", 1).into();

    let ifactioninvoked = Cell::new(false);
    let elseactioninvoked = Cell::new(false);

    source.if_has_value(|_p| ifactioninvoked.set(true)).else_(|| elseactioninvoked.set(true));

    assert!(ifactioninvoked.get());
    assert!(!elseactioninvoked.get());
}

#[test]
fn option_none_has_no_value() {
    let option = None::<ChangeSet<Rc<Person>, String>>;
    assert!(option.is_none());
}

#[test]
fn option_set_to_null_has_no_value1() {
    // A null person is `None`; `Optional.Some` of it is still `None`.
    let person: Option<Rc<Person>> = None;
    let option: Option<Rc<Person>> = Some(person).flatten();
    assert!(option.is_none());
}

#[test]
fn option_set_to_null_has_no_value2() {
    // A null person is `None`; its implicit conversion is the same `None`.
    let person: Option<Rc<Person>> = None;
    let option: Option<Rc<Person>> = person;
    assert!(option.is_none());
}

#[test]
fn option_some_has_value() {
    let person = Person::new("Name", 20);
    let option = Some(person.clone());
    assert!(option.is_some());
    assert!(Rc::ptr_eq(&person, option.as_ref().expect("the option has a value")));
}

// Not ported: OptionConvertThrowsIfConverterIsNull - argument validation

#[test]
fn option_convert_to_optional_invokes_converter_with_value() {
    let option = Some(String::new());
    let invoked = Cell::new(false);

    let converter = |input: String| -> Option<String> {
        invoked.set(true);
        Some(input)
    };

    let result = option.convert_optional(converter);

    assert!(invoked.get());
    assert!(result.is_some());
}

#[test]
fn option_convert_to_optional_invokes_converter_only_with_value() {
    let option = None::<String>;
    let invoked = Cell::new(false);

    let converter = |input: String| -> Option<String> {
        invoked.set(true);
        Some(input)
    };

    let result = option.convert_optional(converter);

    assert!(!invoked.get());
    assert!(result.is_none());
}

#[test]
fn option_convert_to_optional_can_return_value() {
    const TEST_DATA: i32 = 37;

    let option = Some(TEST_DATA.to_string());

    let result = option.convert_optional(|input| parse_int(&input));

    assert!(result.is_some());
    assert_eq!(result, Some(TEST_DATA));
}

#[test]
fn option_convert_to_optional_can_return_none() {
    let option = Some("Not An Int".to_string());

    let result = option.convert_optional(|input| parse_int(&input));

    assert!(result.is_none());
}

// Not ported: OptionConvertToOptionalThrowsIfConverterIsNull - argument validation

#[test]
fn option_or_else_invokes_without_value() {
    let option = None::<String>;
    let invoked = Cell::new(false);

    let fallback = || -> Option<String> {
        invoked.set(true);
        None
    };

    let _result = option.or_else(fallback);

    assert!(invoked.get());
}

#[test]
fn option_or_else_invokes_only_without_value() {
    let option = Some(String::new());
    let invoked = Cell::new(false);

    let fallback = || -> Option<String> {
        invoked.set(true);
        None
    };

    let _result = option.or_else(fallback);

    assert!(!invoked.get());
}

#[test]
fn option_or_else_can_return_value() {
    const TEST_STRING: &str = "TestString";

    let option = None::<String>;
    let result = option.or_else(|| Some(TEST_STRING.to_string()));

    assert!(result.is_some());
    assert_eq!(result.as_deref(), Some(TEST_STRING));
}

#[test]
fn option_or_else_can_return_none() {
    let option = None::<String>;
    let result = option.or_else(|| None::<String>);

    assert!(result.is_none());
}

#[test]
fn option_or_else_can_be_chained() {
    const EXPECTED: i32 = 0xc001_d00d_u32 as i32;

    let option = None::<String>;
    let result = option
        .or_else(|| None::<String>)
        .or_else(|| Some(format!("{EXPECTED:x}")))
        .convert_optional(|s| parse_int(&s).or_else(|| parse_hex(&s)));

    assert!(result.is_some());
    assert_eq!(result, Some(EXPECTED));
}

// Not ported: OptionOrElseThrowsIfFallbackIsNull - argument validation
