// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::SplitViewDisplayMode as ModelType;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue, FerroProperty,
};
use ferroui_controls::SplitViewDisplayMode as NativeType;
use std::rc::Rc;
/// Original SplitViewDisplayModeConverter preserving unrelated input object identity.
pub struct SplitViewDisplayModeConverter;
impl PartialEq for SplitViewDisplayModeConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl SplitViewDisplayModeConverter {
    /// Creates the source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the source shared instance on the UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<SplitViewDisplayModeConverter>=SplitViewDisplayModeConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for SplitViewDisplayModeConverter {
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
            ModelType::Inline => Rc::new(NativeType::Inline),
            ModelType::CompactInline => Rc::new(NativeType::CompactInline),
            ModelType::Overlay => Rc::new(NativeType::Overlay),
            ModelType::CompactOverlay => Rc::new(NativeType::CompactOverlay),
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
            NativeType::Inline => Rc::new(ModelType::Inline),
            NativeType::CompactInline => Rc::new(ModelType::CompactInline),
            NativeType::Overlay => Rc::new(ModelType::Overlay),
            NativeType::CompactOverlay => Rc::new(ModelType::CompactOverlay),
        };
        Ok(Some(result))
    }
}
ferroui_base::ferro_markup_type!(class SplitViewDisplayModeConverter{namespace:"FerroDock.Ferro.Converters",this:Rc<SplitViewDisplayModeConverter>,handles:[Rc<SplitViewDisplayModeConverter>,Option<Rc<SplitViewDisplayModeConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>SplitViewDisplayModeConverter::new],fields:[Instance:Rc<SplitViewDisplayModeConverter> =>SplitViewDisplayModeConverter::instance],});
