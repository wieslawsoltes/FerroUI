//! Not from upstream (#566): the registrations of a value type with the untyped value
//! conversions (its nullable form, its casts and conversions) are made when the type is first
//! used, not from a start-up list that names every type. A type is used when a property of the
//! type is registered, when its markup metadata becomes known, or when code makes it a used type
//! ([`ValueTypes::register_used`]); the tables of types (`type-tables`) register every type at
//! start-up.
//!
//! Each test uses types of its own: the registrations apply to every thread, and the tests run
//! on threads of their own.

use super::*;
use crate::data::core::{ValueType, ValueTypeRegistration, ValueTypes};
use crate::metadata::{MarkupType, MarkupTyped};
use crate::{ferro_markup_enum, ferro_property, FerroObject, FerroProperty, StyledProperty};
use crate::BoxedValue;
use std::rc::Rc;

/// A value type with registrations that nothing uses.
#[derive(Clone, Debug, PartialEq)]
struct NeverUsed(i32);

impl ValueTypeRegistration for NeverUsed {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<NeverUsed>();
    }
}

/// A value type that a test makes a used type, with a cast and a conversion from text.
#[derive(Clone, Debug, PartialEq)]
struct UsedExplicitly(i32);

/// The type `UsedExplicitly` casts to.
#[derive(Clone, Debug, PartialEq)]
struct Wider(i32);

impl ValueTypeRegistration for UsedExplicitly {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<UsedExplicitly>();
        ValueTypes::register_cast::<UsedExplicitly, Wider>(|value| Wider(value.0));
        ValueTypes::register_conversion::<String, UsedExplicitly>(|text| text.trim().parse().ok().map(UsedExplicitly));
    }
}

/// The value type of a property.
#[derive(Clone, Debug, PartialEq)]
struct HeldByProperty(i32);

impl ValueTypeRegistration for HeldByProperty {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<HeldByProperty>();
    }
}

/// A contract a property holds the handle of (`Option<Rc<dyn IHeldContract>>`).
trait IHeldContract {}

impl PartialEq for dyn IHeldContract {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

impl ValueTypeRegistration for dyn IHeldContract {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<Rc<dyn IHeldContract>>();
    }
}

test_class!(FirstUseOwner: FerroObject);

impl FirstUseOwner {
    ferro_property!(pub fn value_property() -> StyledProperty<Option<HeldByProperty>> {
        FerroProperty::register::<FirstUseOwner, _>("Value", None)
    });
    ferro_property!(pub fn contract_property() -> StyledProperty<Option<Rc<dyn IHeldContract>>> {
        FerroProperty::register::<FirstUseOwner, _>("Contract", None)
    });
}

/// An enumeration with metadata, whose metadata a test registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KnownByMetadata {
    One,
    Two,
}

ferro_markup_enum!(KnownByMetadata { One, Two });

impl ValueTypeRegistration for KnownByMetadata {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<KnownByMetadata>();
    }
}

#[test]
fn a_type_that_nothing_uses_is_not_registered() {
    let nullable = ValueType::of::<Option<NeverUsed>>();
    assert!(ValueTypes::nullable_inner(nullable).is_none());
    assert!(!ValueTypes::accepts_null(nullable));
    let value: BoxedValue = Rc::new(NeverUsed(1));
    assert!(ValueTypes::try_cast(&value, nullable).is_none());
}

#[test]
fn the_registrations_of_a_type_are_made_when_it_is_used() {
    let nullable = ValueType::of::<Option<UsedExplicitly>>();
    assert!(ValueTypes::nullable_inner(nullable).is_none());
    assert!(!ValueTypes::is_assignable(ValueType::of::<UsedExplicitly>(), ValueType::of::<Wider>()));

    ValueTypes::register_used::<UsedExplicitly>();

    assert_eq!(ValueTypes::nullable_inner(nullable), Some(ValueType::of::<UsedExplicitly>()));
    let value: BoxedValue = Rc::new(UsedExplicitly(3));
    let wrapped = ValueTypes::try_cast(&value, nullable).expect("the nullable form");
    assert_eq!(wrapped.downcast_ref::<Option<UsedExplicitly>>(), Some(&Some(UsedExplicitly(3))));
    assert!(ValueTypes::is_assignable(ValueType::of::<UsedExplicitly>(), ValueType::of::<Wider>()));
    let wider = ValueTypes::try_cast(&value, ValueType::of::<Wider>()).expect("the cast");
    assert_eq!(wider.downcast_ref::<Wider>(), Some(&Wider(3)));
    let text: BoxedValue = Rc::new(" 7 ".to_string());
    let parsed = ValueTypes::try_convert(Some(&text), ValueType::of::<UsedExplicitly>()).flatten().expect("the conversion");
    assert_eq!(parsed.downcast_ref::<UsedExplicitly>(), Some(&UsedExplicitly(7)));

    // Made once, on every thread: a thread that uses the table later knows the type.
    let elsewhere = std::thread::spawn(|| ValueTypes::nullable_inner(ValueType::of::<Option<UsedExplicitly>>()))
        .join()
        .expect("the other thread");
    assert_eq!(elsewhere, Some(ValueType::of::<UsedExplicitly>()));
    // Using the type again registers nothing more.
    ValueTypes::register_used::<UsedExplicitly>();
    assert_eq!(ValueTypes::nullable_inner(nullable), Some(ValueType::of::<UsedExplicitly>()));
}

#[test]
fn the_first_property_of_a_type_registers_it() {
    let nullable = ValueType::of::<Option<HeldByProperty>>();
    assert!(ValueTypes::nullable_inner(nullable).is_none());
    let property = FirstUseOwner::value_property();
    assert_eq!(ValueTypes::nullable_inner(nullable), Some(ValueType::of::<HeldByProperty>()));

    // A null converts to the nullable form, and a value is wrapped in it.
    let target = FirstUseOwner::new();
    target.set_value_untyped(property, &HeldByProperty(5), crate::data::BindingPriority::LocalValue);
    assert_eq!(target.get_value(property), Some(HeldByProperty(5)));
    assert_eq!(
        ValueTypes::try_convert(None, nullable).flatten().and_then(|value| value.downcast_ref::<Option<HeldByProperty>>().cloned()),
        Some(None)
    );
}

#[test]
fn the_first_property_of_a_contract_handle_registers_the_contract() {
    let nullable = ValueType::of::<Option<Rc<dyn IHeldContract>>>();
    assert!(ValueTypes::nullable_inner(nullable).is_none());
    let _ = FirstUseOwner::contract_property();
    assert_eq!(ValueTypes::nullable_inner(nullable), Some(ValueType::of::<Rc<dyn IHeldContract>>()));
}

#[test]
fn the_registrations_of_a_type_are_made_when_its_metadata_becomes_known() {
    let nullable = ValueType::of::<Option<KnownByMetadata>>();
    assert!(ValueTypes::nullable_inner(nullable).is_none());
    MarkupType::register(<KnownByMetadata as MarkupTyped>::MARKUP);
    assert_eq!(ValueTypes::nullable_inner(nullable), Some(ValueType::of::<KnownByMetadata>()));
}

#[test]
fn the_tables_of_types_register_every_type_at_start_up() {
    crate::register_types();
    // A type of the framework that registers on first use and that no property of this test
    // holds: the tables register it at start-up, and without them it is not registered.
    let registered = ValueTypes::nullable_inner(ValueType::of::<Option<Vec<bool>>>()).is_some();
    assert_eq!(registered, ValueTypes::TYPE_TABLES);
    // What no first use shows stays in the start-up list: registered either way.
    assert_eq!(
        ValueTypes::nullable_inner(ValueType::of::<Option<&'static crate::TypeInfo>>()),
        Some(ValueType::of::<&'static crate::TypeInfo>())
    );
}
