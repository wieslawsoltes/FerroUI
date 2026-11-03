//! Selected callback/target projections from the unchanged complete panel lifecycle oracle.
//! Constructor tree/subscriptions/Attach itself remain SOURCE-ONLY until the public native type exists.
use crate::{
    panel_context::SettingsPanelContext, theme_choices::ThemeChoices, view_model::MainViewModel,
};
use ferroboard_core::{
    engine::LanguageModels,
    settings::{KeyboardSettings, ThemeMode},
};
use ferroui_reactiveui::ICommand;
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn snapshot(settings: &KeyboardSettings, models: &Arc<LanguageModels>) -> Value {
    json!({"numberRow":settings.number_row(),"keyHeight":settings.key_height(),"theme":settings.theme_mode().0,"subtypes":settings.subtypes().borrow().iter().map(|s|s.as_deref().map(|s|s.encode_utf16().collect::<Vec<_>>())).collect::<Vec<_>>(),"words":models.get_user_dictionary(&"en".into()).get_count(&"keepword".into())})
}
#[cfg(target_os = "macos")]
#[test]
fn original_retained_callback_projections_resolve_current_context_instead_of_build_target() {
    use crate::languages::{LanguageCatalog, LanguagePanelViewModel};
    use ferroboard_core::layouts::{ComponentName, KeyboardResources, Subtype, SubtypeLayoutMap};
    use ferroboard_core::text::locale_text::LocaleText;
    let a = Rc::new(KeyboardSettings::default());
    a.set_number_row(true);
    a.set_key_height(42.0);
    a.set_theme_mode(ThemeMode::Dark);
    let b = Rc::new(KeyboardSettings::default());
    b.set_key_height(87.0);
    b.set_theme_mode(ThemeMode::Light);
    let ma = LanguageModels::new();
    let mb = LanguageModels::new();
    let context = Rc::new(SettingsPanelContext::new(a.clone(), ma.clone()));
    context.retarget(None);
    let mut old = None;
    let mut language = None;
    let mut cases = 0;
    for line in include_str!("../../tests/fixtures/settings-panel-lifecycle.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        match row["operation"].as_str().unwrap() {
            "constructor" => assert!(context.try_settings().is_err()),
            "attach-a" => {
                context.retarget(Some((a.clone(), ma.clone())));
                old = Some(MainViewModel::with_context(
                    context.clone(),
                    ThemeChoices::default(),
                ));
                let resources = KeyboardResources::create_empty();
                let preset = Subtype::new("en-US".into());
                preset.set_display_name(Some("alpha".into()));
                preset.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
                    ferroboard_core::layouts::SubtypeLayoutMapValues {
                        characters: ComponentName::new("test", "alpha"),
                        ..SubtypeLayoutMap::default().values()
                    },
                ))));
                resources.add_subtype_preset(Arc::new(preset));
                let sort = LocaleText::new("").unwrap();
                let catalog = LanguageCatalog::snapshot(&resources, |a, b| {
                    sort.compare(a, b, true).map_err(|e| e.to_string())
                })
                .unwrap();
                let search = LocaleText::new("").unwrap();
                language = Some(
                    LanguagePanelViewModel::with_settings_source(
                        context.settings_source(),
                        catalog,
                        Rc::new(move |a, b| search.contains(a, b, true).map_err(|e| e.to_string())),
                    )
                    .unwrap(),
                );
            }
            "attach-b" | "attach-b-again" => context.retarget(Some((b.clone(), mb.clone()))),
            "old-a-setting" => a.set_font_scale(1.4),
            "current-b-setting" => b.set_font_scale(1.3),
            "b-two-refreshes" => b.set_font_scale(1.2),
            "old-toggle-a" => old.as_ref().unwrap().settings().set_number_row(true),
            "old-slider-a" => old.as_ref().unwrap().set_key_height_value(61.25),
            "set-b-raw" => b.set_key_height(24.0),
            "old-slider-a-to-boundary" => old.as_ref().unwrap().set_key_height_value(36.0),
            "old-combo-a" => old.as_ref().unwrap().set_theme_mode_index(2),
            "old-language-a" => language.as_ref().unwrap().rows()[0]
                .dispatch_checked(true)
                .unwrap(),
            "old-language-filter-a" => {
                language
                    .as_ref()
                    .unwrap()
                    .text_changed(Some("alpha".into()))
                    .unwrap();
                assert_eq!(
                    language.as_ref().unwrap().rows()[0].is_checked(),
                    Some(true)
                );
            }
            "old-forget-a" => {
                ma.get_user_dictionary(&"en".into())
                    .learn(&"keepword".into(), None);
                mb.get_user_dictionary(&"en".into())
                    .learn(&"keepword".into(), None);
                old.as_ref().unwrap().clear_learned_words().execute(None);
            }
            "old-reset-a" => old.as_ref().unwrap().reset_settings().execute(None),
            "attach-null" => context.retarget(None),
            "old-toggle-after-null" => assert!(context.try_settings().is_err()),
            "recover-a" => context.retarget(Some((a.clone(), ma.clone()))),
            "a-duplicate-refresh" => a.set_font_scale(1.1),
            operation => panic!("unknown source operation {operation}"),
        }
        // Deliberately compare selected target/mutation projections only: no fake panel UI or
        // subscriptions are constructed, and no whole-lifecycle parity assertion is made.
        for (actual, expected) in [
            (snapshot(&a, &ma), &row["a"]),
            (snapshot(&b, &mb), &row["b"]),
        ] {
            assert_eq!(
                actual["keyHeight"].as_f64().unwrap().to_bits(),
                expected["keyHeight"].as_f64().unwrap().to_bits(),
                "exact source double bits: {row}"
            );
            for key in ["numberRow", "theme", "subtypes", "words"] {
                assert_eq!(actual[key], expected[key], "{key}: {row}");
            }
        }
        cases += 1;
    }
    assert_eq!(cases, 20);
}
#[test]
fn shared_refresh_flag_guard_order_unwinding_and_retained_commands_do_not_depend_on_old_vm() {
    use ferroui_system::component_model::INotifyPropertyChanged;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let a = Rc::new(KeyboardSettings::default());
    let b = Rc::new(KeyboardSettings::default());
    let ma = LanguageModels::new();
    let mb = LanguageModels::new();
    let context = Rc::new(SettingsPanelContext::new(a.clone(), ma.clone()));
    let first = MainViewModel::with_context(context.clone(), ThemeChoices::default());
    let reset = first.reset_settings();
    let forget = first.clear_learned_words();
    let weak = Rc::downgrade(&first);
    context.retarget(Some((b.clone(), mb.clone())));
    let second = MainViewModel::with_context(context.clone(), ThemeChoices::default());
    let captured = first.clone();
    let guarded = context.clone();
    let token = second.property_changed().add(Rc::new(move |_| {
        assert!(guarded.is_refreshing());
        captured.set_key_height_value(77.0);
        captured.set_theme_mode_index(2);
    }));
    second.refresh_presentation();
    second.property_changed().remove(token);
    assert_eq!(b.key_height(), 54.0);
    assert_eq!(b.theme_mode(), ThemeMode::FollowSystem);
    {
        let _outer = context.begin_refresh();
        assert!(context.is_refreshing());
        {
            let _inner = context.begin_refresh();
            assert!(context.is_refreshing());
        }
        assert!(
            !context.is_refreshing(),
            "source finally resets its Boolean during nested refresh"
        );
    }
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            let _guard = context.begin_refresh();
            panic!("observer failed");
        }))
        .is_err()
    );
    assert!(!context.is_refreshing());
    context.retarget(None);
    first.set_theme_mode_index(-1);
    first.set_light_theme_index(-1);
    {
        let _guard = context.begin_refresh();
        first.set_theme_mode_index(2);
        first.set_key_height_value(77.0);
    }
    context.retarget(Some((b.clone(), mb.clone())));
    drop(first);
    assert!(weak.upgrade().is_none());
    a.set_number_row(true);
    b.set_number_row(true);
    ma.get_user_dictionary(&"en".into())
        .learn(&"keepword".into(), None);
    mb.get_user_dictionary(&"en".into())
        .learn(&"keepword".into(), None);
    forget.execute(None);
    assert_eq!(
        ma.get_user_dictionary(&"en".into())
            .get_count(&"keepword".into()),
        1
    );
    assert_eq!(
        mb.get_user_dictionary(&"en".into())
            .get_count(&"keepword".into()),
        0
    );
    reset.execute(None);
    assert!(a.number_row());
    assert!(!b.number_row());
    // Dynamic settings access must not borrow the context across a reentrant settings callback.
    let target = context.clone();
    let old = a.clone();
    let models = ma.clone();
    let events = Rc::new(RefCell::new(0));
    let observed = events.clone();
    let token = b.property_changed().add(Rc::new(move |_| {
        *observed.borrow_mut() += 1;
        target.retarget(Some((old.clone(), models.clone())));
    }));
    second.set_key_height_value(65.0);
    assert_eq!(b.key_height(), 65.0);
    assert!(Rc::ptr_eq(&context.settings(), &a));
    assert_eq!(*events.borrow(), 1);
    b.property_changed().remove(token);
}

#[test]
fn releasing_old_target_services_can_reenter_the_context_without_a_borrow_failure() {
    use ferroui_system::component_model::INotifyPropertyChanged;
    use std::{cell::Cell, rc::Weak};
    struct Probe {
        context: Weak<SettingsPanelContext>,
        expected: Weak<KeyboardSettings>,
        called: Rc<Cell<bool>>,
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            let context = self.context.upgrade().unwrap();
            let expected = self.expected.upgrade().unwrap();
            assert!(Rc::ptr_eq(&context.try_settings().unwrap(), &expected));
            self.called.set(true);
        }
    }
    let a = Rc::new(KeyboardSettings::default());
    let b = Rc::new(KeyboardSettings::default());
    let context = Rc::new(SettingsPanelContext::new(a.clone(), LanguageModels::new()));
    let called = Rc::new(Cell::new(false));
    let probe = Probe {
        context: Rc::downgrade(&context),
        expected: Rc::downgrade(&b),
        called: called.clone(),
    };
    a.property_changed().add(Rc::new(move |_| {
        let _ = &probe;
    }));
    drop(a);
    context.retarget(Some((b, LanguageModels::new())));
    assert!(called.get());
}
