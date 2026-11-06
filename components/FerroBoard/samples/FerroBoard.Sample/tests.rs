use crate::{App, composition::SampleSession};
use ferroui_base::{Point, Ref, Visual, threading::Dispatcher};
use ferroui_controls::{AppBuilder, Button, TextBox};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
    HeadlessWindowExtensions,
};
use ferroui_system::{component_model::INotifyPropertyChanged, threading::CancellationToken};
use std::{
    rc::Rc,
    sync::{Arc, OnceLock},
};
pub(crate) fn run(f: impl FnOnce() + Send + 'static) {
    static SESSION: OnceLock<Arc<HeadlessUnitTestSession>> = OnceLock::new();
    SESSION
        .get_or_init(|| {
            HeadlessUnitTestSession::start_new(|| {
                AppBuilder::configure::<App>().use_headless(FerroHeadlessPlatformOptions::default())
            })
        })
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap();
}
fn find_visual(root: &Ref<Visual>, name: &str) -> Option<Ref<Visual>> {
    if root.get_type().name() == name {
        return Some(root.clone());
    }
    for index in 0..root.visual_children().count() {
        let child = root.visual_children().get(index);
        if let Some(v) = find_visual(&child, name) {
            return Some(v);
        }
    }
    None
}

#[test]
fn hosted_keyboards_restore_their_own_field_when_windows_reactivate() {
    run(|| {
        let a = SampleSession::new(crate::storage_tests::file_storage());
        let b = SampleSession::new(crate::storage_tests::file_storage());
        a.window.show();
        b.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let first = crate::test_ui::ui(&a)
            .find("MessageBox")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        let second = crate::test_ui::ui(&b)
            .find("NameBox")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        assert!(first.focus());
        assert!(second.focus());
        Dispatcher::ui_thread().run_jobs(None);
        let scope_a = a
            .window
            .focus_manager()
            .get_focused_element_in_scope(&a.window)
            .and_then(|e| e.name());
        let scope_b = b
            .window
            .focus_manager()
            .get_focused_element_in_scope(&b.window)
            .and_then(|e| e.name());
        let is_scope = a.window.is_focus_scope();
        a.window.activate();
        Dispatcher::ui_thread().run_jobs(None);
        let restored_a = first.is_focused();
        let attached_a = a.keyboard.target_element() == Some(first.clone().upcast());
        b.window.activate();
        Dispatcher::ui_thread().run_jobs(None);
        let restored_b = second.is_focused();
        let attached_b = b.keyboard.target_element() == Some(second.clone().upcast());
        a.window.close();
        b.window.close();
        a.release();
        b.release();
        assert!(
            is_scope,
            "sample Window must inherit the native focus-scope override"
        );
        assert!(
            restored_a,
            "A did not restore its own field; scope A={scope_a:?}, scope B={scope_b:?}"
        );
        assert!(
            restored_b,
            "B did not restore its own field; scope A={scope_a:?}, scope B={scope_b:?}"
        );
        assert!(
            attached_a,
            "A keyboard must follow its restored local field"
        );
        assert!(
            attached_b,
            "B keyboard must follow its restored local field"
        );
    });
}
#[test]
fn compiled_sample_has_original_settings_reactive_bindings_and_real_reset_command() {
    run(|| {
        let s = SampleSession::new(crate::storage_tests::file_storage());
        s.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&s);
        let message = scope.find("MessageBox").unwrap().cast::<TextBox>().unwrap();
        assert!(Rc::ptr_eq(
            &s.bindings.settings(),
            &s.keyboard.engine().settings()
        ));
        message.set_text(Some("external 😀"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(s.bindings.model().message().as_deref(), Some("external 😀"));
        s.bindings
            .model()
            .set_message(Some("reactive change".into()));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(message.text().as_deref(), Some("reactive change"));
        let check = scope
            .find("NumberRowToggle")
            .unwrap()
            .cast::<ferroui_controls::primitives::ToggleButton>()
            .unwrap();
        check.set_current_value(
            ferroui_controls::primitives::ToggleButton::is_checked_property(),
            Some(true),
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(s.keyboard.settings().number_row());
        let reset = scope.find("ResetButton").unwrap().cast::<Button>().unwrap();
        assert!(
            reset.command().is_none(),
            "source uses a direct Click callback"
        );
        let previous_status = s.bindings.status();
        reset.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            Button::click_event(),
        ));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!s.keyboard.settings().number_row());
        assert_eq!(check.is_checked(), Some(false));
        assert_eq!(
            s.bindings.status(),
            previous_status,
            "source Reset does not change StatusText"
        );
        s.window.close();
        s.release();
        assert!(s.keyboard.engine().target().is_none());
    });
}
#[test]
fn compiled_sample_raw_keyboard_pointer_preserves_focus_and_password_privacy() {
    run(|| {
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        let s = SampleSession::new(crate::storage_tests::file_storage());
        // The host's animated opening starts below the viewport. This test exercises pointer
        // input after placement; deterministic host-clock tests cover the animation itself.
        s.host.set_is_animated(false);
        s.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&s);
        let message = scope.find("MessageBox").unwrap().cast::<TextBox>().unwrap();
        assert!(message.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(s.keyboard.target_element(), Some(message.clone().upcast()));
        let engine = s.keyboard.engine();
        let key = engine
            .keyboard()
            .unwrap()
            .keys()
            .map(|key| key.unwrap().unwrap())
            .find(|k| k.code() == 81)
            .unwrap();
        let b = key.visible_bounds();
        let keys = find_visual(&s.keyboard.clone().upcast(), "KeysView").unwrap();
        let p = keys
            .translate_point(
                Point::new(b.x + b.width / 2.0, b.y + b.height / 2.0),
                &s.window,
            )
            .unwrap();
        s.window
            .mouse_down(p, MouseButton::Left, RawInputModifiers::NONE);
        s.window
            .mouse_up(p, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(message.text().as_deref(), Some("Q"));
        assert_eq!(s.bindings.message().as_deref(), Some("Q"));
        assert!(message.is_focused());
        let password = scope
            .find("PasswordField")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        assert!(password.focus());
        Dispatcher::ui_thread().run_jobs(None);
        let attrs = engine.attributes();
        assert!(attrs.is_password());
        assert!(attrs.is_private());
        assert!(!attrs.supports_nlp());
        engine.type_text(&"secret".into()).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(password.text().as_deref(), Some("secret"));
        assert!(password.is_focused());
        assert_eq!(message.text().as_deref(), Some("Q"));
        s.window.close();
        s.release();
    });
}

#[test]
fn native_command_adapter_preserves_parameter_and_can_execute_subscription_disposal() {
    use ferroui_base::metadata::into_markup_value;
    use ferroui_reactive::concurrency::ImmediateScheduler;
    use ferroui_reactiveui::ReactiveCommand;
    use std::cell::Cell;
    let value = Rc::new(Cell::new(0));
    let target = value.clone();
    let original = Rc::new(ReactiveCommand::create_param_with(
        move |parameter: i32| target.set(parameter),
        None,
        Some(ImmediateScheduler::instance()),
    ));
    let adapter = crate::native_command::NativeCommand::wrap(original.clone());
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let token = adapter.can_execute_changed(Rc::new(move || observed.set(observed.get() + 1)));
    let parameter = into_markup_value(42i32).unwrap();
    assert!(adapter.can_execute(Some(&parameter)));
    adapter.execute(Some(&parameter));
    assert_eq!(value.get(), 42);
    assert!(changes.get() > 0);
    token.dispose();
    let before = changes.get();
    adapter.execute(Some(&parameter));
    assert_eq!(changes.get(), before);
    ferroui_system::IDisposable::dispose(&*original);
}

#[test]
fn sample_persistence_saves_on_source_notifications_and_release_then_detaches() {
    run(|| {
        use crate::sample_storage::{FileSampleStoragePlatform, SETTINGS_KEY, SampleStorage};
        let folder = crate::storage_tests::unique_folder();
        let s = SampleSession::new(SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(
            folder.clone(),
        ))));
        s.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let settings = s.keyboard.settings();
        let history = s.keyboard.engine().emoji_history();
        let settings_handlers = settings.property_changed().handler_count();
        let emoji_handlers = history.changed().handler_count();
        // Startup load does not install autosave until loading finishes.
        assert!(!folder.join(SETTINGS_KEY).exists());
        settings.set_number_row(true);
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(SETTINGS_KEY)).unwrap())
                .unwrap();
        assert_eq!(saved["numberRow"], true);
        let clipboard = s.keyboard.engine().clipboard_history();
        let pinned = clipboard
            .add(Some(&"pinned native".into()))
            .unwrap()
            .unwrap();
        clipboard.set_pinned(&pinned, true);
        clipboard.add(Some(&"transient native".into())).unwrap();
        assert!(!folder.join("clipboard.json").exists());
        history.add(&"😀".into()).unwrap();
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join("clipboard.json")).unwrap())
                .unwrap();
        assert_eq!(saved.as_array().unwrap().len(), 1);
        assert_eq!(saved[0]["text"], "pinned native");
        // Changes to learned words save on the source's unload trigger, not a new asynchronous path.
        let dictionary_before = std::fs::read(folder.join("user-dictionary-en.json")).unwrap();
        s.keyboard
            .engine()
            .language_models()
            .get_user_dictionary(&"en".into())
            .learn(&"learnedfixture".into(), None);
        assert_eq!(
            std::fs::read(folder.join("user-dictionary-en.json")).unwrap(),
            dictionary_before
        );
        s.window.close();
        s.release();
        let saved: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(folder.join("user-dictionary-en.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(saved["words"]["learnedfixture"], 1);
        assert!(settings.property_changed().handler_count() < settings_handlers);
        // Native owner/panel teardown also removes its own history subscriptions.
        assert!(history.changed().handler_count() < emoji_handlers);
        let settings_before = std::fs::read(folder.join(SETTINGS_KEY)).unwrap();
        let emoji_before = std::fs::read(folder.join("emoji-history.json")).unwrap();
        settings.set_number_row(false);
        history.add(&"👍".into()).unwrap();
        s.release();
        assert_eq!(
            std::fs::read(folder.join(SETTINGS_KEY)).unwrap(),
            settings_before
        );
        assert_eq!(
            std::fs::read(folder.join("emoji-history.json")).unwrap(),
            emoji_before
        );
        let reopened = SampleSession::new(SampleStorage::new(Rc::new(
            FileSampleStoragePlatform::new(folder),
        )));
        reopened.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&reopened);
        let toggle = scope
            .find("NumberRowToggle")
            .unwrap()
            .cast::<ferroui_controls::primitives::ToggleButton>()
            .unwrap();
        assert_eq!(toggle.is_checked(), Some(true));
        assert!(reopened.keyboard.settings().number_row());
        assert!(!Rc::ptr_eq(&settings, &reopened.keyboard.settings()));
        assert_eq!(
            reopened
                .keyboard
                .engine()
                .emoji_history()
                .recent()
                .lock()
                .clone(),
            vec![Some("😀".into())]
        );
        assert_eq!(
            reopened
                .keyboard
                .engine()
                .language_models()
                .get_user_dictionary(&"en".into())
                .words()
                .get(&"learnedfixture".into()),
            Some(&1)
        );
        reopened.window.close();
        reopened.release();
    });
}

#[test]
fn startup_options_override_storage_before_bindings_and_apply_loaded_focus_and_settings_flags() {
    run(|| {
        use crate::{
            sample_options::SampleOptions,
            sample_storage::{FileSampleStoragePlatform, SETTINGS_KEY, SampleStorage},
        };
        use ferroui_controls::{SplitView, SplitViewDisplayMode};
        let folder = crate::storage_tests::unique_folder();
        std::fs::write(
            folder.join(SETTINGS_KEY),
            r#"{"numberRow":false,"keyHeight":70}"#,
        )
        .unwrap();
        let options = Arc::new(SampleOptions::default());
        options.parse(Some(&"--numberrow --keyheight=61.25 --glide=false --settings=false --focus=EmailBox --longpress=450 --onehanded=right --langs=en-US/qwerty,de-DE/qwertz".into()));
        let session = SampleSession::with_options(
            SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(folder.clone()))),
            options,
        )
        .unwrap();
        // Startup application subscriptions are installed after Apply, so overrides are not yet saved.
        let before: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(SETTINGS_KEY)).unwrap())
                .unwrap();
        assert_eq!(before["keyHeight"], 70);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&session);
        let email = scope.find("EmailBox").unwrap().cast::<TextBox>().unwrap();
        assert!(email.is_focused());
        assert_eq!(session.keyboard.target_element(), Some(email.upcast()));
        let split = scope.find("Split").unwrap().cast::<SplitView>().unwrap();
        assert!(!split.is_pane_open());
        assert_eq!(split.display_mode(), SplitViewDisplayMode::Inline);
        let toggle = scope
            .find("NumberRowToggle")
            .unwrap()
            .cast::<ferroui_controls::primitives::ToggleButton>()
            .unwrap();
        assert_eq!(toggle.is_checked(), Some(true));
        let settings = session.keyboard.settings();
        assert_eq!(settings.key_height(), 61.25);
        assert!(!settings.glide_typing());
        assert_eq!(settings.long_press_delay(), 450);
        assert_eq!(settings.one_handed_mode().0, 2);
        assert_eq!(settings.subtypes().borrow().len(), 2);
        session
            .keyboard
            .settings_requested()
            .raise(&session.keyboard);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(split.is_pane_open());
        session.window.set_width(899.0);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(split.display_mode(), SplitViewDisplayMode::Overlay);
        assert!(split.is_pane_open());
        session.window.set_width(900.0);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(split.display_mode(), SplitViewDisplayMode::Inline);
        assert!(split.is_pane_open());
        session.window.close();
        session.release();
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(folder.join(SETTINGS_KEY)).unwrap())
                .unwrap();
        assert_eq!(saved["keyHeight"], 61.25);
    });
}

#[test]
fn source_initial_narrow_pane_rule_and_exact_width_boundary_are_preserved() {
    run(|| {
        use crate::sample_options::SampleOptions;
        use ferroui_controls::{SplitView, SplitViewDisplayMode};
        let options = Arc::new(SampleOptions::default());
        options.parse(Some(&"--settings=FALSE --focus=MissingField".into()));
        let session =
            SampleSession::with_options(crate::storage_tests::file_storage(), options).unwrap();
        session.window.set_width(899.0);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&session);
        let split = scope.find("Split").unwrap().cast::<SplitView>().unwrap();
        assert_eq!(split.display_mode(), SplitViewDisplayMode::Overlay);
        assert!(!split.is_pane_open());
        assert!(
            scope
                .find("MessageBox")
                .unwrap()
                .cast::<TextBox>()
                .unwrap()
                .is_focused()
        );
        session.window.close();
        session.release();
    });
    // Presentation rule uses the original strict < comparison, without rounding dimensions.
    let vm = crate::view_model::MainViewModel::new(
        Rc::new(ferroboard_core::settings::KeyboardSettings::default()),
        ferroboard_core::engine::LanguageModels::new(),
        crate::theme_choices::ThemeChoices::default(),
    );
    vm.update_width(899.999999, 0.0);
    assert!(vm.narrow());
    assert!(!vm.settings_pane_open());
    vm.set_settings_pane_open(true);
    vm.update_width(900.0, 899.999999);
    assert!(!vm.narrow());
    assert!(vm.settings_pane_open());
    vm.update_width(899.999999, 900.0);
    assert!(vm.narrow());
    assert!(vm.settings_pane_open());
}

#[test]
fn invalid_startup_options_return_failure_without_saving_a_partial_session() {
    run(|| {
        use crate::{
            sample_options::{OptionErrorKind, SampleOptions},
            sample_storage::{FileSampleStoragePlatform, SETTINGS_KEY, SampleStorage},
        };
        let folder = crate::storage_tests::unique_folder();
        let original = br#"{"keyHeight":70}"#;
        std::fs::write(folder.join(SETTINGS_KEY), original).unwrap();
        let options = Arc::new(SampleOptions::default());
        options.parse(Some(
            &"--theme=dark --split=invalid --keyheight=61.25".into(),
        ));
        let error = SampleSession::with_options(
            SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(folder.clone()))),
            options,
        )
        .err()
        .expect("invalid boolean fails construction");
        assert_eq!(error.option, "split");
        assert_eq!(error.kind, OptionErrorKind::Format);
        assert_eq!(std::fs::read(folder.join(SETTINGS_KEY)).unwrap(), original);
        assert!(!folder.join("emoji-history.json").exists());
    });
}

#[test]
fn compiled_host_controls_visibility_viewport_floating_and_same_field_reopen() {
    run(|| {
        use ferroboard_controls::{KeyboardVisibilityMode, ScreenKeyboardHost};
        use ferroui_base::input::{MouseButton, RawInputModifiers};
        let session = SampleSession::new(crate::storage_tests::file_storage());
        let scope = crate::test_ui::ui(&session);
        let host = scope
            .find("Host")
            .unwrap()
            .cast::<ScreenKeyboardHost>()
            .unwrap();
        assert_eq!(host, session.host);
        assert_eq!(host.keyboard(), session.keyboard);
        assert_eq!(host.visibility_mode(), KeyboardVisibilityMode::Auto);
        assert!(host.is_animated());
        assert!(!session.keyboard.auto_attach());
        assert!(!host.is_keyboard_open());
        assert!(!session.keyboard.is_visible());
        assert_eq!(host.keyboard().visual_parent(), Some(host.clone().upcast()));
        host.set_is_animated(false);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let message = scope.find("MessageBox").unwrap().cast::<TextBox>().unwrap();
        let disabled = scope
            .find("DisabledKeyboardBox")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        assert!(host.is_keyboard_open());
        assert!(session.keyboard.is_visible());
        assert!(message.is_focused());
        let content = host.content().unwrap();
        let docked_height = content.bounds().height;
        assert!(docked_height > 0.0);
        assert!(docked_height < host.bounds().height);
        assert!(disabled.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!host.is_keyboard_open());
        assert!(!session.keyboard.is_visible());
        assert!(session.keyboard.target_element().is_none());
        assert!(content.bounds().height > docked_height);
        assert!(message.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(host.is_keyboard_open());
        host.set_is_keyboard_open(false);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(message.is_focused());
        let p = message
            .translate_point(Point::new(8.0, 8.0), &session.window)
            .unwrap();
        session
            .window
            .mouse_down(p, MouseButton::Left, RawInputModifiers::NONE);
        session
            .window
            .mouse_up(p, MouseButton::Left, RawInputModifiers::NONE);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(host.is_keyboard_open());
        assert!(message.is_focused());
        session.keyboard.settings().set_floating(true);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(host.is_floating());
        assert_ne!(
            session.keyboard.visual_parent(),
            Some(host.clone().upcast())
        );
        assert!(content.bounds().height > docked_height);
        assert!(message.is_focused());
        session.keyboard.settings().set_floating(false);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!host.is_floating());
        assert_eq!(
            session.keyboard.visual_parent(),
            Some(host.clone().upcast())
        );
        assert!(host.is_keyboard_open());
        host.set_visibility_mode(KeyboardVisibilityMode::AlwaysVisible);
        let engine = session.keyboard.engine();
        engine.hide_requested().raise(&engine);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(host.is_keyboard_open());
        host.set_visibility_mode(KeyboardVisibilityMode::Manual);
        host.set_is_keyboard_open(false);
        let name = scope.find("NameBox").unwrap().cast::<TextBox>().unwrap();
        assert!(name.focus());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!host.is_keyboard_open());
        assert_eq!(
            session.keyboard.target_element(),
            Some(name.clone().upcast())
        );
        session.window.close();
        session.release();
        assert!(session.keyboard.target_element().is_none());
        assert_eq!(
            session
                .keyboard
                .settings()
                .property_changed()
                .handler_count(),
            0
        );
    });
}

#[test]
fn hosted_sample_uses_native_scrolling_without_capability_errors() {
    run(|| {
        let session = SampleSession::new(crate::storage_tests::file_storage());
        session.host.set_is_animated(false);
        let failures = Rc::new(std::cell::RefCell::new(Vec::new()));
        let observed = failures.clone();
        let token = session
            .host
            .integration_failed()
            .add(Rc::new(move |e| observed.borrow_mut().push(*e)));
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(failures.borrow().is_empty());
        session.host.integration_failed().remove(token);
        session.window.close();
        session.release();
    });
}

#[test]
fn compiled_application_theme_and_explicit_resources_keep_original_identity_across_windows() {
    run(|| {
        use crate::{App, composition::SampleComposition, sample_options::SampleOptions};
        use ferroboard_controls::themes::{KeyboardTheme, KeyboardThemes};
        use ferroboard_core::settings::ThemeMode;
        let app = ferroui_controls::Application::current()
            .unwrap()
            .cast::<App>()
            .unwrap();
        let theme = ferroui_base::metadata::from_markup_value::<Arc<KeyboardTheme>>(
            &app.resources().get(&"SunsetTheme".into()),
        )
        .unwrap();
        assert!(Arc::ptr_eq(
            &theme,
            &KeyboardThemes::get("sample_sunset").unwrap()
        ));
        let source = include_str!("../../tests/fixtures/sample-resources.jsonl")
            .lines()
            .map(|l| serde_json::from_str::<serde_json::Value>(l).unwrap())
            .find(|r| r["kind"] == "theme")
            .unwrap();
        assert_eq!(source["registeredIdentity"], true);
        let actual: serde_json::Value = serde_json::from_str(&theme.to_json().unwrap()).unwrap();
        assert_eq!(
            crate::storage_tests::numeric_values(actual),
            crate::storage_tests::numeric_values(source["json"].clone())
        );
        let options = Arc::new(SampleOptions::default());
        options.parse(Some(
            &"--langs=en-US/alphabetical --glide=false --theme=dark --darktheme=sample_sunset"
                .into(),
        ));
        let first =
            SampleSession::with_options(crate::storage_tests::file_storage(), options.clone())
                .unwrap();
        let second =
            SampleSession::with_options(crate::storage_tests::file_storage(), options).unwrap();
        let resources = SampleComposition::resources().unwrap();
        assert!(Rc::ptr_eq(&resources, &first.keyboard.engine().resources()));
        assert!(Rc::ptr_eq(
            &resources,
            &second.keyboard.engine().resources()
        ));
        assert!(Arc::ptr_eq(&theme, &first.keyboard.effective_theme()));
        assert_eq!(first.keyboard.settings().theme_mode(), ThemeMode::Dark);
        assert_eq!(
            first.keyboard.engine().active_subtype().id(),
            "en-US/alphabetical".into()
        );
        let first_changed = Rc::new(std::cell::Cell::new(0));
        let observed = first_changed.clone();
        let first_token = first
            .keyboard
            .engine()
            .keyboard_changed()
            .add(Rc::new(move |_| observed.set(observed.get() + 1)));
        let second_changed = Rc::new(std::cell::Cell::new(0));
        let observed = second_changed.clone();
        let second_token = second
            .keyboard
            .engine()
            .keyboard_changed()
            .add(Rc::new(move |_| observed.set(observed.get() + 1)));
        crate::custom_layouts::register(&resources).unwrap();
        assert!(first_changed.get() > 0);
        assert!(second_changed.get() > 0);
        first
            .keyboard
            .engine()
            .keyboard_changed()
            .remove(first_token);
        second
            .keyboard
            .engine()
            .keyboard_changed()
            .remove(second_token);
        first.release();
        second.release();
        assert_eq!(resources.changed().handler_count(), 0);
    });
}

#[test]
fn source_boolean_slider_settings_and_clear_learning_command_reach_original_engine() {
    run(|| {
        use ferroui_controls::{
            Slider,
            primitives::{RangeBase, ToggleButton},
        };
        let session = SampleSession::new(crate::storage_tests::file_storage());
        session.host.set_is_animated(false);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&session);
        let settings = session.keyboard.settings();
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../docs/sample-settings-controls.json")).unwrap();
        let mut toggles = 0;
        let mut sliders = 0;
        for item in manifest["controls"].as_array().unwrap() {
            if item["kind"] != "toggle" {
                continue;
            }
            let name = item["property"].as_str().unwrap();
            let mut key = name.to_owned();
            key.replace_range(..1, &name[..1].to_ascii_lowercase());
            let before: serde_json::Value =
                serde_json::from_str(&settings.to_json().unwrap()).unwrap();
            let expected = !before[&key].as_bool().unwrap();
            let control_name = if name == "NumberRow" {
                "NumberRowToggle".to_owned()
            } else {
                format!("{name}Toggle")
            };
            let control = scope
                .find(&control_name)
                .unwrap()
                .cast::<ToggleButton>()
                .unwrap();
            control.set_current_value(ToggleButton::is_checked_property(), Some(expected));
            Dispatcher::ui_thread().run_jobs(None);
            let after: serde_json::Value =
                serde_json::from_str(&settings.to_json().unwrap()).unwrap();
            assert_eq!(after[&key], expected, "{name}");
            toggles += 1;
        }
        for (name, value) in [
            ("KeyHeight", 63.0),
            ("FontScale", 1.25),
            ("KeySpacingHorizontal", 7.0),
            ("KeySpacingVertical", 11.0),
            ("SuggestionCount", 5.0),
            ("LongPressDelay", 480.0),
        ] {
            let control = scope
                .find(&format!("{name}Slider"))
                .unwrap()
                .cast::<Slider>()
                .unwrap();
            control.set_current_value(RangeBase::value_property(), value);
            Dispatcher::ui_thread().run_jobs(None);
            let mut key = name.to_owned();
            key.replace_range(..1, &name[..1].to_ascii_lowercase());
            let after: serde_json::Value =
                serde_json::from_str(&settings.to_json().unwrap()).unwrap();
            assert_eq!(after[&key].as_f64(), Some(value), "{name}");
            sliders += 1;
        }
        assert_eq!(toggles, 21);
        assert_eq!(sliders, 6);
        // The source panel's refresh guard prevents UI clamping from changing the model value.
        settings.set_key_height(24.0);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            scope
                .find("KeyHeightSlider")
                .unwrap()
                .cast::<Slider>()
                .unwrap()
                .value(),
            36.0
        );
        assert_eq!(settings.key_height(), 24.0);
        settings.set_long_press_delay(100);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            scope
                .find("LongPressDelaySlider")
                .unwrap()
                .cast::<Slider>()
                .unwrap()
                .value(),
            150.0
        );
        assert_eq!(settings.long_press_delay(), 100);
        let models = session.keyboard.engine().language_models();
        for language in ["en", "de"] {
            models
                .get_user_dictionary(&language.into())
                .learn(&"clearfixture".into(), None);
        }
        let button = scope
            .find("ClearLearnedButton")
            .unwrap()
            .cast::<Button>()
            .unwrap();
        assert!(
            button.command().is_none(),
            "source uses a direct Click callback"
        );
        button.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            Button::click_event(),
        ));
        for (_, dictionary) in models.user_dictionaries().iter() {
            assert!(dictionary.words().is_empty());
        }
        session.window.close();
        session.release();
    });
}

#[test]
fn compiled_enum_choices_preserve_source_order_and_unknown_model_values() {
    run(|| {
        use crate::enum_choices::EnumChoice;
        use ferroui_controls::ComboBox;
        let session = SampleSession::new(crate::storage_tests::file_storage());
        session.host.set_is_animated(false);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&session);
        let settings = session.keyboard.settings();
        for choice in EnumChoice::ALL {
            let control = scope
                .find(&format!("{}Choice", choice.property()))
                .unwrap()
                .cast::<ComboBox>()
                .unwrap();
            assert_eq!(control.items().count(), choice.items().len());
            for (index, (name, _)) in choice.items().iter().enumerate() {
                assert_eq!(
                    ferroui_base::metadata::from_markup_value::<String>(
                        &control.items().get_at(index)
                    )
                    .as_deref(),
                    Some(*name)
                );
            }
            for index in [0, (choice.items().len() - 1) as i32] {
                control.set_selected_index(index);
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(
                    choice.value(&settings),
                    choice.items()[index as usize].1,
                    "{}",
                    choice.property()
                );
            }
            choice.set_value(&settings, 99);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(control.selected_index(), -1, "{}", choice.property());
            assert_eq!(choice.value(&settings), 99);
            // A refresh caused by an unrelated setting must not replace the unknown raw enum.
            settings.set_number_row(!settings.number_row());
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(choice.value(&settings), 99);
            assert_eq!(control.selected_index(), -1);
            choice.set_value(&settings, choice.items()[0].1);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(control.selected_index(), 0);
        }
        session.window.close();
        session.release();
    });
}

#[test]
fn compiled_theme_choices_preserve_culture_snapshot_unknown_values_and_apply_shared_themes() {
    run(|| {
        use crate::theme_choices::ThemeProperty;
        use ferroboard_controls::themes::{KeyboardTheme, KeyboardThemes};
        use ferroboard_core::settings::ThemeMode;
        use ferroui_base::{metadata::from_markup_value, utilities::CultureInfo};
        use ferroui_controls::ComboBox;
        struct CultureGuard(CultureInfo);
        impl Drop for CultureGuard {
            fn drop(&mut self) {
                CultureInfo::set_current_formatting(self.0.clone());
            }
        }
        let _guard = CultureGuard(CultureInfo::current_formatting());
        CultureInfo::set_current_formatting(CultureInfo::get_culture_info("sv-SE"));
        let null_label = Arc::new(KeyboardTheme::default());
        null_label.set_id(Some("sample_null_label_test".into()));
        null_label.set_name(None);
        KeyboardThemes::register(null_label.clone()).unwrap();
        let session = SampleSession::new(crate::storage_tests::file_storage());
        assert_eq!(
            CultureInfo::current_formatting().name(),
            "sv-SE",
            "session must preserve a host's culture"
        );
        session.host.set_is_animated(false);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&session);
        let settings = session.keyboard.settings();
        let snapshot = session.bindings.model().themes().items().to_vec();
        assert!(session.bindings.light_theme_items() == session.bindings.light_theme_items());
        assert!(session.bindings.dark_theme_items() == session.bindings.dark_theme_items());
        assert!(session.bindings.light_theme_items() != session.bindings.dark_theme_items());
        for (name, property, mode) in [
            (
                "LightThemeChoice",
                ThemeProperty::LightTheme,
                ThemeMode::Light,
            ),
            ("DarkThemeChoice", ThemeProperty::DarkTheme, ThemeMode::Dark),
        ] {
            let combo = scope.find(name).unwrap().cast::<ComboBox>().unwrap();
            assert_eq!(combo.items().count(), snapshot.len());
            for (index, item) in snapshot.iter().enumerate() {
                assert_eq!(
                    from_markup_value::<String>(&combo.items().get_at(index)),
                    item.name.as_ref().map(|n| n.to_utf8().unwrap())
                );
            }
            let null_index = snapshot
                .iter()
                .position(|t| t.id == Some("sample_null_label_test".into()))
                .unwrap();
            settings.set_theme_mode(mode);
            combo.set_selected_index(null_index as i32);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(property.get(&settings), "sample_null_label_test");
            assert!(Arc::ptr_eq(
                &session.keyboard.effective_theme(),
                &null_label
            ));
            property.set(&settings, Some("missing_source_theme".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(combo.selected_index(), -1);
            settings.set_number_row(!settings.number_row());
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(property.get(&settings), "missing_source_theme");
            assert_eq!(combo.selected_index(), -1);
            let index = snapshot
                .iter()
                .position(|t| t.id == Some("sample_sunset".into()))
                .unwrap();
            settings.set_theme_mode(mode);
            combo.set_selected_index(index as i32);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(property.get(&settings), "sample_sunset");
            assert!(Arc::ptr_eq(
                &session.keyboard.effective_theme(),
                &KeyboardThemes::get("sample_sunset").unwrap()
            ));
            property.set(&settings, Some("SAMPLE_SUNSET".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                combo.selected_index(),
                -1,
                "Combo equality is ordinal and case sensitive"
            );
            assert!(
                Arc::ptr_eq(
                    &session.keyboard.effective_theme(),
                    &KeyboardThemes::get("sample_sunset").unwrap()
                ),
                "registry lookup remains case insensitive"
            );
        }
        let late = Arc::new(KeyboardTheme::default());
        late.set_id(Some("late_sample_theme_snapshot_test".into()));
        late.set_name(Some("Late sample theme".into()));
        KeyboardThemes::register(late.clone()).unwrap();
        late.set_name(Some("Mutated after registration".into()));
        Dispatcher::ui_thread().run_jobs(None);
        settings.set_number_row(!settings.number_row());
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(session.bindings.model().themes().items(), snapshot);
        for name in ["LightThemeChoice", "DarkThemeChoice"] {
            assert_eq!(
                scope
                    .find(name)
                    .unwrap()
                    .cast::<ComboBox>()
                    .unwrap()
                    .items()
                    .count(),
                snapshot.len()
            );
        }
        session.window.close();
        session.release();
    });
}

#[test]
fn compiled_language_template_filter_stale_checks_and_original_engine_settings() {
    run(|| {
        use ferroboard_core::{
            layouts::{ComponentName, Subtype, SubtypeLayoutMap},
            settings::KeyboardSettings,
        };
        use ferroui_controls::{CheckBox, ItemsControl};
        use std::cell::RefCell;
        let session = SampleSession::new(crate::storage_tests::file_storage());
        session.host.set_is_animated(false);
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&session);
        let filter = scope
            .find("LanguageFilter")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        let choices = scope
            .find("LanguageChoices")
            .unwrap()
            .cast::<ItemsControl>()
            .unwrap();
        let binding = session.bindings.languages();
        let model = binding.model();
        let source = binding.items();
        assert!(source == binding.items());
        assert_eq!(model.rows_changed().handler_count(), 1);
        let total = model.rows().len();
        assert!(total >= 74, "all bundled presets plus sample ABC");
        assert_eq!(choices.items().count(), total);
        filter.set_text(Some("  ABC  "));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(choices.items().count(), 1);
        assert_eq!(model.rows()[0].id(), &"en-US/alphabetical".into());
        let container = choices
            .container_from_index(0)
            .expect("realized source-style row");
        let check = find_visual(&container.clone().upcast::<Visual>(), "LanguageCheckBox")
            .unwrap()
            .cast::<CheckBox>()
            .unwrap();
        let original = model.rows()[0].clone();
        let settings = session.keyboard.settings();
        // An external settings replacement does not synchronize the existing row. The next filter
        // event recreates rows from the then-current settings, exactly as source Populate does.
        settings.set_subtypes(Some(Rc::new(RefCell::new(vec![Some(
            "en-US/alphabetical".into(),
        )]))));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(check.is_checked(), Some(false));
        assert_eq!(original.is_checked(), Some(false));
        let before_checked = settings.subtypes();
        check.set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            Rc::ptr_eq(&before_checked, &settings.subtypes()),
            "checking an already-enabled id preserves its list identity"
        );
        assert_eq!(
            *settings.subtypes().borrow(),
            vec![Some("en-US/alphabetical".into())]
        );
        assert_eq!(
            session.keyboard.engine().active_subtype().id(),
            "en-US/alphabetical".into()
        );
        let before_indeterminate = settings.subtypes();
        check.set_is_checked(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(original.is_checked(), None);
        assert!(Rc::ptr_eq(&before_indeterminate, &settings.subtypes()));
        check.set_is_checked(Some(false));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            *settings.subtypes().borrow(),
            vec![Some("en-US/qwerty".into())]
        );
        assert_eq!(
            session.keyboard.engine().active_subtype().id(),
            "en-US/qwerty".into()
        );
        filter.set_text(Some("no such language"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(choices.items().count(), 0);
        // Removed row objects retain their captured id/settings callback if a caller retains them.
        check.set_is_checked(Some(true));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            original.is_checked(),
            Some(true),
            "retained removed native checkbox keeps its captured callback"
        );
        assert_eq!(
            *settings.subtypes().borrow(),
            vec![
                Some("en-US/qwerty".into()),
                Some("en-US/alphabetical".into())
            ]
        );
        let late = Subtype::new("en-US".into());
        late.set_display_name(Some("Late sample language".into()));
        late.set_layouts(Some(Arc::new(SubtypeLayoutMap::from_values(
            ferroboard_core::layouts::SubtypeLayoutMapValues {
                characters: ComponentName::new("test", "late-language-test"),
                ..SubtypeLayoutMap::default().values()
            },
        ))));
        session
            .keyboard
            .engine()
            .resources()
            .add_subtype_preset(Arc::new(late));
        filter.set_text(Some(""));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            choices.items().count(),
            total,
            "catalog remains the Build snapshot"
        );
        assert!(source == binding.items());
        let row = model
            .rows()
            .iter()
            .find(|r| r.id() == &"en-US/alphabetical".into())
            .unwrap()
            .clone();
        assert_eq!(row.is_checked(), Some(true));
        assert!(!Rc::ptr_eq(&row, &original));
        let retained = check.downgrade();
        drop(check);
        drop(container);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            retained.upgrade().is_none(),
            "removed control must not be retained by its model event subscription"
        );
        assert_eq!(
            original.property_changed().handler_count(),
            0,
            "control drop removes its persistent row subscription"
        );
        // Settings reset also leaves currently displayed checks untouched; filter refresh observes it.
        settings.copy_from(&KeyboardSettings::default()).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(row.is_checked(), Some(true));
        filter.set_text(Some("ABC"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(model.rows()[0].is_checked(), Some(false));
        session.window.close();
        session.release();
        assert_eq!(model.rows_changed().handler_count(), 0);
    });
}

#[test]
fn retained_native_language_callback_uses_retargeted_services_and_ignores_scalar_refresh_guard() {
    run(|| {
        use ferroui_controls::{CheckBox, ItemsControl};
        let first = SampleSession::new(crate::storage_tests::file_storage());
        let second = SampleSession::new(crate::storage_tests::file_storage());
        first.host.set_is_animated(false);
        first.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let scope = crate::test_ui::ui(&first);
        let filter = scope
            .find("LanguageFilter")
            .unwrap()
            .cast::<TextBox>()
            .unwrap();
        let list = scope
            .find("LanguageChoices")
            .unwrap()
            .cast::<ItemsControl>()
            .unwrap();
        filter.set_text(Some("ABC"));
        Dispatcher::ui_thread().run_jobs(None);
        let check = find_visual(
            &list.container_from_index(0).unwrap().upcast::<Visual>(),
            "LanguageCheckBox",
        )
        .unwrap()
        .cast::<CheckBox>()
        .unwrap();
        let context = first.bindings.model().panel_context();
        context.retarget(Some((
            second.keyboard.settings(),
            second.keyboard.engine().language_models(),
        )));
        filter.set_text(Some("no such language"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(list.items().count(), 0);
        {
            let _guard = context.begin_refresh();
            check.set_is_checked(Some(true)); // Source language handlers do not inspect _refreshing.
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(
            *first.keyboard.settings().subtypes().borrow(),
            vec![Some("en-US/qwerty".into())]
        );
        assert_eq!(
            *second.keyboard.settings().subtypes().borrow(),
            vec![
                Some("en-US/qwerty".into()),
                Some("en-US/alphabetical".into())
            ]
        );
        assert!(
            second
                .keyboard
                .engine()
                .subtypes()
                .borrow()
                .iter()
                .any(|s| s.id() == "en-US/alphabetical".into())
        );
        first.window.close();
        first.release();
        second.release();
    });
}
