//! Actual desktop AppBuilder initialization uses its supplied native storage service.
#[path = "../examples/support/memory_storage.rs"]
mod memory_storage;

use ferroboard_controls::ScreenKeyboardHost;
use ferroboard_sample::{
    App,
    sample_storage::{SETTINGS_KEY, SampleStorage},
};
use ferroui_base::controls::NameScope;
use ferroui_controls::{AppBuilder, application_lifetimes::ClassicDesktopStyleApplicationLifetime};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};

#[test]
fn desktop_app_uses_injected_storage_for_loading_live_saves_and_release() {
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    let storage = memory_storage::MemoryStorage::private();
    let builder = AppBuilder::configure::<App>()
        .with(SampleStorage::new(storage.clone()))
        .use_headless(FerroHeadlessPlatformOptions::default());
    builder.setup_with_lifetime(lifetime.clone());
    let app = builder.instance().unwrap().cast::<App>().unwrap();
    let window = lifetime.main_window().unwrap();
    let host = NameScope::get_name_scope(&window)
        .unwrap()
        .find("Host")
        .unwrap()
        .cast::<ScreenKeyboardHost>()
        .unwrap();
    let settings = host.keyboard().settings();
    assert!(storage.reads.get() > 0);
    assert!(settings.incognito_mode());
    assert!(!settings.learn_words());
    assert!(!settings.clipboard_history_enabled());
    assert!(!settings.clipboard_suggestion());
    assert!(!settings.emoji_history_enabled());
    let before = storage.writes.get();
    settings.set_key_height(73.0);
    assert!(storage.writes.get() > before);
    app.save_session();
    let saved = storage.settings.borrow().get(SETTINGS_KEY).unwrap().clone();
    assert_eq!(
        ferroboard_core::settings::KeyboardSettings::from_json(&saved)
            .unwrap()
            .key_height(),
        73.0
    );
    assert!(
        storage
            .documents
            .borrow()
            .contains_key(std::path::Path::new("clipboard.json"))
    );
    app.release_session();
    let retired = storage.writes.get();
    app.save_session();
    assert_eq!(storage.writes.get(), retired);
    assert!(storage.errors.borrow().is_empty());
    window.close();
}

#[test]
fn default_file_storage_loads_live_saves_and_reloads_across_processes() {
    const PHASE: &str = "FERROBOARD_TEST_FILE_STORAGE_PHASE";
    if let Some(phase) = std::env::var_os(PHASE) {
        let lifetime = ClassicDesktopStyleApplicationLifetime::new();
        let builder =
            AppBuilder::configure::<App>().use_headless(FerroHeadlessPlatformOptions::default());
        // No SampleStorage service is installed: exercise App's actual file fallback.
        builder.setup_with_lifetime(lifetime.clone());
        let app = builder.instance().unwrap().cast::<App>().unwrap();
        let window = lifetime.main_window().unwrap();
        struct SessionCleanup {
            app: ferroui_base::Ref<App>,
            window: ferroui_base::Ref<ferroui_controls::Window>,
        }
        impl Drop for SessionCleanup {
            fn drop(&mut self) {
                self.app.release_session();
                self.window.close();
            }
        }
        let _cleanup = SessionCleanup {
            app: app.clone(),
            window: window.clone(),
        };
        let host = NameScope::get_name_scope(&window)
            .unwrap()
            .find("Host")
            .unwrap()
            .cast::<ScreenKeyboardHost>()
            .unwrap();
        let settings = host.keyboard().settings();
        assert!(settings.incognito_mode());
        assert!(!settings.learn_words());
        assert!(!settings.clipboard_history_enabled());
        assert!(!settings.clipboard_suggestion());
        assert!(!settings.emoji_history_enabled());
        match phase.to_str().unwrap() {
            "load-save" => {
                assert_eq!(settings.key_height(), 54.0);
                assert!(!settings.number_row());
                settings.set_key_height(61.0);
                settings.set_number_row(true);
                // Check the live settings subscription, before explicit session save.
                let path =
                    std::path::PathBuf::from(std::env::var_os("FERROBOARD_DATA_DIR").unwrap())
                        .join(SETTINGS_KEY);
                let saved = ferroboard_core::settings::KeyboardSettings::from_json(
                    &std::fs::read_to_string(path).unwrap(),
                )
                .unwrap();
                assert_eq!(saved.key_height(), 61.0);
                assert!(saved.number_row());
                app.save_session();
            }
            "reload" => {
                assert_eq!(settings.key_height(), 61.0);
                assert!(settings.number_row());
            }
            other => panic!("Unexpected file-storage test phase: {other}"),
        }
        return;
    }

    // Child-process environments keep the public fallback intact without unsafe global
    // set_var calls or races with the injected-storage test in this executable.
    let profile = std::env::temp_dir().join(format!(
        "ferroboard-default-file-storage-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&profile).unwrap();
    std::fs::write(
        profile.join(SETTINGS_KEY),
        serde_json::json!({
            "keyHeight": 54.0, "numberRow": false, "incognitoMode": true,
            "learnWords": false, "emojiHistoryEnabled": false,
            "clipboardHistoryEnabled": false, "clipboardSuggestion": false,
        })
        .to_string(),
    )
    .unwrap();
    for phase in ["load-save", "reload"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "default_file_storage_loads_live_saves_and_reloads_across_processes",
                "--nocapture",
            ])
            .env(PHASE, phase)
            .env("FERROBOARD_DATA_DIR", &profile)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{phase} failed in private profile {}:\n{}\n{}",
            profile.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(profile.join(SETTINGS_KEY)).unwrap()).unwrap();
    assert_eq!(saved["keyHeight"], 61.0);
    assert_eq!(saved["numberRow"], true);
    assert_eq!(saved["incognitoMode"], true);
    for name in ["clipboard.json", "emoji-history.json"] {
        let document = std::fs::read_to_string(profile.join(name)).unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&document).is_ok());
    }
    std::fs::remove_dir_all(profile).unwrap();
}
