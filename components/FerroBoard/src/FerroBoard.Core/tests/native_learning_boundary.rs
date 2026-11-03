use ferroboard_core::text::{UserDictionary, Utf16Text};
use serde_json::{Value, json};
use std::collections::HashMap;

#[test]
fn native_word_snapshots_support_standard_map_ownership_and_mutation() {
    let dictionary = UserDictionary::default();
    let shared = dictionary.clone();
    dictionary.learn(&"word".into(), None);
    dictionary.learn(&Utf16Text::from_units(vec![0xd800]), None);
    let mut snapshot: HashMap<Utf16Text, i32> = dictionary.words();
    *snapshot.entry("word".into()).or_default() = 7;
    snapshot.retain(|word, _| word.units() != [0xd800]);
    snapshot.insert("snapshot-only".into(), 12);
    assert_eq!(shared.get_count(&"word".into()), 1);
    assert_eq!(shared.get_count(&"snapshot-only".into()), 0);
    assert_eq!(shared.get_count(&Utf16Text::from_units(vec![0xd800])), 1);
    assert!(dictionary.ptr_eq(&shared));
    shared.learn(&"later".into(), None);
    assert!(!snapshot.contains_key(&Utf16Text::from("later")));
    let moved = std::thread::spawn(move || {
        assert_eq!(snapshot.len(), 2);
        snapshot.remove(&Utf16Text::from("word")).unwrap()
    })
    .join()
    .unwrap();
    assert_eq!(moved, 7);
}

#[test]
fn native_blocked_set_keeps_first_spelling_and_exports_deterministically() {
    let dictionary = UserDictionary::default();
    for word in ["Zulu", "zULU", "😀", "alpha", "𐐀"] {
        dictionary.block(&word.into());
    }
    let export: Value = serde_json::from_str(&dictionary.export()).unwrap();
    assert_eq!(export["blocked"], json!(["Zulu", "alpha", "𐐀", "😀"]));
    dictionary.learn(&"ZULU".into(), Some(&"context".into()));
    assert_eq!(dictionary.get_count(&"ZULU".into()), 0);
    dictionary.unblock(&"ZULU".into());
    dictionary.block(&"zulu".into());
    dictionary.clear();
    let export: Value = serde_json::from_str(&dictionary.export()).unwrap();
    assert_eq!(export["blocked"], json!(["alpha", "zulu", "𐐀", "😀"]));
    assert!(dictionary.is_blocked(&"ZULU".into()));
    assert!(dictionary.is_blocked(&"𐐨".into()));
    let restored = UserDictionary::default();
    restored.import(&dictionary.export()).unwrap();
    assert_eq!(restored.export(), dictionary.export());
}

#[test]
fn hash_growth_and_snapshot_order_do_not_change_learning_or_prediction_rank() {
    // Repeated independent randomized native maps must not leak their iteration into ranking.
    for _ in 0..16 {
        let dictionary = UserDictionary::default();
        dictionary.set_max_words(3);
        dictionary.set_learn_threshold(1);
        dictionary.learn(&"a".into(), Some(&"context".into()));
        dictionary.learn(&"b".into(), Some(&"context".into()));
        dictionary.learn(&"c".into(), Some(&"context".into()));
        dictionary.block(&"b".into());
        dictionary.learn(&"d".into(), Some(&"context".into()));
        dictionary.learn(&"e".into(), Some(&"context".into()));
        // Exact unchanged-source slot/tie scenario: a is evicted; d,c,e remain.
        assert_eq!(dictionary.get_count(&"a".into()), 0);
        assert_eq!(
            dictionary.complete(&"".into()).unwrap(),
            vec![("d".into(), 1), ("c".into(), 1), ("e".into(), 1)]
        );
        assert_eq!(
            dictionary.predict(&"CONTEXT".into(), 4),
            vec![
                ("a".into(), 1),
                ("d".into(), 1),
                ("c".into(), 1),
                ("e".into(), 1)
            ]
        );
        for index in 0..1024 {
            dictionary.block(&format!("blocked-{index:04}").as_str().into());
        }
        let snapshot: HashMap<Utf16Text, i32> = dictionary.words();
        assert_eq!(snapshot.len(), 3);
        let export = dictionary.export();
        let copy = UserDictionary::default();
        copy.import(&export).unwrap();
        assert_eq!(copy.export(), export);
        assert!(copy.is_blocked(&"BLOCKED-0999".into()));
        assert!(copy.is_blocked(&"B".into()));
        copy.unblock(&"blocked-0999".into());
        assert!(!copy.is_blocked(&"BLOCKED-0999".into()));
    }
}

#[test]
fn native_learning_storage_preserves_source_rank_through_churn_and_context_replacement() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let rows: Vec<Value> = include_str!("../../../tests/fixtures/native-learning-storage.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 339);
    // Independently seeded standard HashMaps must not decide any observed tie.
    for _ in 0..4 {
        let dictionary = UserDictionary::default();
        let events = Arc::new(AtomicUsize::new(0));
        let observer = events.clone();
        dictionary.changed().add(Arc::new(move |_| {
            observer.fetch_add(1, Ordering::Relaxed);
        }));
        for (index, row) in rows.iter().enumerate() {
            let operation = row["operation"].as_array().unwrap();
            let word = || Utf16Text::from(operation[1].as_str().unwrap());
            match operation[0].as_str().unwrap() {
                "max" => dictionary.set_max_words(operation[1].as_i64().unwrap() as i32),
                "threshold" => {
                    dictionary.set_learn_threshold(operation[1].as_i64().unwrap() as i32)
                }
                "learn" => {
                    let previous = operation[2].as_str().map(Utf16Text::from);
                    dictionary.learn(&word(), previous.as_ref());
                }
                "block" => dictionary.block(&word()),
                "unblock" => dictionary.unblock(&word()),
                "import" => dictionary.import(operation[1].as_str().unwrap()).unwrap(),
                "clear" => dictionary.clear(),
                other => panic!("unknown reference operation {other}"),
            }
            let raw = dictionary.export();
            let document: Value = serde_json::from_str(&raw).unwrap();
            let mut blocked = document["blocked"].as_array().unwrap().clone();
            blocked.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
            let prefix = &raw[..raw.rfind(",\"blocked\":").unwrap()];
            let export = format!(
                "{prefix},\"blocked\":{}}}",
                serde_json::to_string(&blocked).unwrap()
            );
            assert_eq!(export, row["export"].as_str().unwrap(), "operation {index}");
            assert_eq!(
                events.load(Ordering::Relaxed),
                row["events"].as_u64().unwrap() as usize,
                "operation {index}"
            );
            let pairs = |values: Vec<(Utf16Text, i32)>| -> Value {
                values
                    .into_iter()
                    .map(|(word, count)| json!([word.to_utf8().unwrap(), count]))
                    .collect()
            };
            assert_eq!(
                pairs(dictionary.complete(&"".into()).unwrap()),
                row["complete"],
                "operation {index}"
            );
            let predictions: Vec<_> = ["w0002", "context", "new"]
                .into_iter()
                .map(|previous| pairs(dictionary.predict(&previous.into(), 5)))
                .collect();
            assert_eq!(json!(predictions), row["predictions"], "operation {index}");
        }
    }
}
