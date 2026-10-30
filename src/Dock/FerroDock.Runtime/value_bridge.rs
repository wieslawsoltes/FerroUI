// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{AnyValue, BoxedValue, FerroObject, Ref};
use std::any::Any;
use std::rc::Rc;

struct NativeValue(BoxedValue);
// A transient borrowed-value view for the shared class-handle registry. It is
// never stored as a property/model value: a successful query immediately yields
// the actual native object handle, retaining its class and virtual dispatch.
struct NativeClassProbe(Rc<dyn Any>);
impl AnyValue for NativeClassProbe {
    fn any_value_as_any(&self) -> &dyn Any {
        self.0.as_ref()
    }
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        match (ValueTypes::as_object(self), ValueTypes::as_object(other)) {
            (Some(a), Some(b)) => a.ptr_eq(&b),
            _ => std::ptr::addr_eq(self.0.as_ref(), other.any_value_as_any()),
        }
    }
    fn any_value_type_name(&self) -> &'static str {
        "native object projection"
    }
}
#[derive(Clone)]
struct ReferenceValue(Rc<dyn Any>);
impl PartialEq for ReferenceValue {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// Carries a native value through the UI-independent object contracts without
/// copying its object or discarding the shared property value's equality metadata.
pub fn to_source_value(value: BoxedValue) -> Rc<dyn Any> {
    if let Some(value) = value.downcast_ref::<ReferenceValue>() {
        return value.0.clone();
    }
    let convert = VALUE_TYPES.with(|types| {
        types
            .borrow()
            .get(&value.as_ref().as_any().type_id())
            .map(|entry| entry.1)
    });
    if let Some(value) = convert.and_then(|convert| convert(&value)) {
        return value;
    }
    Rc::new(NativeValue(value))
}

/// Recovers a native value passed with [`to_source_value`]. Canonical raw
/// control/object handles and scalar values are also accepted. Other raw domain
/// objects retain reference identity; their automatic UI type metadata is pending.
pub fn from_source_value(value: &Rc<dyn Any>) -> BoxedValue {
    if let Some(value) = value.downcast_ref::<NativeValue>() {
        return value.0.clone();
    }
    macro_rules! known {
        ($($t:ty),+ $(,)?) => { $(if let Ok(value) = value.clone().downcast::<$t>() {return value;})+ };
    }
    known!(
        Ref<FerroObject>,
        String,
        bool,
        char,
        i8,
        i16,
        i32,
        i64,
        i128,
        isize,
        u8,
        u16,
        u32,
        u64,
        u128,
        usize,
        f32,
        f64
    );
    let convert = VALUE_TYPES.with(|types| {
        types
            .borrow()
            .get(&value.as_ref().type_id())
            .map(|entry| entry.0)
    });
    if let Some(value) = convert.and_then(|convert| convert(value)) {
        return value;
    }
    if let Some(object) = ValueTypes::as_object(&NativeClassProbe(value.clone())) {
        return Rc::new(object);
    }
    Rc::new(ReferenceValue(value.clone()))
}

// Shared type metadata at the UI-independent object boundary. Entries retain no
// application objects; the two monomorphized functions only recover the original Rc.
type ToNativeValue = fn(&Rc<dyn Any>) -> Option<BoxedValue>;
type ToSourceValue = fn(&BoxedValue) -> Option<Rc<dyn Any>>;
ferroui_base::ferro_thread_local! {
    static VALUE_TYPES: std::cell::RefCell<std::collections::HashMap<std::any::TypeId,(ToNativeValue,ToSourceValue)>> = std::cell::RefCell::new(std::collections::HashMap::new());
}
/// Registers a source value type for lossless native object projection.
/// Registration adds type metadata only; values retain their actual object,
/// equality and Rc identity in both directions, including erased model handles.
pub fn register_value_type<T: PartialEq + 'static>() {
    fn native<T: PartialEq + 'static>(value: &Rc<dyn Any>) -> Option<BoxedValue> {
        value
            .clone()
            .downcast::<T>()
            .ok()
            .map(|value| value as BoxedValue)
    }
    fn source<T: PartialEq + 'static>(value: &BoxedValue) -> Option<Rc<dyn Any>> {
        let value: Rc<dyn Any> = value.clone();
        value.downcast::<T>().ok().map(|value| value as Rc<dyn Any>)
    }
    VALUE_TYPES.with(|types| {
        types
            .borrow_mut()
            .insert(std::any::TypeId::of::<T>(), (native::<T>, source::<T>))
    });
}
