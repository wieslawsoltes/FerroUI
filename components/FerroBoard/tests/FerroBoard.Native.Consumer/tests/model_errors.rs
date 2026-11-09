//! Public native errors retain their diagnostics through FerroUI metadata.
use ferroboard_core::{
    layouts::{KeyDataError, TextKeyData},
    text::Utf16Text,
};
use ferroboard_core_ferro::TextKeyDataMetadata;
use ferroui_base::metadata::{MarkupInvokeError, MarkupTyped, into_markup_value};

#[test]
fn text_key_factory_metadata_reports_native_errors_without_exception_type_names() {
    ferroboard_core_ferro::register_types();
    let factory = TextKeyDataMetadata::MARKUP
        .find_methods("Char")
        .find(|method| method.parameters.len() == 1)
        .unwrap();
    for (text, expected) in [
        (None, KeyDataError::NullCharacter),
        (Some(Utf16Text::default()), KeyDataError::EmptyCharacter),
        (
            Some(Utf16Text::from_units(vec![0xd800])),
            KeyDataError::InvalidSurrogate(0xd800),
        ),
    ] {
        assert_eq!(
            TextKeyData::char_nullable(text.clone(), None).err(),
            Some(expected.clone())
        );
        let result = (factory.invoke)(&[into_markup_value(text)]);
        assert!(
            matches!(result, Err(MarkupInvokeError::Failed(message)) if message == expected.to_string())
        );
    }
}

struct CustomKey(TextKeyData);
impl ferroboard_core::layouts::AbstractKeyData for CustomKey {
    fn compute_nullable(
        self: std::rc::Rc<Self>,
        _: Option<&dyn ferroboard_core::layouts::IKeyEvaluator>,
    ) -> Result<Option<ferroboard_core::layouts::TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, display: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        self.0.as_string(display)
    }
}
impl ferroboard_core::layouts::ITextKeyData for CustomKey {
    fn data(&self) -> &TextKeyData {
        &self.0
    }
}
#[test]
fn custom_key_diagnostics_use_rust_type_names_without_runtime_mapping() {
    let key: ferroboard_core::layouts::TextKey =
        std::rc::Rc::new(CustomKey(TextKeyData::default()));
    assert_eq!(key.diagnostic_name(), std::any::type_name::<CustomKey>());
    let text = key.to_text().unwrap().unwrap().to_utf8().unwrap();
    assert!(text.starts_with(&format!("{}(code=", std::any::type_name::<CustomKey>())));
}
