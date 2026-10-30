// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::{core::IDockable, BoxedValue as SourceValue};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    AnyValue, BoxedValue, PropertyValue,
};
use std::{
    any::{Any, TypeId},
    rc::Rc,
};
// A borrowed-type probe for the shared cast table, never an application value.
// A successful cast immediately returns the actual source class reference.
pub(super) struct ModelProbe(pub(super) SourceValue);
impl AnyValue for ModelProbe {
    fn any_value_as_any(&self) -> &dyn Any {
        self.0.as_ref()
    }
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        std::ptr::addr_eq(self.0.as_ref(), other.any_value_as_any())
    }
    fn any_value_type_name(&self) -> &'static str {
        "source model projection"
    }
}
pub(crate) fn project<T: PropertyValue>(value: &BoxedValue) -> Option<T> {
    let original = value
        .downcast_ref::<Rc<dyn IDockable>>()?
        .clone()
        .boxed_reference();
    if original.as_ref().type_id() == TypeId::of::<Rc<dyn IDockable>>() {
        return None;
    }
    let probe: BoxedValue = Rc::new(ModelProbe(original));
    ValueTypes::try_cast(&probe, ValueType::of::<T>())?
        .downcast_ref::<T>()
        .cloned()
}

pub(crate) fn source_name(value: &Rc<dyn IDockable>) -> Option<String> {
    let original = value.clone().boxed_reference();
    if original.as_ref().type_id() == TypeId::of::<Rc<dyn IDockable>>() {
        return None;
    }
    let probe: BoxedValue = Rc::new(ModelProbe(original));
    ValueTypes::try_convert(Some(&probe), ValueType::of::<String>())??
        .downcast_ref::<String>()
        .cloned()
}
