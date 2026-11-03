//! Downstream custom subtypes remain usable when a locale identifier is invalid.
use ferroboard_core::{
    editor::{ITextInputTarget, TextBufferTarget},
    engine::{EngineError, KeyboardEngine, LanguageModels},
    layouts::{KeyboardResources, Subtype, SubtypeValues},
    settings::KeyboardSettings,
    text::{TextUtils, Utf16Text},
};
use std::{error::Error, rc::Rc, sync::Arc};

#[test]
fn custom_invalid_language_can_type_cycle_and_display_through_public_api()
-> Result<(), Box<dyn Error>> {
    // Leave suggestions/glide enabled: the locale fallback must also survive the
    // normal language-model and suggestion updates that follow subtype activation.
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_learn_words(false);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_default()?),
        LanguageModels::new(),
    )?;
    let target = Rc::new(TextBufferTarget::new("".into(), None));
    engine.attach(target.clone())?;
    for tag in ["invalid_tag!", "not a locale", "en/US"] {
        let invalid = Arc::new(Subtype::from_values(SubtypeValues {
            language_tag: Some(tag.into()),
            ..Subtype::shared_default().values()
        }));
        let turkish = Arc::new(Subtype::from_values(SubtypeValues {
            language_tag: Some("tr-TR".into()),
            ..Subtype::shared_default().values()
        }));
        engine.set_subtypes([invalid.clone(), turkish.clone()])?;
        assert!(Arc::ptr_eq(&engine.active_subtype(), &invalid));
        assert_eq!(engine.culture().name(), "");
        assert_eq!(
            KeyboardEngine::get_subtype_display_name(&invalid)?,
            Utf16Text::from(tag)
        );
        assert_eq!(
            TextUtils::capitalize(&"i".into(), engine.culture()),
            Utf16Text::from("I")
        );
        engine.type_text(&"ok ".into())?;
        engine.next_subtype()?;
        assert!(Arc::ptr_eq(&engine.active_subtype(), &turkish));
        assert_eq!(
            TextUtils::capitalize(&"i".into(), engine.culture()),
            Utf16Text::from("İ")
        );
        engine.previous_subtype()?;
        assert!(Arc::ptr_eq(&engine.active_subtype(), &invalid));
        assert_eq!(engine.culture().name(), "");
    }
    assert_eq!(target.text(), Utf16Text::from("ok ok ok "));
    Ok(())
}

#[test]
fn locale_fallback_preserves_null_encoding_errors_and_explicit_display_override()
-> Result<(), Box<dyn Error>> {
    for (tag, null) in [
        (None, true),
        (Some(Utf16Text::from_units(vec![0xd800])), false),
    ] {
        let subtype = Subtype::from_values(SubtypeValues {
            language_tag: tag,
            ..Subtype::shared_default().values()
        });
        let error = KeyboardEngine::get_subtype_display_name(&subtype).unwrap_err();
        assert!(if null {
            matches!(error, EngineError::ArgumentNull("name"))
        } else {
            matches!(error, EngineError::UnrepresentedText)
        });
        let named = Subtype::from_values(SubtypeValues {
            display_name: Some("My keyboard".into()),
            ..subtype.values()
        });
        assert_eq!(
            KeyboardEngine::get_subtype_display_name(&named)?,
            Utf16Text::from("My keyboard")
        );
    }
    Ok(())
}
