// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    data::{
        converters::IMultiValueConverter,
        core::{ValueType, ValueTypes},
        BindingError,
    },
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
/// Original reference-identity multi converter.
pub struct ReferenceEqualsMultiConverter;
impl PartialEq for ReferenceEqualsMultiConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ReferenceEqualsMultiConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<ReferenceEqualsMultiConverter>=ReferenceEqualsMultiConverter::new();}
        INSTANCE.with(Rc::clone)
    }
    fn reference_equals(first: &BoxedValue, other: &BoxedValue) -> bool {
        if ValueTypes::reference_equals(first, other) {
            return true;
        }
        if let (Some(a), Some(b)) = (
            crate::model_projection::dockable(Some(first)),
            crate::model_projection::dockable(Some(other)),
        ) {
            return Rc::ptr_eq(&a, &b);
        }
        false
    }
}
impl IMultiValueConverter for ReferenceEqualsMultiConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(first) = values.first() else {
            return Ok(None);
        };
        let Some(first) = first else {
            return Ok(Some(Rc::new(false)));
        };
        for value in &values[1..] {
            if !value
                .as_ref()
                .is_some_and(|value| Self::reference_equals(first, value))
            {
                return Ok(Some(Rc::new(false)));
            }
        }
        Ok(Some(Rc::new(true)))
    }
}
ferroui_base::ferro_markup_type!(class ReferenceEqualsMultiConverter {namespace:"FerroDock.Ferro.Converters",this:Rc<ReferenceEqualsMultiConverter>,handles:[Rc<ReferenceEqualsMultiConverter>,Option<Rc<ReferenceEqualsMultiConverter>>],interfaces:[Rc<dyn IMultiValueConverter>],constructors:[()=>ReferenceEqualsMultiConverter::new],static_properties:[Instance:Rc<ReferenceEqualsMultiConverter>{get:ReferenceEqualsMultiConverter::instance}],});
