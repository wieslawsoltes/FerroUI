//! Original MainPage handler projections and the actual integrated public control.
use crate::{
    composition::SampleSession,
    sample_options::SampleOptions,
    sample_storage::{
        FileSampleStoragePlatform, SETTINGS_KEY, SampleStorage, SampleStoragePlatform,
        StorageResult,
    },
    view_model::MainViewModel,
};
use ferroboard_core::editor::EnterAction;
use ferroui_base::{Ref, Visual, controls::NameScope, threading::Dispatcher};
use ferroui_controls::{Control, SplitView, SplitViewDisplayMode, TextBox};
use std::{cell::Cell, path::PathBuf, rc::Rc, sync::Arc};
struct CountedStorage {
    file: FileSampleStoragePlatform,
    writes: Cell<usize>,
}
impl SampleStoragePlatform for CountedStorage {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        self.file.get_setting(key)
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.writes.set(self.writes.get() + 1);
        self.file.set_setting(key, value)
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        self.file.local_folder()
    }
    fn log(&self, message: &str) {
        self.file.log(message)
    }
}
#[test]
fn integrated_main_page_handlers_match_unchanged_source_and_share_the_actual_panel() {
    crate::tests::run(|| {
        let folder = crate::storage_tests::unique_folder();
        std::fs::write(folder.join(SETTINGS_KEY), r#"{"keyHeight":70}"#).unwrap();
        let platform = Rc::new(CountedStorage {
            file: FileSampleStoragePlatform::new(folder.clone()),
            writes: Cell::new(0),
        });
        let options = Arc::new(SampleOptions::default());
        options.parse(Some(&"--keyheight=61.25".into()));
        let session =
            SampleSession::with_options(SampleStorage::new(platform.clone()), options).unwrap();
        let scope = NameScope::get_name_scope(&session.window).unwrap();
        assert_eq!(
            scope
                .find("SettingsPane")
                .unwrap()
                .cast::<crate::SettingsPanel>(),
            Some(session.settings_panel.clone())
        );
        assert!(
            scope.find("NumberRowToggle").is_none(),
            "former test ids are not invented source namescope aliases"
        );
        let presentation = session.settings_panel.presentation().unwrap();
        assert!(Rc::ptr_eq(
            &presentation.languages,
            &session.bindings.languages()
        ));
        assert!(Rc::ptr_eq(
            &presentation.context,
            &session.bindings.model().panel_context()
        ));
        let split = scope.find("Split").unwrap().cast::<SplitView>().unwrap();
        let rows = include_str!("../../tests/fixtures/main-page.jsonl")
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap());
        let mut enters = 0;
        let mut targets = 0;
        let mut sizes = 0;
        for row in rows {
            match row["operation"].as_str().unwrap() {
                "constructor" => {
                    assert_eq!(session.bindings.status(), None);
                    assert_eq!(
                        session.keyboard.settings().key_height(),
                        row["keyHeight"].as_f64().unwrap()
                    );
                    assert_eq!(
                        platform.writes.get(),
                        0,
                        "storage/options/panel initialization must precede save subscription"
                    );
                    assert_eq!(
                        std::fs::read_to_string(folder.join(SETTINGS_KEY)).unwrap(),
                        r#"{"keyHeight":70}"#
                    );
                }
                "target" => {
                    let kind = row["input"].as_str().unwrap();
                    if kind == "control" {
                        // The recorder can inject an arbitrary Control. The native target factory
                        // currently only accepts text editors, so compare this handler projection
                        // separately, without claiming custom native editor support.
                        assert_eq!(
                            MainViewModel::target_status(Some("Control")),
                            row["status"].as_str().unwrap()
                        );
                        continue;
                    }
                    let target: Option<Ref<Visual>> = match kind {
                        "none" => None,
                        "message" => Some(
                            scope
                                .find("MessageBox")
                                .unwrap()
                                .cast::<TextBox>()
                                .unwrap()
                                .upcast(),
                        ),
                        "password" => Some(
                            scope
                                .find("PasswordField")
                                .unwrap()
                                .cast::<Control>()
                                .unwrap()
                                .upcast(),
                        ),
                        "no-header" | "empty-header" => {
                            let text = TextBox::new();
                            if kind == "empty-header" {
                                ferroui_controls::automation::AutomationProperties::set_name(
                                    &text,
                                    Some(""),
                                );
                            }
                            Some(text.upcast())
                        }
                        _ => panic!("unknown source target {kind}"),
                    };
                    assert!(session.keyboard.attach_to(target).unwrap());
                    assert_eq!(
                        session.bindings.status().as_deref(),
                        row["status"].as_str(),
                        "{kind}"
                    );
                    targets += 1;
                }
                "enter" => {
                    let input = &row["input"];
                    let action = match input["action"].as_i64().unwrap() {
                        0 => EnterAction::Default,
                        1 => EnterAction::NewLine,
                        2 => EnterAction::Go,
                        3 => EnterAction::Search,
                        4 => EnterAction::Send,
                        5 => EnterAction::Next,
                        6 => EnterAction::Previous,
                        7 => EnterAction::Done,
                        _ => panic!("unknown source enum"),
                    };
                    let element = input["name"].as_str().map(|name| {
                        let text = TextBox::new();
                        text.set_name(Some(name.to_owned()));
                        text.upcast::<Visual>()
                    });
                    let args =
                        ferroboard_controls::EnterActionRequestedEventArgs::new(element, action);
                    args.set_handled(input["handled"].as_bool().unwrap());
                    session.keyboard.enter_action_requested().raise(&args);
                    assert_eq!(session.bindings.status().as_deref(), row["status"].as_str());
                    assert_eq!(args.handled(), row["result"].as_bool().unwrap());
                    enters += 1;
                }
                "resize" => {
                    session.bindings.model().update_width(
                        row["input"]["value"].as_f64().unwrap(),
                        row["input"]["previous"].as_f64().unwrap(),
                    );
                    Dispatcher::ui_thread().run_jobs(None);
                    assert_eq!(split.is_pane_open(), row["pane"].as_bool().unwrap());
                    assert_eq!(
                        split.display_mode(),
                        if row["mode"] == "Inline" {
                            SplitViewDisplayMode::Inline
                        } else {
                            SplitViewDisplayMode::Overlay
                        }
                    );
                    sizes += 1;
                }
                "settings" => {
                    session
                        .keyboard
                        .settings_requested()
                        .raise(&session.keyboard);
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(split.is_pane_open());
                }
                "external-setting" => {
                    let before = platform.writes.get();
                    session.keyboard.settings().set_font_scale(1.2);
                    assert_eq!(
                        platform.writes.get(),
                        before + 1,
                        "one original save subscriber, no duplicate projection-triggered writes"
                    );
                }
                // Native loaded focus/options/storage tests exercise real UI effects separately;
                // repeated Page.Unloaded is not modeled by the additional Rust owner boundary.
                "loaded" | "unloaded" | "unloaded-again" => (),
                other => panic!("unknown source operation {other:?}"),
            }
        }
        assert_eq!((targets, enters, sizes), (5, 48, 3));
        session.release();
    });
}
#[test]
fn integrated_session_release_breaks_panel_ownership_and_allows_native_objects_to_drop() {
    crate::tests::run(|| {
        let session = SampleSession::new(crate::storage_tests::file_storage());
        let weak_panel = session.settings_panel.downgrade();
        let weak_keyboard = session.keyboard.downgrade();
        let languages = session.bindings.languages().model();
        session.window.show();
        Dispatcher::ui_thread().run_jobs(None);
        session.window.close();
        session.release();
        assert_eq!(languages.rows_changed().handler_count(), 0);
        drop(session);
        // Closing queues compositor disposal. Finish actual render batches, as the selected
        // HeadlessUnitTestSession does between tests, before measuring native object lifetime.
        for _ in 0..10 {
            ferroui_headless::FerroHeadlessPlatform::force_render_timer_tick(1);
            Dispatcher::ui_thread().run_jobs(None);
        }
        assert!(weak_panel.upgrade().is_none());
        assert!(weak_keyboard.upgrade().is_none());
    });
}
#[test]
fn failed_public_panel_construction_releases_partial_native_owners_without_saving() {
    crate::tests::run(|| {
        use ferroboard_controls::themes::{KeyboardTheme, KeyboardThemes};
        use ferroboard_core::text::Utf16Text;
        let resources = crate::composition::SampleComposition::resources().unwrap();
        let baseline = resources.changed().handler_count();
        let theme = Arc::new(KeyboardTheme::default());
        theme.set_id(Some("sample_failure_cleanup".into()));
        theme.set_name(Some("ZZZ failure cleanup".into()));
        KeyboardThemes::register(theme.clone()).unwrap();
        theme.set_name(Some(Utf16Text::from_units(vec![0xd800])));
        let folder = crate::storage_tests::unique_folder();
        let original = r#"{"keyHeight":70}"#;
        std::fs::write(folder.join(SETTINGS_KEY), original).unwrap();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            SampleSession::new(SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(
                folder.clone(),
            ))))
        }));
        theme.set_name(Some("ZZZ failure cleanup".into()));
        assert!(
            outcome.is_err(),
            "native String bridge must reject malformed UTF-16, not replace it"
        );
        assert_eq!(
            resources.changed().handler_count(),
            baseline,
            "failed construction must release keyboard/engine subscriptions even after partial panel creation"
        );
        assert_eq!(
            std::fs::read_to_string(folder.join(SETTINGS_KEY)).unwrap(),
            original
        );
        assert!(!folder.join("emoji-history.json").exists());
    });
}
