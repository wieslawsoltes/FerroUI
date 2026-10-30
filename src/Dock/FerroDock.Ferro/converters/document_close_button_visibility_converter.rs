// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::DocumentCloseButtonShowMode;
use ferroui_base::{
    data::{converters::IMultiValueConverter, core::ValueType, BindingError},
    metadata::MarkupTyped,
    utilities::CultureInfo,
    BoxedValue,
};
use std::rc::Rc;
/// Determines original close-button visibility from mode and active state.
pub struct DocumentCloseButtonVisibilityConverter;
impl PartialEq for DocumentCloseButtonVisibilityConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DocumentCloseButtonVisibilityConverter {
    /// Creates the stateless source converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared native UI-thread instance.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<DocumentCloseButtonVisibilityConverter>=DocumentCloseButtonVisibilityConverter::new();}
        INSTANCE.with(Rc::clone)
    }
    fn parse_mode(value: Option<&BoxedValue>) -> DocumentCloseButtonShowMode {
        let Some(value) = value else {
            return DocumentCloseButtonShowMode::Always;
        };
        if let Some(mode) = value.downcast_ref::<DocumentCloseButtonShowMode>() {
            return *mode;
        }
        let parsed = value.downcast_ref::<String>().and_then(|text| {
            let text = text.trim();
            if let Ok(number) = text.parse::<i32>() {
                return Some(number);
            }
            // Enum.TryParse also accepts comma-separated names on non-Flags enums.
            // Resolve names through the source-generated native enum descriptor.
            let metadata = ferrodock_settings::DocumentCloseButtonShowModeMetadata::MARKUP;
            let mut result = 0_i32;
            for name in text.split(',') {
                result |=
                    i32::try_from(metadata.find_enum_member(name.trim(), true)?.value).ok()?;
            }
            Some(result)
        });
        parsed
            .and_then(|raw| DocumentCloseButtonShowMode::try_from(raw).ok())
            .unwrap_or(DocumentCloseButtonShowMode::Always)
    }
}
impl IMultiValueConverter for DocumentCloseButtonVisibilityConverter {
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        _target: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if values.len() < 2 {
            return Ok(Some(Rc::new(true)));
        }
        let mode = Self::parse_mode(values[0].as_ref());
        let active = values[1]
            .as_ref()
            .and_then(|v| v.downcast_ref::<bool>())
            .copied()
            .unwrap_or(false);
        Ok(Some(Rc::new(match mode {
            DocumentCloseButtonShowMode::Always => true,
            DocumentCloseButtonShowMode::Active => active,
            DocumentCloseButtonShowMode::Never => false,
        })))
    }
}
ferroui_base::ferro_markup_type!(class DocumentCloseButtonVisibilityConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<DocumentCloseButtonVisibilityConverter>,handles:[Rc<DocumentCloseButtonVisibilityConverter>,Option<Rc<DocumentCloseButtonVisibilityConverter>>],interfaces:[Rc<dyn IMultiValueConverter>],constructors:[()=>DocumentCloseButtonVisibilityConverter::new],fields:[Instance:Rc<DocumentCloseButtonVisibilityConverter> =>DocumentCloseButtonVisibilityConverter::instance],
});
