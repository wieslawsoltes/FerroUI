// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::DocumentLayoutMode;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use ferroui_base::{metadata::MarkupTyped, FerroProperty};
use std::rc::Rc;
/// Original DocumentLayoutModeMatchConverter binding converter.
pub struct DocumentLayoutModeMatchConverter;
impl PartialEq for DocumentLayoutModeMatchConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DocumentLayoutModeMatchConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<DocumentLayoutModeMatchConverter>=DocumentLayoutModeMatchConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for DocumentLayoutModeMatchConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some((layout, text)) = value
            .and_then(|v| v.downcast_ref::<DocumentLayoutMode>())
            .zip(parameter.and_then(|p| p.downcast_ref::<String>()))
        else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        let metadata = ferrodock_settings::DocumentLayoutModeMetadata::MARKUP;
        for part in text.split(',').filter(|p| !p.is_empty()) {
            let candidate = part.trim();
            let parsed = candidate.parse::<i32>().ok().or_else(|| {
                metadata
                    .find_enum_member(candidate, true)
                    .and_then(|m| i32::try_from(m.value).ok())
            });
            if parsed == Some(*layout as i32) {
                return Ok(Some(Rc::new(true)));
            }
        }
        Ok(Some(Rc::new(false)))
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
ferroui_base::ferro_markup_type!(class DocumentLayoutModeMatchConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<DocumentLayoutModeMatchConverter>,handles:[Rc<DocumentLayoutModeMatchConverter>,Option<Rc<DocumentLayoutModeMatchConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>DocumentLayoutModeMatchConverter::new],fields:[Instance:Rc<DocumentLayoutModeMatchConverter> =>DocumentLayoutModeMatchConverter::instance],
});
