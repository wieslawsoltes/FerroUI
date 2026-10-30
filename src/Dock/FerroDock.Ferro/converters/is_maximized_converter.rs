// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use ferroui_controls::WindowState;
use std::rc::Rc;
/// Original IsMaximizedConverter.
pub struct IsMaximizedConverter;
impl PartialEq for IsMaximizedConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl IsMaximizedConverter {
    /// Creates the source stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared source converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<IsMaximizedConverter>=IsMaximizedConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for IsMaximizedConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(Rc::new(
            value
                .and_then(|v| v.downcast_ref::<WindowState>())
                .is_some_and(|v| *v == WindowState::Maximized),
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
ferroui_base::ferro_markup_type!(class IsMaximizedConverter {namespace:"FerroDock.Ferro.Converters",this:Rc<IsMaximizedConverter>,handles:[Rc<IsMaximizedConverter>,Option<Rc<IsMaximizedConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>IsMaximizedConverter::new],static_properties:[Instance:Rc<IsMaximizedConverter>{get:IsMaximizedConverter::instance}],});
