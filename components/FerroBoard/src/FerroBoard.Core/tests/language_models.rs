use ferroboard_core::{engine::*, text::*};
use serde_json::{Value, json};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc,
};
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/language-models.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn d(label: &str) -> Arc<WordDictionary> {
    Arc::new(WordDictionary::from_words([("test".into(), 42)], label.into()).unwrap())
}
fn languages(m: &LanguageModels) -> Value {
    json!(m.languages().iter().map(|s| s.units()).collect::<Vec<_>>())
}
fn error(e: &LanguageModelError) -> &'static str {
    match e {
        LanguageModelError::NullFactory => "ArgumentNullException",
        LanguageModelError::RecursiveFactory => "InvalidOperationException",
        LanguageModelError::Factory(_) => "ApplicationException",
        _ => panic!("unexpected {e:?}"),
    }
}
fn found(r: LanguageModelResult<Option<Arc<WordDictionary>>>) -> Value {
    match r {
        Ok(v) => json!({"value":v.map(|d|d.language_tag().unwrap().to_utf8().unwrap())}),
        Err(e) => json!({"error":error(&e)}),
    }
}
#[test]
fn source_registry_normalization_order_lazy_instances_and_nulls() {
    let m = LanguageModels::new();
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "initial" => {
                assert_eq!(languages(&m), row["languages"]);
                for q in row["queries"].as_array().unwrap() {
                    let t = text(&q["tag"]);
                    assert_eq!(m.has_dictionary(&t), q["has"]);
                    assert_eq!(m.is_loaded(&t), q["loaded"]);
                }
            }
            "register" => {
                let tag = text(&row["tag"]);
                let dictionary = d(row["label"].as_str().unwrap());
                m.register_dictionary(&tag, Some(dictionary.clone()));
                assert_eq!(languages(&m), row["languages"]);
                assert_eq!(m.is_loaded(&tag), row["before"]);
                let value = m.get_dictionary(&tag).unwrap().unwrap();
                assert_eq!(
                    value.language_tag().unwrap().to_utf8().unwrap(),
                    row["found"]
                );
                assert!(Arc::ptr_eq(&value, &dictionary));
                assert!(Arc::ptr_eq(
                    &value,
                    &m.get_dictionary(&tag).unwrap().unwrap()
                ));
                assert_eq!(m.is_loaded(&tag), row["loaded"]);
            }
            "queries" => {
                for q in row["queries"].as_array().unwrap() {
                    let t = text(&q["tag"]);
                    assert_eq!(m.has_dictionary(&t), q["has"]);
                    assert_eq!(m.is_loaded(&t), q["loaded"]);
                    assert_eq!(
                        m.get_dictionary(&t)
                            .unwrap()
                            .unwrap()
                            .language_tag()
                            .unwrap()
                            .to_utf8()
                            .unwrap(),
                        q["found"]
                    );
                }
                assert_eq!(languages(&m), row["languages"]);
            }
            "nulls" => {
                let m = LanguageModels::new();
                let dictionary = d("kept");
                m.register_dictionary(&"zz".into(), Some(dictionary.clone()));
                let e = m.register_factory(&"zz".into(), None).unwrap_err();
                assert_eq!(json!({"error":error(&e)}), row["result"]);
                assert_eq!(m.is_loaded(&"zz".into()), row["before"]);
                assert!(Arc::ptr_eq(
                    &dictionary,
                    &m.get_dictionary(&"zz".into()).unwrap().unwrap()
                ));
                m.register_dictionary(&"nil".into(), None);
                assert_eq!(m.is_loaded(&"nil".into()), row["nullBefore"]);
                assert!(m.get_dictionary(&"nil".into()).unwrap().is_none());
                assert_eq!(m.is_loaded(&"nil".into()), row["nullAfter"]);
                assert!(m.get_dictionary(&"missing".into()).unwrap().is_none());
            }
            "english" => {
                let m = LanguageModels::new();
                assert_eq!(m.is_loaded(&"EN-US".into()), row["before"]);
                assert_eq!(
                    m.get_dictionary(&"en".into()).unwrap().unwrap().count(),
                    row["count"].as_u64().unwrap() as usize
                );
                assert_eq!(m.is_loaded(&"en".into()), row["after"]);
                assert_eq!(m.languages(), vec![Utf16Text::from("en")]);
            }
            _ => {}
        }
    }
}
#[test]
fn source_factory_execution_failure_identity_recursion_and_replacement() {
    for row in rows().iter().filter(|r| r["kind"] == "factory") {
        let m = LanguageModels::new();
        let calls = Arc::new(AtomicUsize::new(0));
        let log = Arc::new(Mutex::new(Vec::new()));
        let counter = calls.clone();
        let output = log.clone();
        let weak = Arc::downgrade(&m);
        let mode = row["mode"].as_str().unwrap().to_owned();
        let expected = d("outer");
        let replacement = d("replacement");
        m.register_factory(
            &"xx-YY".into(),
            Some(Arc::new(move || {
                let m = weak.upgrade().unwrap();
                counter.fetch_add(1, Ordering::Relaxed);
                output.lock().unwrap().push(format!(
                    "enter:{}",
                    if m.is_loaded(&"xx".into()) {
                        "True"
                    } else {
                        "False"
                    }
                ));
                match mode.as_str() {
                    "fail" => return Err(Arc::new(LanguageModelError::Factory("factory".into()))),
                    "recursive" => return m.get_dictionary(&"xx".into()),
                    "caught-recursive" => {
                        let e = m.get_dictionary(&"xx".into()).err().unwrap();
                        assert!(matches!(&*e, LanguageModelError::RecursiveFactory));
                        let again = m.get_dictionary(&"xx".into()).err().unwrap();
                        assert!(Arc::ptr_eq(&e, &again));
                        output.lock().unwrap().push("caught".into());
                    }
                    "replace" => m.register_dictionary(&"XX".into(), Some(replacement.clone())),
                    _ => {}
                }
                Ok(if mode == "null" {
                    None
                } else {
                    Some(expected.clone())
                })
            })),
        )
        .unwrap();
        assert_eq!(m.is_loaded(&"xx".into()), row["before"]);
        let a = m.get_dictionary(&"xx".into());
        assert_eq!(found(a.clone()), row["a"]);
        assert_eq!(m.is_loaded(&"xx".into()), row["loadedA"]);
        let b = m.get_dictionary(&"xx".into());
        assert_eq!(found(b.clone()), row["b"]);
        assert_eq!(m.is_loaded(&"xx".into()), row["loadedB"]);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            row["calls"].as_u64().unwrap() as usize
        );
        assert_eq!(json!(*log.lock().unwrap()), row["log"]);
        let same = match (a, b) {
            (Err(a), Err(b)) => Arc::ptr_eq(&a, &b),
            _ => false,
        };
        assert_eq!(same, row["sameFailure"]);
    }
}
#[test]
fn source_user_cache_event_sender_reentrancy_and_snapshot_comparer() {
    let row = rows().into_iter().find(|r| r["kind"] == "users").unwrap();
    let m = LanguageModels::new();
    let log = Arc::new(Mutex::new(Vec::new()));
    let output = log.clone();
    let first = AtomicBool::new(true);
    let expected = Arc::downgrade(&m);
    m.user_dictionary_changed().add(Arc::new(move|event|{output.lock().unwrap().push(json!({"sender":Arc::ptr_eq(&event.sender,&expected.upgrade().unwrap()),"language":event.language.units(),"keys":event.sender.user_dictionaries().iter().map(|(k,_)|k.units()).collect::<Vec<_>>()}));if first.swap(false,Ordering::Relaxed){event.sender.get_user_dictionary(&"DE-de".into()).learn(&"nested".into(),None);}}));
    let user = m.get_user_dictionary(&"EN-us".into());
    let snapshot = m.user_dictionaries();
    user.learn(&"hello".into(), None);
    assert!(user.ptr_eq(&m.get_user_dictionary(&"en_GB".into())));
    m.get_user_dictionary(&"Σ-x".into())
        .learn(&"sigma".into(), None);
    m.get_user_dictionary(&"ς-y".into())
        .learn(&"final".into(), None);
    assert_eq!(json!(*log.lock().unwrap()), row["log"]);
    assert_eq!(
        json!(snapshot.iter().map(|(k, _)| k.units()).collect::<Vec<_>>()),
        row["snapshotKeys"]
    );
    assert_eq!(
        json!(
            m.user_dictionaries()
                .iter()
                .map(|(key, user)| json!({"key":key.units(),"json":user.export()}))
                .collect::<Vec<_>>()
        ),
        row["current"]
    );
    assert!(snapshot.get(&"EN".into()).unwrap().ptr_eq(&user));
    assert!(snapshot.get(&"en-US".into()).is_none());
}
#[test]
fn concurrent_initialization_matches_source_without_holding_registry_lock() {
    let expected = rows()
        .into_iter()
        .find(|r| r["kind"] == "concurrent")
        .unwrap();
    let m = LanguageModels::new();
    let (entered_tx, entered) = mpsc::channel();
    let (release_tx, release) = mpsc::channel();
    let release = Mutex::new(release);
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let dictionary = d("shared");
    let value = dictionary.clone();
    m.register_factory(
        &"xx".into(),
        Some(Arc::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            entered_tx.send(()).unwrap();
            release
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            Ok(Some(value.clone()))
        })),
    )
    .unwrap();
    let jobs: Vec<_> = (0..6)
        .map(|_| {
            let m = m.clone();
            std::thread::spawn(move || m.get_dictionary(&"XX-us".into()).unwrap().unwrap())
        })
        .collect();
    entered
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let loaded_during = m.is_loaded(&"xx".into());
    let (registered_tx, registered) = mpsc::channel();
    let registry = m.clone();
    let other = std::thread::spawn(move || {
        registry.register_dictionary(&"yy".into(), Some(d("other")));
        registered_tx
            .send(
                registry
                    .get_dictionary(&"yy".into())
                    .unwrap()
                    .unwrap()
                    .language_tag()
                    .unwrap()
                    .to_utf8()
                    .unwrap(),
            )
            .unwrap();
    });
    let other_label = registered.recv_timeout(std::time::Duration::from_secs(2));
    release_tx.send(()).unwrap();
    let same = jobs
        .into_iter()
        .all(|job| Arc::ptr_eq(&dictionary, &job.join().unwrap()));
    other.join().unwrap();
    assert_eq!(
        json!({"kind":"concurrent","calls":calls.load(Ordering::Relaxed),"loadedDuring":loaded_during,"loadedAfter":m.is_loaded(&"xx".into()),"same":same,"other":other_label.unwrap()}),
        expected
    );
}
#[test]
fn recursive_fault_is_published_before_the_outer_factory_recovers() {
    let expected = rows()
        .into_iter()
        .find(|r| r["kind"] == "recursive-publication")
        .unwrap();
    let m = LanguageModels::new();
    let weak = Arc::downgrade(&m);
    let (failed_tx, failed) = mpsc::channel();
    let (finish_tx, finish) = mpsc::channel();
    let finish = Mutex::new(finish);
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    m.register_factory(
        &"xx".into(),
        Some(Arc::new(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            let e = weak
                .upgrade()
                .unwrap()
                .get_dictionary(&"xx".into())
                .err()
                .unwrap();
            failed_tx.send(e).unwrap();
            finish
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
            Ok(Some(d("recovered")))
        })),
    )
    .unwrap();
    let outer = {
        let m = m.clone();
        std::thread::spawn(move || m.get_dictionary(&"xx".into()).unwrap().unwrap())
    };
    let first = failed
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    let loaded = m.is_loaded(&"xx".into());
    let (observed_tx, observed) = mpsc::channel();
    let middle = {
        let m = m.clone();
        std::thread::spawn(move || {
            observed_tx
                .send(m.get_dictionary(&"xx".into()).err().unwrap())
                .unwrap()
        })
    };
    let second = observed.recv_timeout(std::time::Duration::from_secs(2));
    finish_tx.send(()).unwrap();
    let value = outer.join().unwrap();
    middle.join().unwrap();
    let second = second.unwrap();
    assert_eq!(
        json!({"kind":"recursive-publication","calls":calls.load(Ordering::Relaxed),"loadedDuring":loaded,"sameFailure":Arc::ptr_eq(&first,&second),"error":error(&second),"value":value.language_tag().unwrap().to_utf8().unwrap(),"loadedAfter":m.is_loaded(&"xx".into())}),
        expected
    );
}
#[test]
fn user_handles_preserve_source_retention_identity_and_release_internal_cycles() {
    let expected = rows()
        .into_iter()
        .find(|r| r["kind"] == "lifetime")
        .unwrap();
    let events = Arc::new(AtomicUsize::new(0));
    let m = LanguageModels::new();
    let owner = Arc::downgrade(&m);
    let counter = events.clone();
    m.user_dictionary_changed().add(Arc::new(move |_| {
        counter.fetch_add(1, Ordering::Relaxed);
    }));
    let user = m.get_user_dictionary(&"en".into());
    let weak = user.downgrade();
    drop(m);
    let while_user = owner.upgrade().is_some();
    let user_alive = weak.upgrade().is_some();
    user.learn(&"alive".into(), None);
    drop(user);
    assert_eq!(
        json!({"kind":"lifetime","whileUser":while_user,"userAlive":user_alive,"afterDrop":owner.upgrade().is_some(),"userAfterDrop":weak.upgrade().is_some(),"Events":events.load(Ordering::Relaxed)}),
        expected
    );
    let expected = rows()
        .into_iter()
        .find(|r| r["kind"] == "weak-cache")
        .unwrap();
    let m = LanguageModels::new();
    let weak = m.get_user_dictionary(&"en".into()).downgrade();
    let cached = weak.upgrade().is_some();
    let same = weak
        .upgrade()
        .unwrap()
        .ptr_eq(&m.get_user_dictionary(&"en".into()));
    assert_eq!(
        json!({"kind":"weak-cache","cached":cached,"same":same}),
        expected
    );
    let owner = Arc::downgrade(&m);
    let snapshot = m.user_dictionaries();
    drop(m);
    assert!(owner.upgrade().is_some());
    drop(snapshot);
    assert!(owner.upgrade().is_none());
    assert!(weak.upgrade().is_none());
}
#[test]
fn factory_panics_are_cached_as_explicit_failures_with_original_payload() {
    let m = LanguageModels::new();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    m.register_factory(
        &"xx".into(),
        Some(Arc::new(move || {
            count.fetch_add(1, Ordering::Relaxed);
            std::panic::panic_any(17u32)
        })),
    )
    .unwrap();
    let a = m.get_dictionary(&"xx".into()).err().unwrap();
    let b = m.get_dictionary(&"xx".into()).err().unwrap();
    assert!(Arc::ptr_eq(&a, &b));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(!m.is_loaded(&"xx".into()));
    let LanguageModelError::Panic(panic) = &*a else {
        panic!("unexpected {a:?}")
    };
    panic.inspect(|p| assert_eq!(p.downcast_ref::<u32>(), Some(&17)));
}

#[test]
fn user_reference_handle_clone_equality_and_weak_lifetime() {
    use std::hash::{Hash, Hasher};
    let user = UserDictionary::default();
    let copy = user.clone();
    assert!(user == copy);
    assert!(user != UserDictionary::default());
    let hash = |u: &UserDictionary| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        u.hash(&mut h);
        h.finish()
    };
    assert_eq!(hash(&user), hash(&copy));
    let weak = user.downgrade();
    user.learn(&"shared".into(), None);
    assert_eq!(copy.get_count(&"shared".into()), 1);
    drop(user);
    assert!(weak.upgrade().unwrap().ptr_eq(&copy));
    drop(copy);
    assert!(weak.upgrade().is_none());
}
