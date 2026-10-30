// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::Orientation as ModelType;
use ferroui_base::layout::Orientation as NativeType;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue, FerroProperty,
};
use std::rc::Rc;
/// Original OrientationConverter preserving unrelated input object identity.
pub struct OrientationConverter;
impl PartialEq for OrientationConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl OrientationConverter {
    /// Creates the source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the source shared instance on the UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<OrientationConverter>=OrientationConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for OrientationConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(value) = value else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        let Some(model) = value.downcast_ref::<ModelType>() else {
            return Ok(Some(value.clone()));
        };
        let result: BoxedValue = match model {
            ModelType::Horizontal => Rc::new(NativeType::Horizontal),
            ModelType::Vertical => Rc::new(NativeType::Vertical),
        };
        Ok(Some(result))
    }
    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some(value) = value else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        let Some(native) = value.downcast_ref::<NativeType>() else {
            return Ok(Some(value.clone()));
        };
        let result: BoxedValue = match native {
            NativeType::Horizontal => Rc::new(ModelType::Horizontal),
            NativeType::Vertical => Rc::new(ModelType::Vertical),
        };
        Ok(Some(result))
    }
}
ferroui_base::ferro_markup_type!(class OrientationConverter{namespace:"FerroDock.Ferro.Converters",this:Rc<OrientationConverter>,handles:[Rc<OrientationConverter>,Option<Rc<OrientationConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>OrientationConverter::new],fields:[Instance:Rc<OrientationConverter> =>OrientationConverter::instance],});
