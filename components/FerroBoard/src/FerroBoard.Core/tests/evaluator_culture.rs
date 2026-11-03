#[path = "../../../tests/support/native_evaluator_culture.rs"]
mod contract;
#[test]
fn native_locale_casing_and_evaluator_callbacks_match_source() {
    assert_eq!(
        contract::verify(include_str!(
            "../../../tests/fixtures/evaluator-culture.jsonl"
        )),
        697
    );
}

#[test]
fn engine_context_keeps_locale_values_across_subtype_changes() {
    use ferroboard_core::{
        engine::{KeyboardEngine, LanguageModels},
        layouts::{KeyboardResources, Subtype},
        settings::KeyboardSettings,
    };
    use std::{rc::Rc, sync::Arc};
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_show_suggestions(false);
    settings.set_glide_typing(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap();
    for language in ["en-US", "tr-TR", "ja-JP"] {
        engine
            .set_subtypes([Arc::new(Subtype::new(language.into()))])
            .unwrap();
        let original = engine.culture();
        let context = engine.create_compute_context().unwrap();
        assert_eq!(context.culture(), Some(original));
        assert_eq!(context.unshifted().culture(), Some(original));
    }
    engine.dispose().unwrap();
}
