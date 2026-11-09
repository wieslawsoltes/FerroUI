use crate::sample_storage::{
    FileSampleStoragePlatform, SampleStorage, SampleStoragePlatform, StorageResult,
};
use ferroboard_core::{
    engine::{KeyboardEngine, LanguageModels},
    layouts::KeyboardResources,
    settings::KeyboardSettings,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

pub(crate) fn unique_folder() -> PathBuf {
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = std::env::temp_dir()
        .join("ferroboard-storage-tests")
        .join(format!(
            "{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
    fs::create_dir_all(&path).unwrap();
    path
}
pub(crate) fn file_storage() -> Rc<SampleStorage> {
    SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(unique_folder())))
}
struct Boundary {
    folder: Option<PathBuf>,
    denied: bool,
    settings: RefCell<String>,
    trace: RefCell<Vec<String>>,
    log: RefCell<Vec<String>>,
}
impl SampleStoragePlatform for Boundary {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        self.trace.borrow_mut().push(format!("get:{key}"));
        if self.denied {
            return Err("settings denied".into());
        }
        Ok(Some(self.settings.borrow().clone()))
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.trace.borrow_mut().push(format!("set:{key}"));
        if self.denied {
            return Err("settings denied".into());
        }
        *self.settings.borrow_mut() = value.into();
        Ok(())
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        self.trace.borrow_mut().push("folder".into());
        if self.denied {
            return Err("folder denied".into());
        }
        Ok(self.folder.clone())
    }
    fn log(&self, message: &str) {
        self.log
            .borrow_mut()
            .push(message.split(": ").next().unwrap().into());
    }
}
fn engine() -> Rc<KeyboardEngine> {
    KeyboardEngine::new_explicit(
        Rc::new(KeyboardSettings::default()),
        Rc::new(KeyboardResources::create_default().unwrap()),
        LanguageModels::new(),
    )
    .unwrap()
}
fn value(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}
// JSON number spelling (1 versus 1.0) is not a numeric difference. Corpus integers are all exactly
// representable as doubles; compare exact double values without any tolerance or rounding.
pub(crate) fn numeric_values(value: Value) -> Value {
    match value {
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                assert!(i.unsigned_abs() <= (1u64 << 53));
            }
            json!(n.as_f64().unwrap())
        }
        Value::Array(a) => Value::Array(a.into_iter().map(numeric_values).collect()),
        Value::Object(o) => {
            Value::Object(o.into_iter().map(|(k, v)| (k, numeric_values(v))).collect())
        }
        other => other,
    }
}
fn observe(engine: &Rc<KeyboardEngine>) -> Arc<Mutex<Vec<String>>> {
    let events = Arc::new(Mutex::new(Vec::new()));
    let target = events.clone();
    engine
        .settings()
        .property_changed()
        .add(Rc::new(move |name| {
            target.lock().unwrap().push(format!("settings:{name}"))
        }));
    let target = events.clone();
    engine.emoji_history().changed().add(Rc::new(move |_| {
        target.lock().unwrap().push("emoji".into())
    }));
    let target = events.clone();
    engine.clipboard_history().changed().add(Rc::new(move |_| {
        target.lock().unwrap().push("clipboard".into())
    }));
    let target = events.clone();
    engine
        .language_models()
        .user_dictionary_changed()
        .add(Arc::new(move |event| {
            target.lock().unwrap().push(format!(
                "user:{}",
                String::from_utf16_lossy(event.language.units())
            ))
        }));
    events
}
#[test]
fn unchanged_csharp_storage_corpus_matches_loading_saving_events_failure_boundaries_and_encodings()
{
    for line in include_str!("../../tests/fixtures/sample-storage.jsonl").lines() {
        let expected = value(line);
        let name = expected["name"].as_str().unwrap();
        let root = unique_folder();
        let folder = root.join(name);
        if name == "file-folder" {
            fs::write(&folder, "file").unwrap();
        } else {
            fs::create_dir(&folder).unwrap();
        }
        let clipboard = r#"[{"text":"pinned fixture","timestamp":"2026-01-01T00:00:00+00:00","pinned":true},{"text":"private transient","pinned":false}]"#;
        let emoji = r#"{"recent":["😀"],"pinned":["👍"],"tones":{}}"#;
        let dictionary =
            r#"{"words":{"ferrofixture":5},"bigrams":{},"blocked":["blockedfixture"]}"#;
        if name != "file-folder" {
            let content = if name == "invalid" { "{" } else { emoji };
            let mut bytes = content.as_bytes().to_vec();
            match name {
                "utf8-bom" => {
                    bytes = [&[0xef, 0xbb, 0xbf][..], &bytes].concat();
                }
                "utf16-le" | "utf16-be" => {
                    let little = name == "utf16-le";
                    bytes = if little {
                        vec![0xff, 0xfe]
                    } else {
                        vec![0xfe, 0xff]
                    };
                    for unit in content.encode_utf16() {
                        bytes.extend(if little {
                            unit.to_le_bytes()
                        } else {
                            unit.to_be_bytes()
                        });
                    }
                }
                "utf32-le" | "utf32-be" => {
                    let little = name == "utf32-le";
                    bytes = if little {
                        vec![0xff, 0xfe, 0, 0]
                    } else {
                        vec![0, 0, 0xfe, 0xff]
                    };
                    for c in content.chars() {
                        bytes.extend(if little {
                            (c as u32).to_le_bytes()
                        } else {
                            (c as u32).to_be_bytes()
                        });
                    }
                }
                "bad-utf8" => {
                    bytes = [
                        br#"{"recent":[""#.as_slice(),
                        &[0xf0, 0x80, 0x80, 0x80, 0xed, 0xa0, 0x80, 0xff],
                        br#""]}"#.as_slice(),
                    ]
                    .concat();
                }
                "bad-utf16" => {
                    bytes = vec![0xff, 0xfe];
                    for unit in "{\"recent\":[\""
                        .encode_utf16()
                        .chain([0xd800])
                        .chain("\"]}".encode_utf16())
                    {
                        bytes.extend(unit.to_le_bytes());
                    }
                }
                _ => {}
            }
            fs::write(folder.join("emoji-history.json"), bytes).unwrap();
            fs::write(folder.join("clipboard.json"), clipboard).unwrap();
            fs::write(folder.join("user-dictionary-en.json"), dictionary).unwrap();
            fs::write(folder.join("user-dictionary-fr.json"), dictionary).unwrap();
        }
        let platform = Rc::new(Boundary {
            folder: if name == "null-folder" {
                None
            } else {
                Some(folder.clone())
            },
            denied: name == "unavailable",
            settings: RefCell::new(
                if name == "invalid" {
                    "{"
                } else {
                    r#"{"numberRow":true,"keyHeight":61.25}"#
                }
                .into(),
            ),
            trace: RefCell::default(),
            log: RefCell::default(),
        });
        let storage = SampleStorage::new(platform.clone());
        let engine = engine();
        let events = observe(&engine);
        let throwing = Rc::new(std::cell::Cell::new(true));
        if name == "settings-callback" {
            let throwing = throwing.clone();
            engine
                .settings()
                .property_changed()
                .add(Rc::new(move |name| {
                    assert!(
                        !(throwing.get() && name == "NumberRow"),
                        "synchronous settings callback"
                    );
                }));
        }
        if name == "emoji-callback" {
            let throwing = throwing.clone();
            engine.emoji_history().changed().add(Rc::new(move |_| {
                assert!(!throwing.get(), "synchronous emoji callback");
            }));
        }
        storage.load(&engine);
        let users = engine
            .language_models()
            .user_dictionaries()
            .iter()
            .map(|(tag, d)| (tag.to_utf8().unwrap(), value(&d.export())))
            .collect::<serde_json::Map<_, _>>();
        let loaded = json!({"numberRow":engine.settings().number_row(),"keyHeight":engine.settings().key_height(),"emoji":value(&engine.emoji_history().export()),"clipboard":value(&engine.clipboard_history().export(true).unwrap()),"users":users});
        assert_eq!(
            numeric_values(loaded),
            numeric_values(expected["loaded"].clone()),
            "{name} loaded"
        );
        assert_eq!(
            json!(*platform.trace.borrow()),
            expected["loadTrace"],
            "{name} trace"
        );
        assert_eq!(
            json!(*events.lock().unwrap()),
            expected["loadEvents"],
            "{name} load events"
        );
        assert_eq!(
            json!(*platform.log.borrow()),
            expected["loadErrors"],
            "{name} errors"
        );
        throwing.set(false);
        engine
            .language_models()
            .get_user_dictionary(&"de-DE".into())
            .learn(&"zusatz".into(), None);
        if name == "surrogate-filename" {
            engine
                .language_models()
                .get_user_dictionary(&ferroboard_core::text::Utf16Text::from_units(vec![0xd800]))
                .learn(&"pathfixture".into(), None);
        }
        if name == "nul-filename" {
            engine
                .language_models()
                .get_user_dictionary(&"bad\0name".into())
                .learn(&"pathfixture".into(), None);
        }
        if name == "nan-settings" {
            engine.settings().set_key_height(f64::NAN);
        }
        events.lock().unwrap().clear();
        platform.trace.borrow_mut().clear();
        platform.log.borrow_mut().clear();
        storage.save(&engine).unwrap();
        let mut saved = serde_json::Map::new();
        if name != "file-folder" {
            for file in fs::read_dir(&folder).unwrap() {
                let file = file.unwrap();
                saved.insert(
                    file.file_name().into_string().unwrap(),
                    value(&fs::read_to_string(file.path()).unwrap()),
                );
            }
        }
        assert_eq!(
            json!(*platform.trace.borrow()),
            expected["saveTrace"],
            "{name} save trace"
        );
        assert_eq!(
            json!(*events.lock().unwrap()),
            expected["saveEvents"],
            "{name} save events"
        );
        assert_eq!(
            json!(*platform.log.borrow()),
            expected["saveErrors"],
            "{name} save errors"
        );
        assert_eq!(json!(saved), expected["saved"], "{name} saved");
        let settings = platform.settings.borrow();
        let settings = if settings.as_str() == "{" {
            Value::Null
        } else {
            value(&settings)
        };
        assert_eq!(
            numeric_values(settings),
            numeric_values(expected["settings"].clone()),
            "{name} settings"
        );
        engine.dispose().unwrap();
    }
}

#[test]
fn desktop_storage_retains_original_settings_and_reopens_all_persisted_source_data() {
    let folder = unique_folder();
    let storage = SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(folder.clone())));
    let first = engine();
    first.settings().set_number_row(true);
    first.emoji_history().add(&"😀".into()).unwrap();
    let clipboard = first.clipboard_history();
    let item = clipboard.add(Some(&"keep".into())).unwrap().unwrap();
    clipboard.set_pinned(&item, true);
    clipboard.add(Some(&"discard".into())).unwrap();
    first
        .language_models()
        .get_user_dictionary(&"en-US".into())
        .learn(&"fixture".into(), None);
    storage.save(&first).unwrap();
    first.dispose().unwrap();
    let second = engine();
    let original = second.settings();
    storage.load(&second);
    assert!(Rc::ptr_eq(&original, &second.settings()));
    assert!(original.number_row());
    assert_eq!(
        second.emoji_history().recent().lock().clone(),
        vec![Some("😀".into())]
    );
    assert_eq!(
        second
            .clipboard_history()
            .items()
            .unwrap()
            .iter()
            .map(|i| i.text.clone())
            .collect::<Vec<_>>(),
        vec!["keep".into()]
    );
    assert_eq!(
        second
            .language_models()
            .get_user_dictionary(&"en".into())
            .words()
            .get(&"fixture".into()),
        Some(&1)
    );
    second.dispose().unwrap();
}

#[derive(Default)]
struct DocumentStore {
    values: RefCell<std::collections::BTreeMap<String, String>>,
    denied: std::cell::Cell<bool>,
    messages: RefCell<Vec<String>>,
}
impl DocumentStore {
    fn read(&self, key: &str) -> StorageResult<Option<String>> {
        if self.denied.get() {
            return Err("native store denied".into());
        }
        Ok(self.values.borrow().get(key).cloned())
    }
    fn write(&self, key: &str, value: &str) -> StorageResult<()> {
        if self.denied.get() {
            return Err("native store denied".into());
        }
        self.values.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
}
impl SampleStoragePlatform for DocumentStore {
    fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        self.read(key)
    }
    fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        self.write(key, value)
    }
    fn local_folder(&self) -> StorageResult<Option<PathBuf>> {
        panic!("document storage must not require a filesystem folder")
    }
    fn log(&self, message: &str) {
        self.messages.borrow_mut().push(message.into());
    }
    fn read_document(&self, name: &std::path::Path) -> StorageResult<Option<String>> {
        self.read(name.to_str().unwrap())
    }
    fn write_document(&self, name: &std::path::Path, content: &str) -> StorageResult<()> {
        self.write(name.to_str().unwrap(), content)
    }
}

#[test]
fn native_document_store_restores_histories_and_learning_without_filesystem_emulation() {
    let platform = Rc::new(DocumentStore::default());
    let storage = SampleStorage::new(platform.clone());
    let first = engine();
    first.settings().set_number_row(true);
    first.emoji_history().add(&"😀".into()).unwrap();
    let clipboard = first.clipboard_history();
    let item = clipboard
        .add(Some(&"native store".into()))
        .unwrap()
        .unwrap();
    clipboard.set_pinned(&item, true);
    first
        .language_models()
        .get_user_dictionary(&"en".into())
        .learn(&"browserfixture".into(), None);
    storage.save(&first).unwrap();
    first.dispose().unwrap();
    assert!(
        platform
            .values
            .borrow()
            .contains_key("user-dictionary-en.json")
    );
    let second = engine();
    storage.load(&second);
    assert!(second.settings().number_row());
    assert_eq!(
        second.emoji_history().recent().lock().clone(),
        vec![Some("😀".into())]
    );
    assert_eq!(
        second.clipboard_history().items().unwrap()[0].text,
        "native store".into()
    );
    assert_eq!(
        second
            .language_models()
            .get_user_dictionary(&"en".into())
            .get_count(&"browserfixture".into()),
        1
    );
    assert!(platform.messages.borrow().is_empty());
    second.dispose().unwrap();
}

#[test]
fn native_document_store_denial_keeps_source_error_labels_and_does_not_fake_writes() {
    let platform = Rc::new(DocumentStore::default());
    platform.denied.set(true);
    let storage = SampleStorage::new(platform.clone());
    let engine = engine();
    storage.load(&engine);
    storage.save(&engine).unwrap();
    let labels: Vec<_> = platform
        .messages
        .borrow()
        .iter()
        .map(|value| value.split(": ").next().unwrap().to_owned())
        .collect();
    assert_eq!(
        labels,
        [
            "Could not load settings",
            "Could not read emoji-history.json",
            "Could not read clipboard.json",
            "Could not read user-dictionary-en.json",
            "Could not save settings",
            "Could not write emoji-history.json",
            "Could not write clipboard.json",
            "Could not write user-dictionary-en.json"
        ]
    );
    assert!(platform.values.borrow().is_empty());
    engine.dispose().unwrap();
}

#[test]
fn native_clipboard_export_error_propagates_before_write_without_fake_success() {
    use ferroboard_core::clipboard::{ClipboardError, OffsetDateTime, TimestampError};
    let folder = unique_folder();
    let storage = SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(folder.clone())));
    let engine = engine();
    let history = engine.clipboard_history();
    history.set_clock(Rc::new(|| OffsetDateTime::UNIX_EPOCH));
    let item = history.add(Some(&"unsaved".into())).unwrap().unwrap();
    history.set_pinned(&item, true);
    item.set_timestamp(OffsetDateTime::UNIX_EPOCH.replace_nanosecond(1).unwrap());
    assert!(matches!(
        storage.save(&engine),
        Err(ClipboardError::Timestamp(
            TimestampError::UnsupportedPrecision
        ))
    ));
    assert!(folder.join("emoji-history.json").is_file());
    assert!(!folder.join("clipboard.json").exists());
    assert_eq!(item.timestamp().nanosecond(), 1);
    engine.dispose().unwrap();
}
