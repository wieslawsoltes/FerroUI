//! Port of the upstream property base-class tests.
//!
//! Upstream derives a test property and test metadata from the abstract base
//! classes. Properties are not subclassable here, so a styled property that is
//! created without being registered stands in for the test property, and
//! styled property metadata (whose coercion callback is its owner-specific
//! part) for the test metadata.

use super::*;
use crate::data::{BindingMode, BindingPriority};
use crate::ferro_property_metadata::PropertyMetadata;
use crate::*;
use std::any::TypeId;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct Class1 {
    base: FerroObject,
    notify_count: Cell<i32>,
}

ferro_class!(Class1: FerroObject);
ferro_impl_classes!(Class1: FerroObjectImpl);

impl Class1 {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register_with::<Class1, _>(
            "Foo",
            StyledPropertyOptions::new(s("default"))
                .inherits(true)
                .default_binding_mode(BindingMode::OneWay)
                .enable_data_validation(false)
                .notifying(Class1::foo_notifying),
        )
    });

    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), notify_count: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn foo_notifying(o: &FerroObject, _notifying: bool) {
        let o = o.downcast_ref::<Class1>().expect("a Class1");
        o.notify_count.set(o.notify_count.get() + 1);
    }
}

test_class!(Class2: Class1);
test_class!(Class3: FerroObject);

fn test_metadata() -> StyledPropertyMetadata<String> {
    StyledPropertyMetadata::new(Some(String::new()))
}

fn test_property(
    name: &str,
    owner_type: &'static TypeInfo,
    metadata: StyledPropertyMetadata<String>,
) -> &'static StyledProperty<String> {
    StyledProperty::create(name, owner_type, owner_type, metadata, false, None, None, false)
}

#[test]
fn constructor_sets_properties() {
    let target = test_property("test", Class1::TYPE, test_metadata());

    assert_eq!("test", target.name());
    assert_eq!(TypeId::of::<String>(), target.property_type());
    assert_eq!(Class1::TYPE, target.owner_type());
}

#[test]
fn name_cannot_contain_periods() {
    assert_panics(|| {
        test_property("Foo.Bar", Class1::TYPE, test_metadata());
    });
}

#[test]
fn get_metadata_returns_supplied_value() {
    let metadata = test_metadata().with_default_binding_mode(BindingMode::TwoWay).with_coerce(|_, v| v);
    let target = test_property("test", Class1::TYPE, metadata);

    let result = target.get_metadata(Class1::TYPE);
    assert_eq!(BindingMode::TwoWay, result.default_binding_mode());
    assert!(result.coerce_value().is_some());
    assert!(Rc::ptr_eq(&result, &target.get_metadata(Class1::TYPE)));
}

#[test]
fn get_metadata_returns_supplied_value_for_derived_class() {
    let metadata = test_metadata().with_coerce(|_, v| v);
    let target = test_property("test", Class1::TYPE, metadata);

    assert!(Rc::ptr_eq(&target.get_metadata(Class1::TYPE), &target.get_metadata(Class2::TYPE)));
}

#[test]
fn get_metadata_returns_type_safe_metadata_for_unrelated_class() {
    let metadata = test_metadata()
        .with_default_binding_mode(BindingMode::OneWayToSource)
        .with_enable_data_validation(true)
        .with_coerce(|o, v| {
            assert!(o.is::<Class3>());
            v
        });
    let target = test_property("test", Class3::TYPE, metadata);

    let owner_metadata = target.get_metadata(Class3::TYPE);
    let target_metadata = target.get_metadata(Class2::TYPE);

    assert_eq!(owner_metadata.default_binding_mode(), target_metadata.default_binding_mode());
    assert_eq!(owner_metadata.enable_data_validation(), target_metadata.enable_data_validation());
    assert!(owner_metadata.coerce_value().is_some());
    assert!(target_metadata.coerce_value().is_none());
}

#[test]
fn get_metadata_returns_overridden_value() {
    let target = test_property("test", Class1::TYPE, test_metadata());

    target.override_metadata::<Class2>(StyledPropertyMetadata::new(Some(s("overridden"))));

    assert_eq!("overridden", target.get_metadata(Class2::TYPE).default_value());
    assert_eq!("", target.get_metadata(Class1::TYPE).default_value());
    assert!(!Rc::ptr_eq(&target.get_metadata(Class1::TYPE), &target.get_metadata(Class2::TYPE)));
}

#[test]
fn override_metadata_should_merge_values() {
    let metadata = test_metadata().with_default_binding_mode(BindingMode::TwoWay);
    let target = test_property("test", Class1::TYPE, metadata);

    target.override_metadata::<Class2>(StyledPropertyMetadata::new(None));

    let result = target.get_metadata(Class2::TYPE);
    assert_eq!(BindingMode::TwoWay, result.default_binding_mode());
}

#[test]
fn default_metadata_cannot_be_changed_after_property_initialization() {
    let property = test_property("test", Class1::TYPE, test_metadata());

    let mut metadata = (*property.get_metadata(Class1::TYPE)).clone();
    assert!(metadata.is_read_only());
    assert_panics(|| PropertyMetadata::merge(&mut metadata, &test_metadata()));
}

#[test]
fn overridden_metadata_cannot_be_changed_after_override_metadata() {
    let metadata = test_metadata().with_default_binding_mode(BindingMode::TwoWay);
    let property = test_property("test", Class1::TYPE, metadata);

    property.override_metadata::<Class2>(StyledPropertyMetadata::new(None));

    let mut overridden = (*property.get_metadata(Class2::TYPE)).clone();
    assert!(overridden.is_read_only());
    assert_panics(|| PropertyMetadata::merge(&mut overridden, &test_metadata()));
}

#[test]
fn changed_observable_fired() {
    let target = Class1::new();
    let value: Recorder<String> = Recorder::new();

    let v = value.clone();
    Class1::foo_property().changed().subscribe(move |x| v.push(x.get_new_value::<String>()));
    target.set_value(Class1::foo_property(), s("newvalue"));

    assert_eq!(vec![s("newvalue")], value.get());
}

#[test]
fn changed_observable_fired_only_on_effective_value_change() {
    let target = Class1::new();
    let result: Recorder<String> = Recorder::new();

    let r = result.clone();
    Class1::foo_property().changed().subscribe(move |x| r.push(x.get_new_value::<String>()));
    target.set_value_with_priority(Class1::foo_property(), s("animated"), BindingPriority::Animation);
    target.set_value(Class1::foo_property(), s("local"));

    assert_eq!(vec![s("animated")], result.get());
}

#[test]
fn notify_fired_only_on_effective_value_change() {
    let target = Class1::new();

    target.set_value_with_priority(Class1::foo_property(), s("animated"), BindingPriority::Animation);
    target.set_value(Class1::foo_property(), s("local"));

    assert_eq!(2, target.notify_count.get());
}

#[test]
fn property_metadata_binding_mode_default_returns_one_way() {
    let data = FerroPropertyMetadata::new(BindingMode::Default, None);

    assert_eq!(BindingMode::OneWay, data.default_binding_mode());
}

// A boxed value is itself a `PartialEq` type, so the blanket `AnyValue`
// implementation also covers the box. With the trait in scope, the untyped
// accessors must still describe the contents of the box, not the box.
#[test]
fn boxed_value_accessors_describe_the_contents_with_the_trait_in_scope() {
    #[allow(unused_imports)]
    use crate::AnyValue;

    let value: crate::BoxedValue = std::rc::Rc::new(5i32);
    let same: crate::BoxedValue = std::rc::Rc::new(5i32);
    let reference = &value;

    assert!(value.as_any().is::<i32>());
    assert!(reference.as_any().is::<i32>());
    assert_eq!(value.type_name(), "i32");
    assert_eq!(reference.type_name(), "i32");
    assert_eq!(value.value_type_id(), std::any::TypeId::of::<i32>());
    assert!(value.value_eq(&*same));
    assert!(*value == *same);
}
