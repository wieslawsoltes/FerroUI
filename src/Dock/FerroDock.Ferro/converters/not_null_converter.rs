// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
/// Original converter testing object nullness with optional inversion.
pub struct NotNullConverter;
impl PartialEq for NotNullConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl NotNullConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared converter in the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<NotNullConverter>=NotNullConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for NotNullConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let invert = parameter.is_some_and(|p| {
            p.downcast_ref::<bool>().copied() == Some(true)
                || p.downcast_ref::<String>()
                    .is_some_and(|s| s.eq_ignore_ascii_case("Not"))
        });
        Ok(Some(Rc::new(value.is_some() != invert)))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message(
            "The method or operation is not implemented.",
        ))
    }
}
ferroui_base::ferro_markup_type!(class NotNullConverter {
    namespace:"FerroDock.Ferro.Converters",
    this:Rc<NotNullConverter>,handles:[Rc<NotNullConverter>,Option<Rc<NotNullConverter>>],
    interfaces:[Rc<dyn IValueConverter>],
    constructors:[()=>NotNullConverter::new],
    fields:[Instance:Rc<NotNullConverter> => NotNullConverter::instance],
});
