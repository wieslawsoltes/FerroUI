use super::*;
use crate::data::core::{ClrPropertyInfo, Value};
use crate::{FerroObject, FerroObjectImpl};
use std::cell::RefCell;

struct Source {
    text: RefCell<String>,
}

impl PartialEq for Source {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn text_property() -> Rc<dyn IPropertyInfo> {
    Rc::new(ClrPropertyInfo::read_write::<Source, Value<String>>(
        "Text",
        |s| s.text.borrow().clone(),
        |s, v| *s.text.borrow_mut() = v,
    ))
}

fn source() -> (Rc<Source>, BoxedValue) {
    ValueTypes::register_reference::<Source>();
    let source = Rc::new(Source {
        text: RefCell::new("a".to_string()),
    });
    let boxed: BoxedValue = source.clone();
    (source, boxed)
}

#[test]
fn set_value_converts_before_it_writes() {
    let (source, boxed) = source();
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), text_property(), None);

    // A value of another type is converted to the type of the property.
    let number: BoxedValue = Rc::new(5i32);
    assert_eq!(accessor.set_value(Some(&number), BindingPriority::LocalValue), Ok(true));
    assert_eq!(*source.text.borrow(), "5");

    // A value that cannot be converted is an error, and nothing is written.
    let accessor = InpcPropertyAccessor::new(
        WeakValue::new(&boxed),
        Rc::new(ClrPropertyInfo::read_write::<Source, Value<i32>>(
            "Length",
            |s| s.text.borrow().len() as i32,
            |_, _| {},
        )),
        None,
    );
    let text: BoxedValue = Rc::new("not a number".to_string());
    let error = accessor
        .set_value(Some(&text), BindingPriority::LocalValue)
        .unwrap_err();
    assert!(error.to_string().contains("cannot be converted to type"), "{error}");
}

#[test]
fn set_value_on_a_source_that_no_longer_exists_is_an_error() {
    let (source, boxed) = source();
    let reference = WeakValue::new(&boxed);
    let accessor = InpcPropertyAccessor::new(reference, text_property(), None);
    drop(boxed);
    drop(source);

    assert!(accessor.value().is_none());
    let value: BoxedValue = Rc::new("b".to_string());
    let error = accessor
        .set_value(Some(&value), BindingPriority::LocalValue)
        .unwrap_err();
    assert_eq!(error.to_string(), "Non-static method requires a target.");
    // The conversion comes first: its error wins over the missing target.
    let read_only = InpcPropertyAccessor::new(
        WeakValue::Value(Rc::new(1i32)),
        Rc::new(ClrPropertyInfo::read_only::<Source, Value<String>>("Text", |s| {
            s.text.borrow().clone()
        })),
        None,
    );
    assert_eq!(
        read_only.set_value(Some(&value), BindingPriority::LocalValue),
        Ok(false)
    );
}

#[repr(C)]
struct NativeSource {
    base: crate::FerroObject,
    changed: ferroui_system::events::Event<str>,
}
crate::ferro_class!(NativeSource:FerroObject);
crate::ferro_impl_classes!(NativeSource:FerroObjectImpl);
crate::ferro_properties! {impl NativeSource {
    fn number_property()->crate::StyledProperty<i32>{crate::FerroProperty::register::<NativeSource,_>("Number",1)}
    fn other_property()->crate::StyledProperty<i32>{crate::FerroProperty::register::<NativeSource,_>("Other",0)}
}}
fn native_source() -> (crate::Ref<NativeSource>, BoxedValue) {
    let source = crate::instantiate(NativeSource {
        base: crate::FerroObject::construct(),
        changed: ferroui_system::events::Event::new(),
    });
    let boxed: BoxedValue = Rc::new(source.clone());
    (source, boxed)
}
fn native_number() -> Rc<dyn IPropertyInfo> {
    Rc::new(ClrPropertyInfo::read_write::<crate::Ref<NativeSource>, Value<i32>>(
        "Number",
        |s| s.get_value(NativeSource::number_property()) + 100,
        |s, v| s.set_value(NativeSource::number_property(), v - 100),
    ))
}
#[test]
fn native_backed_plain_property_follows_changes_without_bypassing_its_accessors() {
    let (source, boxed) = native_source();
    let baseline = source.property_changed_subscriber_count();
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), native_number(), None);
    let values = Rc::new(RefCell::new(Vec::new()));
    let collected = values.clone();
    accessor.subscribe(Rc::new(move |v| {
        collected.borrow_mut().push(*v.unwrap().downcast_ref::<i32>().unwrap())
    }));
    assert_eq!(&*values.borrow(), &[101]);
    source.set_value(NativeSource::other_property(), 12);
    source.set_value(NativeSource::number_property(), 2);
    assert_eq!(&*values.borrow(), &[101, 102]);
    let input: BoxedValue = Rc::new(109i32);
    accessor.set_value(Some(&input), BindingPriority::LocalValue).unwrap();
    assert_eq!(source.get_value(NativeSource::number_property()), 9);
    assert_eq!(&*values.borrow(), &[101, 102, 109]);
    accessor.unsubscribe();
    assert_eq!(source.property_changed_subscriber_count(), baseline);
    source.set_value(NativeSource::number_property(), 3);
    assert_eq!(values.borrow().len(), 3);
    accessor.subscribe(Rc::new(|_| {}));
    accessor.dispose();
    assert_eq!(source.property_changed_subscriber_count(), baseline);
}
#[test]
fn native_backed_plain_accessor_releases_subscription_on_drop_and_keeps_source_weak() {
    let (source, boxed) = native_source();
    let baseline = source.property_changed_subscriber_count();
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), native_number(), None);
    accessor.subscribe(Rc::new(|_| {}));
    drop(accessor);
    assert_eq!(source.property_changed_subscriber_count(), baseline);
    let weak = source.downgrade();
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), native_number(), None);
    accessor.subscribe(Rc::new(|_| {}));
    drop(boxed);
    drop(source);
    assert!(weak.upgrade().is_none());
    assert!(accessor.value().is_none());
    accessor.unsubscribe();
}
impl INotifyPropertyChanged for NativeSource {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        &self.changed
    }
}
#[test]
fn native_backed_plain_accessor_does_not_duplicate_explicit_inpc_notifications() {
    let (source, boxed) = native_source();
    let baseline = source.property_changed_subscriber_count();
    let lookup: InpcLookup = Rc::new(|v| {
        v.downcast_ref::<crate::Ref<NativeSource>>()
            .map(|s| &**s as &dyn INotifyPropertyChanged)
    });
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), native_number(), Some(lookup));
    let values = Rc::new(RefCell::new(Vec::new()));
    let collected = values.clone();
    accessor.subscribe(Rc::new(move |v| {
        collected.borrow_mut().push(*v.unwrap().downcast_ref::<i32>().unwrap())
    }));
    assert_eq!(source.property_changed_subscriber_count(), baseline);
    source.set_value(NativeSource::number_property(), 2);
    assert_eq!(&*values.borrow(), &[101]);
    source.changed.raise("Other");
    source.changed.raise("Number");
    assert_eq!(&*values.borrow(), &[101, 102]);
    accessor.unsubscribe();
    assert_eq!(source.changed.handler_count(), 0);
}
#[test]
fn native_backed_plain_accessor_does_not_subscribe_unmatched_clr_members() {
    let (source, boxed) = native_source();
    let baseline = source.property_changed_subscriber_count();
    let info = Rc::new(ClrPropertyInfo::read_only::<crate::Ref<NativeSource>, Value<i32>>(
        "Computed",
        |s| s.get_value(NativeSource::number_property()) + 100,
    ));
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), info, None);
    let values = Rc::new(RefCell::new(Vec::new()));
    let collected = values.clone();
    accessor.subscribe(Rc::new(move |v| {
        collected.borrow_mut().push(*v.unwrap().downcast_ref::<i32>().unwrap())
    }));
    assert_eq!(source.property_changed_subscriber_count(), baseline);
    source.set_value(NativeSource::number_property(), 2);
    assert_eq!(&*values.borrow(), &[101]);
    accessor.unsubscribe();
}
#[test]
fn native_backed_plain_accessor_reentrant_write_publishes_each_change_once() {
    let (source, boxed) = native_source();
    let accessor = InpcPropertyAccessor::new(WeakValue::new(&boxed), native_number(), None);
    let values = Rc::new(RefCell::new(Vec::new()));
    let collected = values.clone();
    let weak = source.downgrade();
    accessor.subscribe(Rc::new(move |v| {
        let value = *v.unwrap().downcast_ref::<i32>().unwrap();
        collected.borrow_mut().push(value);
        if value == 102 {
            weak.upgrade().unwrap().set_value(NativeSource::number_property(), 3)
        }
    }));
    let input: BoxedValue = Rc::new(102i32);
    accessor.set_value(Some(&input), BindingPriority::LocalValue).unwrap();
    assert_eq!(&*values.borrow(), &[101, 102, 103]);
    assert_eq!(source.get_value(NativeSource::number_property()), 3);
    accessor.unsubscribe();
}
