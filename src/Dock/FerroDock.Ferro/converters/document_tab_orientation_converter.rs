// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::DocumentTabLayout;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use ferroui_base::{layout::Orientation, FerroProperty};
use std::rc::Rc;
/// Original DocumentTabOrientationConverter.
pub struct DocumentTabOrientationConverter;
impl PartialEq for DocumentTabOrientationConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DocumentTabOrientationConverter {
    /// Creates the source stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared source converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<DocumentTabOrientationConverter>=DocumentTabOrientationConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for DocumentTabOrientationConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let result: BoxedValue = match value.and_then(|v| v.downcast_ref::<DocumentTabLayout>()) {
            Some(DocumentTabLayout::Left | DocumentTabLayout::Right) => {
                Rc::new(Orientation::Vertical)
            }
            Some(DocumentTabLayout::Top) => Rc::new(Orientation::Horizontal),
            None => FerroProperty::unset_value(),
        };
        Ok(Some(result))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(FerroProperty::unset_value()))
    }
}
ferroui_base::ferro_markup_type!(class DocumentTabOrientationConverter {namespace:"FerroDock.Ferro.Converters",this:Rc<DocumentTabOrientationConverter>,handles:[Rc<DocumentTabOrientationConverter>,Option<Rc<DocumentTabOrientationConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>DocumentTabOrientationConverter::new],fields:[Instance:Rc<DocumentTabOrientationConverter> =>DocumentTabOrientationConverter::instance],});
