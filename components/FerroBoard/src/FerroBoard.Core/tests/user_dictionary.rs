use ferroboard_core::{serialization::JsonBridgeError, text::*};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
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
    include_str!("../../../tests/fixtures/user-dictionary.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn pairs(v: impl IntoIterator<Item = (Utf16Text, i32)>) -> Value {
    json!(
        v.into_iter()
            .map(|(word, count)| json!({"word":word.units(),"count":count}))
            .collect::<Vec<_>>()
    )
}
// Public native snapshots are unordered; source completion/prediction order stays exact.
fn canonical_pairs(mut value: Value) -> Value {
    value.as_array_mut().unwrap().sort_by(|left, right| {
        text(&left["word"])
            .units()
            .cmp(text(&right["word"]).units())
    });
    value
}
// Only the blocked set's array order migrates. Preserve the raw words/bigrams JSON prefix.
fn canonical_export(value: &str) -> String {
    let offset = value.rfind(",\"blocked\":").unwrap();
    let document: Value = serde_json::from_str(value).unwrap();
    let mut blocked = document["blocked"].as_array().unwrap().clone();
    blocked.sort_by(|left, right| {
        left.as_str()
            .unwrap()
            .encode_utf16()
            .cmp(right.as_str().unwrap().encode_utf16())
    });
    format!(
        "{},\"blocked\":{}}}",
        &value[..offset],
        serde_json::to_string(&blocked).unwrap()
    )
}
fn canonical_state(mut value: Value) -> Value {
    value["words"] = canonical_pairs(value["words"].take());
    value["export"] = Value::String(canonical_export(value["export"].as_str().unwrap()));
    value
}
fn json_error(e: JsonBridgeError) -> &'static str {
    match e {
        JsonBridgeError::Syntax { .. } => "JsonReaderException",
        JsonBridgeError::NumberFormat => "FormatException",
        JsonBridgeError::InvalidOperation | JsonBridgeError::InvalidString => {
            "InvalidOperationException"
        }
    }
}
fn result(r: Result<(), JsonBridgeError>) -> Value {
    match r {
        Ok(()) => json!({"value":true}),
        Err(e) => json!({"error":json_error(e)}),
    }
}
fn state(d: &UserDictionary, events: usize, queries: &Value) -> Value {
    let queries:Vec<_>=queries.as_array().unwrap().iter().map(|query|{let word=text(&query["word"]);let complete=match d.complete(&word){Ok(p)=>json!({"value":pairs(p)}),Err(_)=>json!({"error":"ArgumentException"})};json!({"word":word.units(),"count":d.get_count(&word),"blocked":d.is_blocked(&word),"complete":complete,"predict":pairs(d.predict(&word,5))})}).collect();
    json!({"words":pairs(d.words().iter().map(|(w,c)|(w.clone(),*c))),"export":d.export(),"events":events,"threshold":d.learn_threshold(),"max":d.max_words(),"queries":queries})
}
#[test]
fn learned_words_match_source_operations_partial_imports_and_event_counts() {
    let mut dictionary = UserDictionary::default();
    let mut events = Arc::new(AtomicUsize::new(0));
    let mut operations = 0;
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "start" => {
                dictionary = UserDictionary::default();
                events = Arc::new(AtomicUsize::new(0));
                let n = events.clone();
                dictionary.changed().add(Arc::new(move |_| {
                    n.fetch_add(1, Ordering::Relaxed);
                }));
            }
            "operation" => {
                operations += 1;
                let op = row["op"].as_array().unwrap();
                let name = text(&op[0]).to_utf8().unwrap();
                let actual = match name.as_str() {
                    "learn" => {
                        let previous = if op[2].is_null() {
                            None
                        } else {
                            Some(text(&op[2]))
                        };
                        dictionary.learn(&text(&op[1]), previous.as_ref());
                        json!({"value":true})
                    }
                    "block" => {
                        dictionary.block(&text(&op[1]));
                        json!({"value":true})
                    }
                    "unblock" => {
                        dictionary.unblock(&text(&op[1]));
                        json!({"value":true})
                    }
                    "clear" => {
                        dictionary.clear();
                        json!({"value":true})
                    }
                    "max" => {
                        dictionary.set_max_words(op[1].as_i64().unwrap() as i32);
                        json!({"value":true})
                    }
                    "threshold" => {
                        dictionary.set_learn_threshold(op[1].as_i64().unwrap() as i32);
                        json!({"value":true})
                    }
                    "import" => result(dictionary.import(&text(&op[1]).to_utf8().unwrap())),
                    _ => panic!("unknown {name}"),
                };
                assert_eq!(actual, row["result"], "{row}");
            }
            _ => continue,
        }
        assert_eq!(
            canonical_state(state(
                &dictionary,
                events.load(Ordering::Relaxed),
                &row["state"]["queries"]
            )),
            canonical_state(row["state"].clone()),
            "{} operation {operations}",
            row["name"]
        );
    }
    assert!(operations > 250);
}
#[test]
fn changed_snapshot_dispatch_matches_reentrant_and_throwing_source_handlers() {
    for row in rows().into_iter().filter(|r| r["kind"] == "events") {
        let dictionary = UserDictionary::default();
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let once = AtomicBool::new(false);
        let second_id = Arc::new(AtomicU64::new(0));
        let id = second_id.clone();
        let log = calls.clone();
        let should_throw = row["mode"] == "throw";
        dictionary.changed().add(Arc::new(move |d| {
            log.lock()
                .unwrap()
                .push(format!("first:{}", d.get_count(&"a".into())));
            if !once.swap(true, Ordering::Relaxed) {
                d.changed().remove(id.load(Ordering::Relaxed));
                let log = log.clone();
                d.changed().add(Arc::new(move |d| {
                    log.lock()
                        .unwrap()
                        .push(format!("third:{}", d.get_count(&"a".into())))
                }));
                if should_throw {
                    panic!("source custom callback");
                }
                d.learn(&"a".into(), Some(&"a".into()));
            }
        }));
        let log = calls.clone();
        second_id.store(
            dictionary.changed().add(Arc::new(move |d| {
                log.lock()
                    .unwrap()
                    .push(format!("second:{}", d.get_count(&"a".into())))
            })),
            Ordering::Relaxed,
        );
        let actual = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            dictionary.learn(&"a".into(), None)
        })) {
            Ok(()) => json!({"value":true}),
            Err(_) => json!({"error":"ApplicationException"}),
        };
        assert_eq!(actual, row["result"]);
        assert_eq!(dictionary.export(), row["after"].as_str().unwrap());
        dictionary.learn(&"b".into(), Some(&"a".into()));
        assert_eq!(json!(*calls.lock().unwrap()), row["calls"]);
        assert_eq!(dictionary.export(), row["final"].as_str().unwrap());
    }
}
#[test]
fn snapshots_and_roundtrips_preserve_source_scope() {
    let row = rows()
        .into_iter()
        .find(|r| r["kind"] == "snapshot")
        .unwrap();
    let d = UserDictionary::default();
    d.learn(&"a".into(), None);
    d.learn(&"b".into(), None);
    d.block(&"a".into());
    let mut snapshot = d.words();
    snapshot.insert("x".into(), 9);
    d.learn(&"c".into(), None);
    assert_eq!(
        canonical_pairs(pairs(snapshot.iter().map(|(w, c)| (w.clone(), *c)))),
        canonical_pairs(row["snapshot"].clone())
    );
    assert_eq!(
        canonical_pairs(pairs(d.words().iter().map(|(w, c)| (w.clone(), *c)))),
        canonical_pairs(row["current"].clone())
    );
    assert_eq!(
        canonical_export(&d.export()),
        canonical_export(row["export"].as_str().unwrap())
    );
    // Original TextAndDataTests.User_dictionary_roundtrips.
    let user = UserDictionary::default();
    user.learn(&"foo".into(), None);
    user.learn(&"bar".into(), Some(&"foo".into()));
    user.block(&"baz".into());
    let copy = UserDictionary::default();
    copy.import(&user.export()).unwrap();
    assert_eq!(copy.get_count(&"foo".into()), 1);
    assert!(copy.is_blocked(&"baz".into()));
    assert_eq!(copy.predict(&"foo".into(), 1)[0].0, Utf16Text::from("bar"));
    assert_eq!(copy.learn_threshold(), 2);
    assert_eq!(copy.max_words(), 5000);
    assert!(copy.predict(&"foo".into(), 0).is_empty());
    assert!(copy.predict(&"foo".into(), -1).is_empty());
    copy.clear();
    assert!(copy.is_blocked(&"baz".into()));
}
#[test]
fn exported_json_matches_utf8jsonwriter_for_every_utf16_unit() {
    let corpus = rows();
    let expected = corpus.iter().find(|r| r["kind"] == "writer-bmp").unwrap();
    let mut hash = Sha256::new();
    let mut escaped = Vec::new();
    for unit in 0..=u16::MAX {
        let d = UserDictionary::default();
        d.learn(&Utf16Text::from_units(vec![unit]), None);
        let json = d.export();
        hash.update(json.as_bytes());
        hash.update(b"\n");
        if unit < 128 && json.contains('\\') {
            escaped.push(unit);
        }
    }
    assert_eq!(
        format!("{:x}", hash.finalize()),
        expected["sha256"].as_str().unwrap()
    );
    assert_eq!(json!(escaped), expected["escapedAscii"]);
    for row in corpus.iter().filter(|r| r["kind"] == "writer") {
        let d = UserDictionary::default();
        d.learn(&text(&row["word"]), None);
        assert_eq!(d.export(), row["export"].as_str().unwrap(), "{row}");
    }
}
#[test]
fn strict_json_grammar_and_lazy_conversions_match_source() {
    for row in rows().into_iter().filter(|r| r["kind"] == "grammar") {
        let d = UserDictionary::default();
        d.learn(&"seed".into(), None);
        let events = Arc::new(AtomicUsize::new(0));
        let n = events.clone();
        d.changed().add(Arc::new(move |_| {
            n.fetch_add(1, Ordering::Relaxed);
        }));
        let json = row["json"].as_str().unwrap();
        assert_eq!(result(d.import(json)), row["result"], "user {row}");
        assert_eq!(
            canonical_export(&d.export()),
            canonical_export(row["export"].as_str().unwrap())
        );
        assert_eq!(
            events.load(Ordering::Relaxed),
            row["events"].as_u64().unwrap() as usize
        );
        let actual = match WordDictionary::from_json(json, "en".into()) {
            Ok(d) => json!({"value":d.count()}),
            Err(DictionaryError::Json(e)) => json!({"error":json_error(e)}),
            Err(DictionaryError::Normalization(_)) => json!({"error":"ArgumentException"}),
            Err(e) => panic!("unexpected {e:?}"),
        };
        assert_eq!(actual, row["dictionary"], "dictionary {row}");
    }
}
#[test]
fn shared_learning_and_unlocked_event_delivery() {
    let d = Arc::new(UserDictionary::default());
    let events = Arc::new(AtomicUsize::new(0));
    let n = events.clone();
    let token = d.changed().add(Arc::new(move |_| {
        n.fetch_add(1, Ordering::Relaxed);
    }));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let d = d.clone();
            scope.spawn(move || {
                for _ in 0..100 {
                    d.learn(&"word".into(), Some(&"previous".into()));
                }
            });
        }
    });
    assert_eq!(d.get_count(&"word".into()), 400);
    assert_eq!(d.predict(&"PREVIOUS".into(), 1)[0].1, 400);
    assert_eq!(events.load(Ordering::Relaxed), 400);
    assert!(d.changed().remove(token));
    assert!(!d.changed().has_handlers());
    assert!(!d.changed().remove(token));
    let weak = Arc::downgrade(&d);
    d.changed().add(Arc::new(move |_| {
        let d = weak.upgrade().unwrap();
        let (send, recv) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            send.send(d.get_count(&"word".into())).unwrap();
        });
        assert_eq!(
            recv.recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            401
        );
        thread.join().unwrap();
    }));
    d.learn(&"word".into(), None);
}
