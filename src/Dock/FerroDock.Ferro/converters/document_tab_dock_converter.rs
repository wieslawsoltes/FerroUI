// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::DocumentTabLayout;
use ferroui_base::FerroProperty;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use ferroui_controls::Dock;
use std::rc::Rc;
/// Original DocumentTabDockConverter.
pub struct DocumentTabDockConverter;
impl PartialEq for DocumentTabDockConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DocumentTabDockConverter {
    /// Creates the source stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared source converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<DocumentTabDockConverter>=DocumentTabDockConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for DocumentTabDockConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let result: BoxedValue = match value.and_then(|v| v.downcast_ref::<DocumentTabLayout>()) {
            Some(DocumentTabLayout::Left) => Rc::new(Dock::Left),
            Some(DocumentTabLayout::Right) => Rc::new(Dock::Right),
            Some(DocumentTabLayout::Top) => Rc::new(Dock::Top),
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
ferroui_base::ferro_markup_type!(class DocumentTabDockConverter {namespace:"FerroDock.Ferro.Converters",this:Rc<DocumentTabDockConverter>,handles:[Rc<DocumentTabDockConverter>,Option<Rc<DocumentTabDockConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>DocumentTabDockConverter::new],fields:[Instance:Rc<DocumentTabDockConverter> =>DocumentTabDockConverter::instance],});
