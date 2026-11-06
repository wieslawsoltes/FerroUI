//! Language extensions propagate typed failures through ordinary Rust error chains.
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{EngineError, KeyboardEngine, LanguageModelError, LanguageModels},
    glide::{GlideError, GlideTypingClassifier},
    layouts::{KeyDataError, KeyRect, KeyboardResources, ResourceTextListError},
    serialization::JsonBridgeError,
    settings::KeyboardSettings,
    text::{
        DictionaryError, NormalizationError, SuggestionError, TextCaseError, TextCulture,
        UserDictionary, UserDictionaryInputError, Utf16Text, WordDictionary,
    },
};
use std::{
    error::Error,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
type NativeResult<T> = Result<T, Box<dyn Error>>;

fn normalize(text: &Utf16Text) -> NativeResult<Utf16Text> {
    Ok(WordDictionary::normalize(text)?)
}
fn dictionary(json: &str) -> NativeResult<WordDictionary> {
    Ok(WordDictionary::from_json(json, "en".into())?)
}
fn layout(classifier: &GlideTypingClassifier) -> NativeResult<()> {
    let rect = KeyRect::new(0.0, 0.0, 40.0, 40.0);
    classifier.set_layout([(65, rect), (97, rect)])?;
    Ok(())
}
fn words(classifier: &GlideTypingClassifier) -> NativeResult<()> {
    classifier.set_words([(Utf16Text::from_units(vec![0xd800, 97]), 10)])?;
    Ok(())
}
fn load(models: &LanguageModels) -> NativeResult<()> {
    models.get_dictionary(&"EN-us".into())?;
    Ok(())
}
fn attach(engine: &Rc<KeyboardEngine>, target: Rc<dyn ITextInputTarget>) -> NativeResult<()> {
    engine.attach(target)?;
    Ok(())
}
fn block_missing(dictionary: &UserDictionary) -> NativeResult<()> {
    dictionary.block_nullable(None)?;
    Ok(())
}

#[test]
fn malformed_dictionary_and_glide_inputs_keep_their_typed_causes() {
    let malformed = Utf16Text::from_units(vec![0xd800, 97]);
    let error = normalize(&malformed).unwrap_err();
    assert_eq!(
        error.downcast_ref(),
        Some(&NormalizationError::InvalidUtf16)
    );
    assert!(error.source().is_none());
    let error = dictionary("{").err().unwrap();
    assert!(matches!(
        error.downcast_ref(),
        Some(DictionaryError::Json(_))
    ));
    assert!(matches!(
        error.source().unwrap().downcast_ref(),
        Some(JsonBridgeError::Syntax { .. })
    ));
    let classifier = GlideTypingClassifier::default();
    let error = layout(&classifier).unwrap_err();
    assert!(matches!(
        error.downcast_ref(),
        Some(GlideError::DuplicateLowercaseKey(97))
    ));
    assert!(error.to_string().contains("U+0061"));
    assert!(error.source().is_none());
    classifier
        .set_layout([(97, KeyRect::new(0.0, 0.0, 40.0, 40.0))])
        .unwrap();
    let error = words(&classifier).unwrap_err();
    assert!(matches!(
        error.downcast_ref(),
        Some(GlideError::Normalization(NormalizationError::InvalidUtf16))
    ));
    assert_eq!(
        error.source().unwrap().downcast_ref(),
        Some(&NormalizationError::InvalidUtf16)
    );
    // Real dictionary insertion and a suggestion wrapper retain that same typed leaf.
    let cause = WordDictionary::default().add(&malformed, 10).unwrap_err();
    let error = SuggestionError::Dictionary(cause);
    assert_eq!(
        error.source().unwrap().source().unwrap().downcast_ref(),
        Some(&NormalizationError::InvalidUtf16)
    );
}

#[test]
fn failed_language_factory_remains_cached_through_engine_error_propagation() {
    let models = LanguageModels::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let factory_calls = calls.clone();
    models
        .register_factory(
            &"en".into(),
            Some(Arc::new(move || {
                factory_calls.fetch_add(1, Ordering::SeqCst);
                WordDictionary::from_json("{", "en".into())
                    .map(|dictionary| Some(Arc::new(dictionary)))
                    .map_err(|error| Arc::new(LanguageModelError::Dictionary(error)))
            })),
        )
        .unwrap();
    let original = models.get_dictionary(&"en".into()).err().unwrap();
    let error = load(&models).unwrap_err();
    assert!(Arc::ptr_eq(
        &original,
        error.downcast_ref::<Arc<LanguageModelError>>().unwrap()
    ));
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_glide_typing(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        models,
    )
    .unwrap();
    let target = Rc::new(TextBufferTarget::default());
    let error = attach(&engine, target.clone()).unwrap_err();
    let Some(EngineError::Language(shared)) = error.downcast_ref() else {
        panic!("unexpected engine failure: {error}");
    };
    assert!(Arc::ptr_eq(&original, shared));
    let language = error.source().unwrap();
    assert!(std::ptr::eq(
        language.downcast_ref::<LanguageModelError>().unwrap(),
        original.as_ref()
    ));
    let dictionary = language.source().unwrap();
    assert!(dictionary.downcast_ref::<DictionaryError>().is_some());
    assert!(matches!(
        dictionary.source().unwrap().downcast_ref(),
        Some(JsonBridgeError::Syntax { .. })
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(target.text(), Utf16Text::default());
    engine.dispose().unwrap();
}

#[test]
fn cached_factory_panic_preserves_its_original_native_payload() {
    let models = LanguageModels::new();
    models
        .register_factory(
            &"en".into(),
            Some(Arc::new(|| std::panic::panic_any(17u32))),
        )
        .unwrap();
    let original = models.get_dictionary(&"en".into()).err().unwrap();
    let error = load(&models).unwrap_err();
    let cached = error.downcast_ref::<Arc<LanguageModelError>>().unwrap();
    assert!(Arc::ptr_eq(&original, cached));
    let LanguageModelError::Panic(panic) = cached.as_ref() else {
        panic!("factory panic was not retained");
    };
    assert_eq!(
        panic.inspect(|payload| payload.downcast_ref::<u32>().copied()),
        Some(17)
    );
    assert!(error.source().is_none());
}

#[test]
fn wrappers_borrow_native_causes_and_learning_rejection_stays_fallible() {
    let error =
        DictionaryError::Resource(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    let DictionaryError::Resource(original) = &error else {
        unreachable!()
    };
    let cause = error
        .source()
        .unwrap()
        .downcast_ref::<std::io::Error>()
        .unwrap();
    assert!(std::ptr::eq(original, cause));
    assert_eq!(cause.kind(), std::io::ErrorKind::PermissionDenied);
    let error = EngineError::Glide(GlideError::KeyData(KeyDataError::Collection(
        ResourceTextListError::Modified,
    )));
    assert_eq!(
        error
            .source()
            .unwrap()
            .source()
            .unwrap()
            .source()
            .unwrap()
            .downcast_ref(),
        Some(&ResourceTextListError::Modified)
    );
    let error = SuggestionError::Case(TextCaseError::UnsupportedCulture(
        TextCulture::get("ferroboard-invalid-culture").unwrap_err(),
    ));
    assert!(error.source().unwrap().source().is_some());
    let error = EngineError::Normalization(NormalizationError::InvalidCodePoint);
    assert_eq!(
        error.source().unwrap().downcast_ref(),
        Some(&NormalizationError::InvalidCodePoint)
    );
    let error = SuggestionError::Normalization(NormalizationError::InvalidUtf16);
    assert_eq!(
        error.source().unwrap().downcast_ref(),
        Some(&NormalizationError::InvalidUtf16)
    );
    let error = block_missing(&UserDictionary::default()).unwrap_err();
    assert_eq!(
        error.downcast_ref(),
        Some(&UserDictionaryInputError::ArgumentNull("key"))
    );
    assert!(error.source().is_none());
}
