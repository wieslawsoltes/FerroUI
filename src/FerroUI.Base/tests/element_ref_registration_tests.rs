//! Public conversion behavior of registered weak element references.

use crate::data::core::{ValueType, ValueTypes};
use crate::{BoxedValue, ElementRef, FerroObject, ObjectType, PropertyValue, Ref};
use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

macro_rules! class {
    ($name:ident: $base:ident) => {
        #[repr(C)]
        pub struct $name { base: $base }
        crate::ferro_class!($name: $base);
        crate::ferro_impl_classes!($name: crate::FerroObjectImpl);
        impl $name {
            fn construct() -> Self { Self { base: $base::construct() } }
            fn new() -> Ref<Self> { crate::instantiate(Self::construct()) }
        }
    };
}

class!(Target: FerroObject);
class!(Derived: Target);
class!(Other: FerroObject);

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue { Rc::new(value) }

fn typed<T: PropertyValue>(value: Option<BoxedValue>) -> T {
    value.expect("conversion succeeds").downcast_ref::<T>().expect("exact destination type").clone()
}

fn converted<T: PropertyValue>(value: &BoxedValue) -> T {
    typed(ValueTypes::try_convert_registered(value, ValueType::of::<T>()))
}

fn borrowed<T: PropertyValue>(value: &dyn std::any::Any) -> T {
    typed(ValueTypes::try_convert_for_property(value, TypeId::of::<T>()))
}

fn assert_points_to<T: ObjectType>(element: ElementRef<T>, expected: &Ref<T>) {
    assert!(element.points_to(expected));
    assert!(element.get().expect("live element").ptr_eq(expected));
}

#[test]
fn live_handles_keep_identity_in_boxed_borrowed_and_nullable_conversions() {
    ValueTypes::register_element_ref::<Target>();
    let target = Target::new();
    let source = boxed(target.clone());

    assert_points_to(converted::<ElementRef<Target>>(&source), &target);
    assert_points_to(borrowed::<ElementRef<Target>>(&target), &target);
    assert_points_to(typed(ValueTypes::try_cast(&source, ValueType::of::<ElementRef<Target>>())), &target);
    assert_points_to(converted::<Option<ElementRef<Target>>>(&source).unwrap(), &target);
    assert_points_to(borrowed::<Option<ElementRef<Target>>>(&target).unwrap(), &target);
    assert_points_to(
        converted::<Option<ElementRef<Target>>>(&boxed(Some(target.clone()))).unwrap(),
        &target,
    );

    let element = boxed(ElementRef::of(&target));
    assert!(converted::<Ref<Target>>(&element).ptr_eq(&target));
    assert!(converted::<Option<Ref<Target>>>(&element).unwrap().ptr_eq(&target));
    assert!(borrowed::<Option<Ref<Target>>>(element.as_any()).unwrap().ptr_eq(&target));
    assert!(ValueTypes::as_object(&*element).unwrap().ptr_eq(&target));
}

#[test]
fn expired_elements_fail_strong_conversion_but_produce_nullable_null() {
    ValueTypes::register_element_ref::<Target>();
    let (element, optional) = {
        let target = Target::new();
        let element = ElementRef::of(&target);
        (boxed(element.clone()), boxed(Some(element)))
    };

    assert!(ValueTypes::as_object(&*element).is_none());
    assert!(ValueTypes::try_convert_registered(&element, ValueType::of::<Ref<Target>>()).is_none());
    assert!(converted::<Option<Ref<Target>>>(&element).is_none());
    assert!(converted::<Option<Ref<Target>>>(&optional).is_none());
    assert!(borrowed::<Option<Ref<Target>>>(element.as_any()).is_none());
    assert!(borrowed::<Option<Ref<Target>>>(optional.as_any()).is_none());
    assert!(typed::<Option<Ref<Target>>>(
        ValueTypes::try_cast(&element, ValueType::of::<Option<Ref<Target>>>()),
    ).is_none());
}

#[test]
fn nullable_sources_and_null_values_preserve_exact_destination_types() {
    ValueTypes::register_element_ref::<Target>();
    let no_handle = None::<Ref<Target>>;
    let no_element = None::<ElementRef<Target>>;

    assert!(converted::<Option<ElementRef<Target>>>(&boxed(no_handle.clone())).is_none());
    assert!(borrowed::<Option<ElementRef<Target>>>(&no_handle).is_none());
    assert!(converted::<Option<Ref<Target>>>(&boxed(no_element.clone())).is_none());
    assert!(borrowed::<Option<Ref<Target>>>(&no_element).is_none());
    assert!(typed::<Option<ElementRef<Target>>>(
        ValueTypes::null_value(ValueType::of::<Option<ElementRef<Target>>>()),
    ).is_none());
    assert!(ValueTypes::try_convert(None, ValueType::of::<ElementRef<Target>>()).is_none());
    assert!(typed::<Option<ElementRef<Target>>>(
        ValueTypes::try_convert(None, ValueType::of::<Option<ElementRef<Target>>>()).unwrap(),
    ).is_none());
}

#[test]
fn root_conversion_checks_runtime_subclass_before_constructing_element_reference() {
    ValueTypes::register_element_ref::<Target>();
    let derived = Derived::new();
    let target = derived.cast::<Target>().unwrap();
    let valid = boxed(derived.upcast::<FerroObject>());
    let invalid = boxed(Other::new().upcast::<FerroObject>());

    assert_points_to(converted::<ElementRef<Target>>(&valid), &target);
    assert_points_to(converted::<Option<ElementRef<Target>>>(&valid).unwrap(), &target);
    assert!(ValueTypes::try_convert_registered(&invalid, ValueType::of::<ElementRef<Target>>()).is_none());
    assert!(ValueTypes::try_convert_registered(&invalid, ValueType::of::<Option<ElementRef<Target>>>()).is_none());
    // A checked downcast from the root is a conversion, not an implicit
    // assignment to a narrower property type.
    assert!(ValueTypes::try_convert_for_property(valid.as_any(), TypeId::of::<ElementRef<Target>>()).is_none());
}

#[test]
fn root_element_registration_keeps_overlapping_cast_and_borrowed_routes() {
    ValueTypes::register_element_ref::<FerroObject>();
    let target = Target::new().upcast::<FerroObject>();
    let source = boxed(target.clone());

    for _ in 0..3 {
        ValueTypes::register_element_ref::<FerroObject>();
        assert_points_to(converted::<ElementRef<FerroObject>>(&source), &target);
        assert_points_to(borrowed::<ElementRef<FerroObject>>(&target), &target);
        assert_points_to(typed(ValueTypes::try_cast(&source, ValueType::of::<ElementRef<FerroObject>>())), &target);
        assert_points_to(converted::<Option<ElementRef<FerroObject>>>(&source).unwrap(), &target);
        assert_points_to(borrowed::<Option<ElementRef<FerroObject>>>(&target).unwrap(), &target);
    }
}

#[test]
fn captured_conversion_replacement_preserves_existing_assignability_and_borrowed_casts() {
    ValueTypes::register_element_ref::<Target>();
    let first = Target::new();
    let second = Target::new();
    let source = boxed(first.clone());
    let enabled = Rc::new(Cell::new(false));
    let captured = enabled.clone();
    let replacement = ElementRef::of(&second);
    ValueTypes::register_conversion::<Ref<Target>, ElementRef<Target>>(move |_| {
        captured.get().then(|| replacement.clone())
    });

    assert!(ValueTypes::try_convert_registered(&source, ValueType::of::<ElementRef<Target>>()).is_none());
    enabled.set(true);
    assert_points_to(converted::<ElementRef<Target>>(&source), &second);
    assert_points_to(typed(ValueTypes::try_cast(&source, ValueType::of::<ElementRef<Target>>())), &first);
    assert_points_to(borrowed::<ElementRef<Target>>(&first), &first);

    ValueTypes::register_element_ref::<Target>();
    assert_points_to(converted::<ElementRef<Target>>(&source), &first);
    assert_points_to(borrowed::<ElementRef<Target>>(&first), &first);
}

#[test]
fn root_nullable_override_and_reregistration_keep_conversion_views_independent() {
    ValueTypes::register_element_ref::<FerroObject>();
    let first = Target::new().upcast::<FerroObject>();
    let second = Other::new().upcast::<FerroObject>();
    let source = boxed(first.clone());
    let enabled = Rc::new(Cell::new(false));
    let captured = enabled.clone();
    let replacement = ElementRef::of(&second);
    ValueTypes::register_boxed_conversion::<Ref<FerroObject>, Option<ElementRef<FerroObject>>>(move |_| {
        Some(captured.get().then(|| replacement.clone()))
    });

    assert!(converted::<Option<ElementRef<FerroObject>>>(&source).is_none());
    enabled.set(true);
    assert_points_to(converted::<Option<ElementRef<FerroObject>>>(&source).unwrap(), &second);
    assert_points_to(typed::<Option<ElementRef<FerroObject>>>(
        ValueTypes::try_cast(&source, ValueType::of::<Option<ElementRef<FerroObject>>>()),
    ).unwrap(), &first);
    assert_points_to(borrowed::<Option<ElementRef<FerroObject>>>(&first).unwrap(), &first);

    ValueTypes::register_element_ref::<FerroObject>();
    assert_points_to(converted::<Option<ElementRef<FerroObject>>>(&source).unwrap(), &first);
    assert_points_to(borrowed::<Option<ElementRef<FerroObject>>>(&first).unwrap(), &first);
}
