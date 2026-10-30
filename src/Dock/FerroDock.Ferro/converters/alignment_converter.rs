// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::Alignment as ModelType;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue, FerroProperty,
};
use ferroui_controls::Dock as NativeType;
use std::rc::Rc;
/// Original AlignmentConverter preserving unrelated input object identity.
pub struct AlignmentConverter;
impl PartialEq for AlignmentConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl AlignmentConverter {
    /// Creates the source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the source shared instance on the UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<AlignmentConverter>=AlignmentConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for AlignmentConverter {
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
            ModelType::Left => Rc::new(NativeType::Left),
            ModelType::Bottom => Rc::new(NativeType::Bottom),
            ModelType::Right => Rc::new(NativeType::Right),
            ModelType::Top => Rc::new(NativeType::Top),
            ModelType::Unset => FerroProperty::unset_value(),
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
            NativeType::Left => Rc::new(ModelType::Left),
            NativeType::Bottom => Rc::new(ModelType::Bottom),
            NativeType::Right => Rc::new(ModelType::Right),
            NativeType::Top => Rc::new(ModelType::Top),
        };
        Ok(Some(result))
    }
}
ferroui_base::ferro_markup_type!(class AlignmentConverter{namespace:"FerroDock.Ferro.Converters",this:Rc<AlignmentConverter>,handles:[Rc<AlignmentConverter>,Option<Rc<AlignmentConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>AlignmentConverter::new],fields:[Instance:Rc<AlignmentConverter> =>AlignmentConverter::instance],});
