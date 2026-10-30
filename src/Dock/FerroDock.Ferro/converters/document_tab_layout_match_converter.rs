// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::DocumentTabLayout;
use ferroui_base::{
    data::{converters::IValueConverter, core::ValueType, BindingError},
    utilities::CultureInfo,
    BoxedValue,
};
use ferroui_base::{metadata::MarkupTyped, FerroProperty};
use std::rc::Rc;
/// Original DocumentTabLayoutMatchConverter binding converter.
pub struct DocumentTabLayoutMatchConverter;
impl PartialEq for DocumentTabLayoutMatchConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DocumentTabLayoutMatchConverter {
    /// Creates the original stateless converter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Gets the shared converter on the native UI thread.
    pub fn instance() -> Rc<Self> {
        ferroui_base::ferro_thread_local! {static INSTANCE:Rc<DocumentTabLayoutMatchConverter>=DocumentTabLayoutMatchConverter::new();}
        INSTANCE.with(Rc::clone)
    }
}
impl IValueConverter for DocumentTabLayoutMatchConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target: ValueType,
        parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let Some((layout, text)) = value
            .and_then(|v| v.downcast_ref::<DocumentTabLayout>())
            .zip(parameter.and_then(|p| p.downcast_ref::<String>()))
        else {
            return Ok(Some(FerroProperty::unset_value()));
        };
        let metadata = ferrodock_settings::DocumentTabLayoutMetadata::MARKUP;
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
ferroui_base::ferro_markup_type!(class DocumentTabLayoutMatchConverter {
    namespace:"FerroDock.Ferro.Converters",this:Rc<DocumentTabLayoutMatchConverter>,handles:[Rc<DocumentTabLayoutMatchConverter>,Option<Rc<DocumentTabLayoutMatchConverter>>],interfaces:[Rc<dyn IValueConverter>],constructors:[()=>DocumentTabLayoutMatchConverter::new],fields:[Instance:Rc<DocumentTabLayoutMatchConverter> =>DocumentTabLayoutMatchConverter::instance],
});

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_layout_match_accepts_comma_names_numeric_candidates_and_returns_unset_for_wrong_types(
    ) {
        let c = DocumentTabLayoutMatchConverter::new();
        for (text, expected) in [
            ("Right", true),
            ("right", true),
            ("Top, Left, Right", true),
            (",, Right,,", true),
            (" +2 ", true),
            ("Top,Left", false),
            ("3", false),
            ("2.0", false),
            ("", false),
            ("2147483648", false),
            ("+ 2", false),
        ] {
            let value: BoxedValue = Rc::new(DocumentTabLayout::Right);
            let parameter: BoxedValue = Rc::new(text.to_owned());
            let result = c
                .convert(
                    Some(&value),
                    ValueType::of::<bool>(),
                    Some(&parameter),
                    &CultureInfo::invariant_culture(),
                )
                .unwrap()
                .unwrap();
            assert_eq!(result.downcast_ref::<bool>(), Some(&expected), "{text}");
        }
        let wrong: BoxedValue = Rc::new(2_i32);
        let text: BoxedValue = Rc::new("Right".to_owned());
        for value in [None, Some(&wrong)] {
            assert!(c
                .convert(
                    value,
                    ValueType::of::<bool>(),
                    Some(&text),
                    &CultureInfo::invariant_culture()
                )
                .unwrap()
                .unwrap()
                .is::<ferroui_base::UnsetValueType>());
        }
        assert!(c
            .convert_back(
                None,
                ValueType::of::<bool>(),
                None,
                &CultureInfo::invariant_culture()
            )
            .unwrap()
            .unwrap()
            .is::<ferroui_base::UnsetValueType>());
    }
}
