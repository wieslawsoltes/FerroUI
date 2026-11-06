//! Downstream native key extensions use Rust error propagation, not exception services.
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{EngineError, KeyboardEngine, LanguageModels},
    layouts::{
        AbstractKeyData, IKeyEvaluator, ITextKeyData, KeyDataError, KeyboardResources,
        ResourceTextListError, TextKey, TextKeyData,
    },
    settings::KeyboardSettings,
    text::{TextCaseError, TextCulture, UnsupportedCulture, Utf16Text},
};
use std::{cell::Cell, error::Error, rc::Rc};
type NativeResult<T> = Result<T, Box<dyn Error>>;

struct ExtensionKey {
    base: TextKeyData,
    calls: Cell<usize>,
    casing: bool,
}
impl AbstractKeyData for ExtensionKey {
    fn compute_nullable(
        self: Rc<Self>,
        _: Option<&dyn IKeyEvaluator>,
    ) -> Result<Option<TextKey>, KeyDataError> {
        Ok(Some(self))
    }
    fn as_string(&self, _: bool) -> Result<Option<Utf16Text>, KeyDataError> {
        self.calls.set(self.calls.get() + 1);
        if self.casing {
            let cause = TextCulture::get("ferroboard-invalid-culture").unwrap_err();
            Err(KeyDataError::TextCase(TextCaseError::UnsupportedCulture(
                cause,
            )))
        } else {
            Err(ResourceTextListError::Modified.into())
        }
    }
}
impl ITextKeyData for ExtensionKey {
    fn data(&self) -> &TextKeyData {
        &self.base
    }
    fn produces_text(&self) -> Result<bool, KeyDataError> {
        Ok(true)
    }
}
fn output(key: &dyn ITextKeyData) -> NativeResult<Option<Utf16Text>> {
    Ok(key.as_string(false)?)
}
fn input(engine: &KeyboardEngine, key: &dyn ITextKeyData) -> NativeResult<()> {
    engine.input_key(key)?;
    Ok(())
}
fn invalid_character() -> NativeResult<()> {
    TextKeyData::char(Utf16Text::from_units(vec![0xd800]), None)?;
    Ok(())
}

#[test]
fn external_key_errors_propagate_with_question_mark_and_typed_causes() {
    let key = ExtensionKey {
        base: TextKeyData::default(),
        calls: Cell::new(0),
        casing: false,
    };
    let error = output(&key).unwrap_err();
    assert_eq!(
        error.downcast_ref::<KeyDataError>(),
        Some(&KeyDataError::Collection(ResourceTextListError::Modified))
    );
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<ResourceTextListError>(),
        Some(&ResourceTextListError::Modified)
    );
    assert!(error.to_string().contains("collection"));
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::default());
    engine.attach(target.clone()).unwrap();
    let error = input(&engine, &key).unwrap_err();
    assert!(error.downcast_ref::<EngineError>().is_some());
    let key_error = error.source().unwrap();
    assert_eq!(
        key_error.downcast_ref::<KeyDataError>(),
        Some(&KeyDataError::Collection(ResourceTextListError::Modified))
    );
    assert_eq!(
        key_error
            .source()
            .unwrap()
            .downcast_ref::<ResourceTextListError>(),
        Some(&ResourceTextListError::Modified)
    );
    assert!(target.text().is_empty());
    assert_eq!(key.calls.get(), 2);
}

#[test]
fn invalid_unicode_and_culture_failures_retain_native_error_identity() {
    let error = invalid_character().unwrap_err();
    assert_eq!(
        error.downcast_ref::<KeyDataError>(),
        Some(&KeyDataError::InvalidSurrogate(0xd800))
    );
    assert!(error.source().is_none());
    assert!(error.to_string().contains("0xd800"));
    let key = ExtensionKey {
        base: TextKeyData::default(),
        calls: Cell::new(0),
        casing: true,
    };
    let error = output(&key).unwrap_err();
    let casing = error.source().unwrap();
    assert!(casing.downcast_ref::<TextCaseError>().is_some());
    let culture = casing.source().unwrap();
    assert_eq!(
        culture.downcast_ref::<UnsupportedCulture>().unwrap().0,
        "ferroboard-invalid-culture"
    );
    assert!(culture.source().is_none());
    assert!(error.to_string().contains("ferroboard-invalid-culture"));
}

#[test]
fn dynamic_multi_key_predicate_errors_return_through_native_metadata_without_unwind() {
    use ferroboard_core::layouts::MultiTextKeyData;
    use ferroboard_core_ferro::TextKeyDataMetadata;
    use ferroui_base::metadata::{MarkupTyped, into_markup_value};
    ferroboard_core_ferro::register_types();
    let multi = Rc::new(MultiTextKeyData::default());
    multi.set_code_points(None);
    let key: TextKey = multi;
    assert_eq!(key.produces_text(), Err(KeyDataError::NullReference));
    let getter = TextKeyDataMetadata::MARKUP
        .properties
        .iter()
        .find(|property| property.name == "ProducesText")
        .unwrap()
        .get
        .unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        getter(&[into_markup_value(key)])
    }));
    assert!(
        result.is_ok(),
        "a domain key error must not unwind through native metadata"
    );
    let error = result.unwrap().unwrap_err();
    assert!(
        error.to_string().contains("required key data is missing"),
        "{error}"
    );
}
