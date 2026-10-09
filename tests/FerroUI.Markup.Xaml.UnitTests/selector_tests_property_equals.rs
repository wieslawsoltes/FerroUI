//! Port of the two tests of `Styling/SelectorTests_PropertyEquals.cs` (base unit tests) that parse their selector:
//! they need the selector parser of the markup crate, a text block and the attached column property of the grid,
//! which this crate has together. The other tests of the file are in `styling/selector_tests.rs` of
//! `ferroui-base`.
//!
//! Upstream takes one value of the activator as an observable after every change (`Take(1)`); the activator is
//! asked for its state here, which reads its inputs.

use ferroui_base::{ferro_property, AttachedProperty, FerroObject, FerroProperty, StaticType, TypeInfo};
use ferroui_controls::{Grid, TextBlock};
use ferroui_markup::markup::parsers::SelectorParser;

struct Auth;

impl StaticType for Auth {
    const TYPE: &'static TypeInfo = {
        static TYPE: TypeInfo = TypeInfo::new("Auth", None);
        &TYPE
    };
}

impl Auth {
    ferro_property!(fn name_property() -> AttachedProperty<Option<String>> {
        FerroProperty::register_attached::<Auth, FerroObject, _>("Name", None)
    });

    fn set_name(object: &FerroObject, value: Option<String>) {
        object.set_value(Self::name_property(), value)
    }
}

#[test]
fn property_equals_attached_property_matching_value() {
    let target = SelectorParser::new(|ns, type_| match (ns, type_) {
        ("", "TextBlock") => Some(TextBlock::TYPE),
        ("", "Grid") => Some(Grid::TYPE),
        _ => None,
    })
    .parse("TextBlock[(Grid.Column)=1]")
    .expect("the selector parses");

    let target = target.expect("a selector");

    let control = TextBlock::new();
    let match_ = target.match_(&control, None, true);
    let activator = match_.activator().expect("an activator");

    assert!(!activator.get_is_active());
    Grid::set_column(&control, 1);
    assert!(activator.get_is_active());
    Grid::set_column(&control, 0);
    assert!(!activator.get_is_active());
}

#[test]
fn property_equals_attached_property_with_namespace_matching_value() {
    // Upstream runs the class constructor of `Auth` first: the property has to be registered to be found by name.
    let _ = Auth::name_property();

    let target = SelectorParser::new(|ns, type_| match (ns, type_) {
        ("", "TextBlock") => Some(TextBlock::TYPE),
        ("l", "Auth") => Some(Auth::TYPE),
        _ => None,
    })
    .parse("TextBlock[(l|Auth.Name)=Admin]")
    .expect("the selector parses");

    let target = target.expect("a selector");

    let control = TextBlock::new();
    let match_ = target.match_(&control, None, true);
    let activator = match_.activator().expect("an activator");

    assert!(!activator.get_is_active());
    Auth::set_name(&control, Some("Admin".to_string()));
    assert!(activator.get_is_active());
    Auth::set_name(&control, None);
    assert!(!activator.get_is_active());
}
