//! Product behavior of the process-wide translation registry.
use ferroribbon_core::{RibbonString, localization::*, model::RibbonItemsSource};
use serde_json::{Value, json};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
    sync::{Arc, mpsc},
    time::Duration,
};

fn isolated(name: &str) -> bool {
    if std::env::var("FERRORIBBON_REGISTRY_CASE").as_deref() == Ok(name) {
        return false;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("FERRORIBBON_REGISTRY_CASE", name)
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("running 1 test\n") && output.status.success(),
        "{name}\n{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    true
}
fn case(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!(
        "../../../tests/fixtures/localization-registry.json"
    ))
    .unwrap()["cases"][name]
        .clone()
}
fn add(name: &str, rows: Vec<(&str, Option<&str>)>) {
    RibbonStrings::register(
        Some(name.into()),
        Some(RibbonItemsSource::from_values(
            rows.into_iter()
                .map(|(k, v)| (Some(k.into()), v.map(Into::into)))
                .collect(),
        )),
    )
    .unwrap();
}
fn get(name: &str) -> Arc<RibbonStrings> {
    RibbonStrings::for_culture(Some(RibbonCulture::new(name.into(), "x".into()))).unwrap()
}
fn units(s: Option<RibbonString>) -> Value {
    s.map_or(Value::Null, |s| json!(s.as_utf16()))
}

#[test]
fn background_registration_matches_original_process_wide_fixture() {
    if isolated("background_registration_matches_original_process_wide_fixture") {
        return;
    }
    let _ = RibbonStrings::current();
    std::thread::spawn(|| {
        RibbonStrings::register(
            Some("x-thread".into()),
            Some(RibbonItemsSource::from_values(vec![(
                Some("File".into()),
                Some("Background".into()),
            )])),
        )
        .unwrap();
    })
    .join()
    .unwrap();
    let strings =
        RibbonStrings::for_culture(Some(RibbonCulture::new("x-thread".into(), "x".into())))
            .unwrap();
    let source: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/localization.json")).unwrap();
    assert!(source["threads"]["present"].as_bool().unwrap());
    assert_eq!(
        serde_json::json!(strings.file().unwrap().as_utf16()),
        source["threads"]["file"]
    );
    assert_eq!(strings.file(), Some(RibbonString::from("Background")));
}

#[test]
fn live_shared_key_views_keep_identity_and_detect_background_insertions() {
    if isolated("live_shared_key_views_keep_identity_and_detect_background_insertions") {
        return;
    }
    fn thread_safe<T: Send + Sync>() {}
    thread_safe::<RibbonString>();
    thread_safe::<RibbonLocalizationKeys>();
    let cultures = RibbonStrings::built_in_cultures();
    let keys = RibbonStrings::keys();
    let mut cursor = cultures.iter();
    cursor.next();
    let (other_cultures, other_keys) = std::thread::spawn(|| {
        let c = RibbonStrings::built_in_cultures();
        let k = RibbonStrings::keys();
        add("x-view", vec![("File", Some("Shared"))]);
        (c, k)
    })
    .join()
    .unwrap();
    let invalid = catch_unwind(AssertUnwindSafe(|| cursor.next())).is_err();
    assert_eq!(
        json!({"same":Arc::ptr_eq(&cultures,&other_cultures)&&Arc::ptr_eq(&keys,&other_keys),"invalid":invalid,"present":cultures.iter().any(|s|s==RibbonString::from("x-view")),"count":cultures.count()}),
        case("views")
    );
}

#[test]
fn worker_english_updates_preserve_snapshots_live_fallback_and_key_replacement_version() {
    if isolated(
        "worker_english_updates_preserve_snapshots_live_fallback_and_key_replacement_version",
    ) {
        return;
    }
    let old = get("x-unknown");
    let mut keys = RibbonStrings::keys().iter();
    keys.next();
    std::thread::spawn(|| add("en", vec![("File", Some("Worker English"))]))
        .join()
        .unwrap();
    let valid = keys.next().is_some();
    std::thread::spawn(|| add("en", vec![("Late", Some("Background late"))]))
        .join()
        .unwrap();
    assert_eq!(
        json!({"old":units(old.file()),"fresh":units(get("x-unknown").file()),"late":units(old.get(Some(&"Late".into())).unwrap()),"replacementKeptCursor":valid}),
        case("fallback")
    );
}

#[test]
fn failed_registration_keeps_partial_rows_and_releases_the_transaction() {
    if isolated("failed_registration_keeps_partial_rows_and_releases_the_transaction") {
        return;
    }
    let failed = catch_unwind(AssertUnwindSafe(|| {
        RibbonStrings::register(
            Some("x-abort".into()),
            Some(RibbonItemsSource::new(|| {
                Box::new(
                    std::iter::once((Some("File".into()), Some("Partial".into())))
                        .chain(std::iter::once_with(|| panic!("input"))),
                )
            })),
        )
        .unwrap();
    }));
    assert!(failed.is_err());
    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        add("x-after", vec![("File", Some("After"))]);
        tx.send(get("x-abort").file()).unwrap();
    });
    let partial = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("registration remained locked after panic");
    worker.join().unwrap();
    assert_eq!(
        json!({"partial":units(partial),"after":units(get("x-after").file())}),
        case("abort")
    );
}

struct ReadCulture {
    entered: mpsc::Sender<()>,
}
impl RibbonCultureInfo for ReadCulture {
    fn name(&self) -> Option<RibbonString> {
        self.entered.send(()).unwrap();
        Some("x-txn".into())
    }
    fn two_letter_iso_language_name(&self) -> Option<RibbonString> {
        Some("x".into())
    }
}
#[test]
fn registration_is_reentrant_and_snapshot_getters_wait_for_the_complete_batch() {
    if isolated("registration_is_reentrant_and_snapshot_getters_wait_for_the_complete_batch") {
        return;
    }
    let (ready_tx, ready_rx) = mpsc::channel();
    let (start_tx, start_rx) = mpsc::channel();
    let (name_tx, name_rx) = mpsc::channel();
    let (attempt_tx, attempt_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let _ = RibbonStrings::current();
        ready_tx.send(()).unwrap();
        start_rx.recv().unwrap();
        attempt_tx.send(()).unwrap();
        RibbonStrings::for_culture(Some(Arc::new(ReadCulture { entered: name_tx })))
            .unwrap()
            .file()
    });
    ready_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let (partial_tx, partial_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let release_rx = Rc::new(release_rx);
        RibbonStrings::register(
            Some("x-txn".into()),
            Some(RibbonItemsSource::new(move || {
                let partial = partial_tx.clone();
                let release = release_rx.clone();
                Box::new(
                    std::iter::once((Some("File".into()), Some("First".into()))).chain(
                        std::iter::once_with(move || {
                            add("x-txn", vec![("Options", Some("Nested"))]);
                            partial.send(get("x-txn").file()).unwrap();
                            release.recv_timeout(Duration::from_secs(5)).unwrap();
                            (Some("File".into()), Some("Final".into()))
                        }),
                    ),
                )
            })),
        )
        .unwrap();
    });
    let nested = partial_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("nested registration/snapshot deadlocked");
    start_tx.send(()).unwrap();
    attempt_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let entered = name_rx.recv_timeout(Duration::from_millis(100)).is_ok();
    release_tx.send(()).unwrap();
    writer.join().unwrap();
    let read = reader.join().unwrap();
    assert_eq!(
        json!({"nested":units(nested),"read":units(read),"options":units(get("x-txn").options()),"enteredBeforeRelease":entered}),
        case("transaction")
    );
}

#[test]
fn lossless_text_and_null_translation_values_survive_worker_registration() {
    if isolated("lossless_text_and_null_translation_values_survive_worker_registration") {
        return;
    }
    let name = RibbonString::from_utf16(vec![120, 45, 0xd800]);
    let key: RibbonString = "e\u{301}𐐨".into();
    let mut value = vec![0xdc00];
    value.extend("العربية👩‍💻".encode_utf16());
    let value = RibbonString::from_utf16(value);
    let n = name.clone();
    let k = key.clone();
    let v = value.clone();
    std::thread::spawn(move || {
        RibbonStrings::register(
            Some(n),
            Some(RibbonItemsSource::from_values(vec![
                (Some(k), Some(v)),
                (Some("Options".into()), None),
            ])),
        )
        .unwrap();
    })
    .join()
    .unwrap();
    let s = RibbonStrings::for_culture(Some(RibbonCulture::new(name.clone(), "x".into()))).unwrap();
    assert_eq!(
        json!({"value":units(s.get(Some(&key)).unwrap()),"nullValue":units(s.options()),"retained":RibbonStrings::built_in_cultures().iter().any(|s|s==name)}),
        case("units")
    );
}

#[test]
fn concurrent_workers_publish_all_registered_cultures_with_ordinal_lookup() {
    if isolated("concurrent_workers_publish_all_registered_cultures_with_ordinal_lookup") {
        return;
    }
    let workers = (0..8)
        .map(|i| {
            std::thread::spawn(move || {
                add(
                    &format!("x-worker-{i}"),
                    vec![
                        ("File", Some(&format!("Worker {i}"))),
                        ("Options", Some(&format!("Options {i}"))),
                    ],
                )
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(
        json!({"files":(0..8).map(|i|units(get(&format!("X-WORKER-{i}")).file())).collect::<Vec<_>>(),"count":RibbonStrings::built_in_cultures().count()}),
        case("workers")
    );
}
