use ferroboard_core::{
    engine::{KeyboardEngine, LanguageModels},
    layouts::{KeyboardResources, PunctuationRule},
    settings::KeyboardSettings,
    text::WordDictionary,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

fn fixture() -> Vec<Value> {
    include_str!("../../../tests/fixtures/engine-constructor.jsonl")
        .lines()
        .map(|row| serde_json::from_str(row).unwrap())
        .collect()
}

#[test]
fn nullable_constructor_dependencies_services_learning_and_disposal_match_source() {
    let rows = fixture();
    for resource_kind in ["empty", "bundled"] {
        for explicit_settings in [false, true] {
            for explicit_models in [false, true] {
                let expected = rows
                    .iter()
                    .filter(|r| {
                        r["scope"] == "native"
                            && r["resourceKind"] == resource_kind
                            && r["explicitSettings"] == explicit_settings
                            && r["explicitModels"] == explicit_models
                    })
                    .collect::<Vec<_>>();
                assert_eq!(expected.len(), 4);
                let settings = explicit_settings.then(|| {
                    let settings = Rc::new(KeyboardSettings::new());
                    settings.set_glide_typing(false);
                    settings
                });
                let resources = Rc::new(if resource_kind == "empty" {
                    KeyboardResources::create_empty()
                } else {
                    KeyboardResources::create_default().unwrap()
                });
                let models = explicit_models.then(LanguageModels::new);
                let first =
                    KeyboardEngine::new(settings.clone(), Some(resources.clone()), models.clone())
                        .unwrap();
                let second =
                    KeyboardEngine::new(settings.clone(), Some(resources.clone()), models.clone())
                        .unwrap();
                assert_eq!(resources.changed().handler_count(), 2);
                let initial = json!({
                    "settingsShared": Rc::ptr_eq(&first.settings(), &second.settings()),
                    "resourcesShared": Rc::ptr_eq(&first.resources(), &second.resources()),
                    "modelsShared": Arc::ptr_eq(&first.language_models(), &second.language_models()),
                    "suppliedSettings": settings.as_ref().is_none_or(|s| Rc::ptr_eq(s, &first.settings())),
                    "suppliedResources": Rc::ptr_eq(&resources, &first.resources()),
                    "suppliedModels": models.as_ref().is_none_or(|m| Arc::ptr_eq(m, &first.language_models())),
                    "sourceDefault": false,
                    "clipboardShared": Rc::ptr_eq(&first.clipboard_service(), &second.clipboard_service()),
                    "clipboardHistoryShared": Rc::ptr_eq(&first.clipboard_history(), &second.clipboard_history()),
                    "emojiHistoryShared": Rc::ptr_eq(&first.emoji_history(), &second.emoji_history()),
                    "glideShared": Arc::ptr_eq(&first.glide_classifier(), &second.glide_classifier()),
                    "subtypesShared": Rc::ptr_eq(&first.subtypes(), &second.subtypes()),
                    "glideTyping": first.settings().glide_typing(),
                    "hasEnglish": first.language_models().has_dictionary(&"en-US".into()),
                    "loadedFirst": first.language_models().is_loaded(&"en".into()),
                    "loadedSecond": second.language_models().is_loaded(&"en".into()),
                    "usersFirst": first.language_models().user_dictionaries().len(),
                    "usersSecond": second.language_models().user_dictionaries().len(),
                    "detached": first.target().is_none() && first.editor().is_none()
                        && second.target().is_none() && second.editor().is_none(),
                });
                assert_eq!(initial, expected[0]["result"], "{}", expected[0]);

                let calls = Arc::new(AtomicUsize::new(0));
                let counter = calls.clone();
                first
                    .language_models()
                    .register_factory(
                        &"zz".into(),
                        Some(Arc::new(move || {
                            counter.fetch_add(1, Ordering::SeqCst);
                            Ok(Some(Arc::new(WordDictionary::new("zz".into()))))
                        })),
                    )
                    .unwrap();
                let before = calls.load(Ordering::SeqCst);
                let a = first
                    .language_models()
                    .get_dictionary(&"zz-ZZ".into())
                    .unwrap();
                let b = second
                    .language_models()
                    .get_dictionary(&"zz-ZZ".into())
                    .unwrap();
                let trace = Arc::new(Mutex::new(Vec::new()));
                for (owner, name) in [(&first, "first"), (&second, "second")] {
                    let trace = trace.clone();
                    owner
                        .language_models()
                        .user_dictionary_changed()
                        .add(Arc::new(move |v| {
                            trace
                                .lock()
                                .unwrap()
                                .push(format!("{name}:{}", v.language.to_utf8().unwrap()));
                        }));
                }
                let first_user = first.language_models().get_user_dictionary(&"en-US".into());
                let second_user = second
                    .language_models()
                    .get_user_dictionary(&"en-GB".into());
                first_user.learn(&"constructorprobe".into(), None);
                assert_eq!(
                    json!({
                        "before": before,
                        "calls": calls.load(Ordering::SeqCst),
                        "dictionaryShared": a.as_ref().zip(b.as_ref()).is_some_and(|(a,b)| Arc::ptr_eq(a,b)),
                        "secondHasDictionary": b.is_some(),
                        "userShared": first_user.ptr_eq(&second_user),
                        "firstCount": first_user.get_count(&"constructorprobe".into()),
                        "secondCount": second_user.get_count(&"constructorprobe".into()),
                        "trace": *trace.lock().unwrap(),
                    }),
                    expected[1]["result"],
                    "{}",
                    expected[1]
                );

                for engine in [&first, &second] {
                    engine.language_models().register_dictionary(
                        &"en".into(),
                        Some(Arc::new(WordDictionary::new("en".into()))),
                    );
                }
                let events = Rc::new(RefCell::new(Vec::new()));
                for (engine, name) in [(&first, "first"), (&second, "second")] {
                    let events = events.clone();
                    engine
                        .keyboard_changed()
                        .add(Rc::new(move |_| events.borrow_mut().push(name)));
                }
                first.dispose().unwrap();
                assert_eq!(resources.changed().handler_count(), 1);
                resources.add_punctuation_rule(Arc::new(PunctuationRule::from_values(
                    ferroboard_core::layouts::PunctuationRuleValues {
                        extension_id: Some("constructor-probe".into()),
                        id: Some("rule".into()),
                        ..Default::default()
                    },
                )));
                assert_eq!(json!({"events":*events.borrow()}), expected[2]["result"]);
                second.dispose().unwrap();
                assert_eq!(resources.changed().handler_count(), 0);
                events.borrow_mut().clear();
                resources.add_punctuation_rule(Arc::new(PunctuationRule::from_values(
                    ferroboard_core::layouts::PunctuationRuleValues {
                        extension_id: Some("constructor-probe".into()),
                        id: Some("rule".into()),
                        ..Default::default()
                    },
                )));
                assert_eq!(json!({"events":*events.borrow()}), expected[3]["result"]);
                let first_weak = Rc::downgrade(&first);
                let second_weak = Rc::downgrade(&second);
                drop(first);
                drop(second);
                assert!(first_weak.upgrade().is_none());
                assert!(second_weak.upgrade().is_none());
            }
        }
    }
}

#[test]
fn omitted_resources_create_owned_bundled_registries_with_explicit_sharing_available() {
    // The frozen source-only rows retain C# process-global identity observations. Native
    // ownership is a deliberate migration, not a claim that those identity rows now match.
    assert_eq!(
        fixture()
            .iter()
            .filter(|r| r["scope"] == "source-only")
            .count(),
        16
    );
    for settings in [None, Some(Rc::new(KeyboardSettings::new()))] {
        for models in [None, Some(LanguageModels::new())] {
            let engine = KeyboardEngine::new(settings.clone(), None, models.clone()).unwrap();
            assert!(!engine.resources().extensions().is_empty());
            assert!(!engine.resources().subtype_presets().is_empty());
            assert!(engine.keyboard().is_ok());
            if let Some(settings) = &settings {
                assert!(Rc::ptr_eq(&engine.settings(), settings));
            }
            if let Some(models) = &models {
                assert!(Arc::ptr_eq(&engine.language_models(), models));
            }
            let resources = engine.resources();
            assert_eq!(resources.changed().handler_count(), 1);
            engine.dispose().unwrap();
            assert_eq!(resources.changed().handler_count(), 0);
        }
    }
}

#[test]
fn native_defaults_are_isolated_while_explicit_registry_sharing_preserves_identity() {
    use ferroboard_core::{
        editor::{ITextInputTarget, TextBufferTarget},
        layouts::ComponentName,
    };
    let first = KeyboardEngine::with_defaults().unwrap();
    let second = KeyboardEngine::with_defaults().unwrap();
    assert!(!Rc::ptr_eq(&first.resources(), &second.resources()));
    assert!(!Rc::ptr_eq(&first.settings(), &second.settings()));
    assert!(!Arc::ptr_eq(
        &first.language_models(),
        &second.language_models()
    ));
    let rule = Arc::new(PunctuationRule::from_values(
        ferroboard_core::layouts::PunctuationRuleValues {
            extension_id: Some("native-default-probe".into()),
            id: Some("rule".into()),
            ..Default::default()
        },
    ));
    first.resources().add_punctuation_rule(rule.clone());
    let name = ComponentName::new("native-default-probe", "rule");
    assert!(Arc::ptr_eq(
        &first.resources().get_punctuation_rule(&name),
        &rule
    ));
    assert!(!Arc::ptr_eq(
        &second.resources().get_punctuation_rule(&name),
        &rule
    ));
    let shared = KeyboardEngine::new(None, Some(first.resources()), None).unwrap();
    assert!(Rc::ptr_eq(&first.resources(), &shared.resources()));
    assert!(Arc::ptr_eq(
        &shared.resources().get_punctuation_rule(&name),
        &rule
    ));
    let target = Rc::new(TextBufferTarget::default());
    first.attach(target.clone()).unwrap();
    first.input_text(&"native defaults 😀".into()).unwrap();
    assert_eq!(target.text().to_utf8().unwrap(), "native defaults 😀");
    assert!(second.target().is_none());
    let weak = Rc::downgrade(&first);
    let resources = first.resources();
    first.dispose().unwrap();
    drop(first);
    assert!(weak.upgrade().is_none());
    assert_eq!(resources.changed().handler_count(), 1);
    shared.dispose().unwrap();
    second.dispose().unwrap();
    assert_eq!(resources.changed().handler_count(), 0);
}

#[test]
fn independent_native_default_registries_can_be_constructed_on_worker_threads() {
    let workers: Vec<_> = (0..2)
        .map(|_| {
            std::thread::spawn(|| {
                let engine = KeyboardEngine::with_defaults().unwrap();
                let count = engine.resources().extensions().len();
                assert!(count > 0);
                engine.dispose().unwrap();
                count
            })
        })
        .collect();
    let counts: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(counts[0], counts[1]);
}
