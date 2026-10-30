// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use std::{cell::Cell, rc::Rc};
/// Original integer threshold converter.
pub struct IntLessThanConverter {
    true_if_less_than: Cell<i32>,
}
impl PartialEq for IntLessThanConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl IntLessThanConverter {
    /// Creates a converter with the source zero threshold.
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            true_if_less_than: Cell::new(0),
        })
    }
    /// Gets TrueIfLessThan.
    pub fn true_if_less_than(&self) -> i32 {
        self.true_if_less_than.get()
    }
    /// Sets TrueIfLessThan.
    pub fn set_true_if_less_than(&self, value: i32) {
        self.true_if_less_than.set(value);
    }
}
impl IValueConverter for IntLessThanConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(Rc::new(
            value
                .and_then(|v| v.downcast_ref::<i32>())
                .is_some_and(|v| *v < self.true_if_less_than()),
        )))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message(
            "The method or operation is not implemented.",
        ))
    }
}
ferroui_base::ferro_markup_type!(class IntLessThanConverter{namespace:"FerroDock.Ferro.Converters",this:Rc<IntLessThanConverter>,handles:[Rc<IntLessThanConverter>,Option<Rc<IntLessThanConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>IntLessThanConverter::new],properties:[TrueIfLessThan:i32{get:|this:&Rc<IntLessThanConverter>|this.true_if_less_than(),set:|this:&Rc<IntLessThanConverter>,value|this.set_true_if_less_than(value)}],});
