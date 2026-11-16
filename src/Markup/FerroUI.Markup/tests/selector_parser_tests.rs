//! Ported from the upstream `SelectorParserTests`.
//!
//! The control library is a separate crate: the control of the test support
//! stands for `TextBlock`, and the type `Grid` of this file for the owner of
//! the attached property `Grid.Column`.

use super::test_support::*;
use crate::markup::parsers::{SelectorParser, SelectorParserError};
use ferroui_base::*;

struct Auth;

impl StaticType for Auth {
    const TYPE: &'static TypeInfo = {
        static TYPE: TypeInfo = TypeInfo::new("Auth", None);
        &TYPE
    };
}

impl Auth {
    ferro_property!(fn name_property() -> AttachedProperty<String> {
        FerroProperty::register_attached::<Auth, FerroObject, _>("Name", String::new())
    });
}

struct Grid;

impl StaticType for Grid {
    const TYPE: &'static TypeInfo = {
        static TYPE: TypeInfo = TypeInfo::new("Grid", None);
        &TYPE
    };
}

impl Grid {
    ferro_property!(fn column_property() -> AttachedProperty<i32> {
        FerroProperty::register_attached::<Grid, Control, _>("Column", 0)
    });
}

/// Ensures the attached properties are registered before the tests run.
fn setup() {
    let _ = (Grid::column_property(), Auth::name_property());
}

#[test]
fn parses_boolean_property_selector() {
    setup();
    let target = SelectorParser::new(|_ns, _type| Some(Control::TYPE));
    let result = target.parse("TextBlock[IsPointerOver=True]");

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn parses_attacched_property_selector_with_namespace() {
    setup();
    let target = SelectorParser::new(|ns, type_| match (ns, type_) {
        ("", "TextBlock") => Some(Control::TYPE),
        ("l", "Auth") => Some(Auth::TYPE),
        _ => None,
    });
    let result = target.parse("TextBlock[(l|Auth.Name)=Admin]");

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn parses_attacched_property_selector() {
    setup();
    let target = SelectorParser::new(|ns, type_| match (ns, type_) {
        ("", "TextBlock") => Some(Control::TYPE),
        ("", "Grid") => Some(Grid::TYPE),
        _ => None,
    });
    let result = target.parse("TextBlock[(Grid.Column)=1]");

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn parses_comma_separated_selectors() {
    setup();
    let target = SelectorParser::new(|_ns, _type| Some(Control::TYPE));
    let result = target.parse("TextBlock, TextBlock:foo");

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn throws_if_of_type_type_not_found() {
    let target = SelectorParser::new(|_ns, _type| None);

    assert!(matches!(target.parse("NotFound"), Err(SelectorParserError::InvalidOperation(_))));
}

#[test]
fn throws_if_is_type_not_found() {
    let target = SelectorParser::new(|_ns, _type| None);

    assert!(matches!(target.parse(":is(NotFound)"), Err(SelectorParserError::InvalidOperation(_))));
}

#[test]
fn parses_nesting_selectors() {
    setup();
    let target = SelectorParser::new(|_ns, type_| (type_ == "Button").then_some(Control::TYPE));
    for (text, expected) in [("^", "^"), ("^.selected", "^.selected"), ("^:pointerover", "^:pointerover")] {
        let result = target.parse(text);
        let selector = result.unwrap_or_else(|error| panic!("{text}: {error}")).expect("a selector");
        assert_eq!(selector.to_string(), expected);
    }
    let child = target.parse("^:pointerover > Button").expect("parses").expect("a selector");
    assert!(child.to_string().starts_with("^:pointerover"), "{child}");
}

// Not from the upstream tests: the parser of an application that keeps no tables of types.
#[test]
fn a_type_the_resolver_does_not_know_is_an_error_unless_it_may_be_matched_by_name() {
    let strict = SelectorParser::new(|_ns, _type| None);
    assert!(matches!(strict.parse("TextBlock.accent"), Err(SelectorParserError::InvalidOperation(_))));

    let by_name = SelectorParser::new(|_ns, _type| None).unknown_types_by_name();
    let exact = by_name.parse("TextBlock.accent").expect("parses").expect("a selector");
    assert_eq!(exact.to_string(), "TextBlock.accent");
    let derived = by_name.parse(":is(TextBlock) > Border").expect("parses").expect("a selector");
    assert_eq!(derived.to_string(), ":is(TextBlock) > Border");
    // A known type is still the type itself.
    let known = SelectorParser::new(|_ns, type_| (type_ == "Known").then_some(Control::TYPE)).unknown_types_by_name();
    let selector = known.parse("Known").expect("parses").expect("a selector");
    assert_eq!(selector.target_type(), Some(Control::TYPE));
    // The selector of a property needs its type, to find the property: a class that is known
    // by then has it, one that nothing knows does not.
    assert!(by_name.parse("NoClassOfThisNameIsKnown[IsPointerOver=True]").is_err());
}
