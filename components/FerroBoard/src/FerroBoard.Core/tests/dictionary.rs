use ferroboard_core::text::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
fn text(value: &Value) -> Utf16Text {
    Utf16Text::from_units(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn pairs(value: Vec<(Utf16Text, i32)>) -> Value {
    json!(
        value
            .iter()
            .map(|(w, f)| json!({"word":w.units(),"frequency":f}))
            .collect::<Vec<_>>()
    )
}
fn fuzzy(value: Vec<FuzzyMatch>) -> Value {
    json!(value.iter().map(|p|json!({"word":p.word.units(),"frequency":p.frequency,"distance":format!("{:016x}",p.distance.to_bits())})).collect::<Vec<_>>())
}
fn error(value: DictionaryError) -> &'static str {
    match value {
        DictionaryError::Normalization(_) => "ArgumentException",
        DictionaryError::Json(ferroboard_core::serialization::JsonBridgeError::Syntax {
            ..
        }) => "JsonReaderException",
        DictionaryError::Json(ferroboard_core::serialization::JsonBridgeError::NumberFormat) => {
            "FormatException"
        }
        DictionaryError::Json(_) | DictionaryError::CollectionModified => {
            "InvalidOperationException"
        }
        DictionaryError::EmptyTextParts => "IndexOutOfRangeException",
        DictionaryError::CompletionIndex => "ArgumentOutOfRangeException",
        e => panic!("unexpected {e:?}"),
    }
}
fn result<T>(r: DictionaryResult<T>, f: impl FnOnce(T) -> Value) -> Value {
    match r {
        Ok(v) => json!({"value":f(v)}),
        Err(e) => json!({"error":error(e)}),
    }
}
fn snapshot(d: WordDictionary) -> Value {
    let pairs: Vec<_> = d
        .words()
        .into_iter()
        .map(|w| {
            d.get_variants(&w)
                .unwrap()
                .into_iter()
                .find(|(word, _)| word == &w)
                .unwrap()
        })
        .collect();
    json!({"words":self::pairs(pairs),"max":d.max_frequency()})
}
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/dictionary.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn build(row: &Value) -> WordDictionary {
    WordDictionary::from_words(
        row["words"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (text(&p["word"]), p["frequency"].as_i64().unwrap() as i32)),
        "tr".into(),
    )
    .unwrap()
}
fn float(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
#[test]
fn dictionary_reference_loading_lookup_completion_and_fuzzy() {
    let rows = rows();
    let d = build(&rows[0]);
    let english = WordDictionary::load_bundled_english().unwrap();
    for (i, row) in rows.iter().enumerate() {
        let kind = row["kind"].as_str().unwrap();
        match kind {
            "words" => {
                assert_eq!(d.count(), row["count"].as_u64().unwrap() as usize);
                assert_eq!(d.max_frequency(), row["max"].as_i64().unwrap() as i32);
                assert_eq!(
                    json!(d.words().iter().map(|w| w.units()).collect::<Vec<_>>()),
                    row["all"]
                );
                assert_eq!(
                    d.language_tag().unwrap().to_utf8().unwrap(),
                    row["language"]
                );
            }
            "lookup" => {
                let word = text(&row["word"]);
                assert_eq!(
                    result(d.get_frequency(&word), |f| json!(f)),
                    row["frequency"],
                    "frequency {i}"
                );
                assert_eq!(
                    result(d.get_variants(&word), pairs),
                    row["variants"],
                    "variants {i}"
                );
                assert_eq!(
                    result(
                        WordDictionary::normalize(&word).map_err(DictionaryError::from),
                        |w| json!(w.units())
                    ),
                    row["normalized"],
                    "normalize {i}"
                );
            }
            "complete" => assert_eq!(
                result(
                    d.complete(&text(&row["word"]), row["max"].as_i64().unwrap() as i32),
                    pairs
                ),
                row["result"],
                "completion {i}"
            ),
            "fuzzy" => assert_eq!(
                result(
                    d.find_similar(
                        &text(&row["word"]),
                        float(&row["distance"]),
                        row["max"].as_i64().unwrap() as i32,
                        None
                    ),
                    fuzzy
                ),
                row["result"],
                "fuzzy {i}"
            ),
            "text" => assert_eq!(
                result(
                    WordDictionary::from_text(&text(&row["text"]), "en".into()),
                    snapshot
                ),
                row["result"],
                "text {i}"
            ),
            "json" => {
                assert_eq!(
                    result(
                        WordDictionary::from_json(row["text"].as_str().unwrap(), "en".into()),
                        snapshot
                    ),
                    row["result"],
                    "json {i}"
                );
            }
            "english" => {
                assert_eq!(english.count(), row["count"].as_u64().unwrap() as usize);
                assert_eq!(english.max_frequency(), row["max"].as_i64().unwrap() as i32);
            }
            "english-query" => {
                let word = text(&row["word"]);
                assert_eq!(
                    json!(english.get_frequency(&word).unwrap()),
                    row["frequency"]
                );
                assert_eq!(
                    pairs(english.complete(&word, 5).unwrap()),
                    row["complete"],
                    "english completion {i}"
                );
                assert_eq!(
                    fuzzy(english.find_similar(&word, 1.0, 10, None).unwrap()),
                    row["fuzzy"],
                    "english fuzzy {i}"
                );
            }
            "random" => {
                let d = build(row);
                for probe in row["probes"].as_array().unwrap() {
                    let word = text(&probe["word"]);
                    let max = probe["max"].as_i64().unwrap() as i32;
                    assert_eq!(
                        result(d.complete(&word, max), pairs),
                        probe["complete"],
                        "random complete {i} {probe}"
                    );
                    assert_eq!(
                        result(
                            d.find_similar(&word, float(&probe["distance"]), max, None),
                            fuzzy
                        ),
                        probe["fuzzy"],
                        "random fuzzy {i} {probe}"
                    );
                    assert_eq!(json!(d.get_frequency(&word).unwrap()), probe["frequency"]);
                    assert_eq!(pairs(d.get_variants(&word).unwrap()), probe["variants"]);
                }
            }
            "callback" | "reentrant" => {}
            _ => panic!("unknown {kind}"),
        }
    }
}
#[test]
fn dictionary_reference_custom_costs_and_reentrant_callbacks() {
    for row in rows() {
        match row["kind"].as_str().unwrap() {
            "callback" => {
                let d = WordDictionary::from_words(
                    [
                        ("ab".into(), 2),
                        ("ba".into(), 4),
                        ("bb".into(), 1),
                        ("a".into(), 3),
                    ],
                    "en".into(),
                )
                .unwrap();
                let calls = RefCell::new(Vec::new());
                let cost = float(&row["cost"]);
                let callback = |a, b| {
                    calls.borrow_mut().push([a, b]);
                    cost
                };
                assert_eq!(
                    result(
                        d.find_similar(&"ab".into(), 1.0, 10, Some(&callback)),
                        fuzzy
                    ),
                    row["result"],
                    "{row}"
                );
                assert_eq!(json!(*calls.borrow()), row["calls"]);
            }
            "reentrant" => {
                let d = WordDictionary::from_words(
                    [("ab".into(), 2), ("ac".into(), 4), ("ba".into(), 1)],
                    "en".into(),
                )
                .unwrap();
                let calls = RefCell::new(Vec::new());
                let ran = Cell::new(false);
                let callback = |a, b| {
                    calls.borrow_mut().push([a, b]);
                    if !ran.get() && (row["mode"] != "late-descendant" || calls.borrow().len() == 2)
                    {
                        ran.set(true);
                        match row["mode"].as_str().unwrap() {
                            "existing" => d.add(&"ac".into(), 50).unwrap(),
                            "root" => d.add(&"z".into(), 6).unwrap(),
                            "descendant" | "late-descendant" => d.add(&"ad".into(), 6).unwrap(),
                            "terminal" => d.add(&"AB".into(), 8).unwrap(),
                            "nested" => {
                                d.find_similar(&"ac".into(), 1.0, 2, None).unwrap();
                            }
                            "throw" => panic!("custom extension callback"),
                            _ => unreachable!(),
                        }
                    }
                    1.0
                };
                let actual = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    d.find_similar(&"ab".into(), 2.0, 10, Some(&callback))
                })) {
                    Ok(r) => result(r, fuzzy),
                    Err(_) => json!({"error":"ApplicationException"}),
                };
                assert_eq!(actual, row["result"], "{row}");
                assert_eq!(json!(*calls.borrow()), row["calls"]);
                assert_eq!(
                    json!(d.words().iter().map(|w| w.units()).collect::<Vec<_>>()),
                    row["words"]
                ); // Lock is usable after exception unwinding.
                assert_eq!(d.count(), row["words"].as_array().unwrap().len());
            }
            _ => {}
        }
    }
}
#[test]
fn ordinal_runtime_scalar_and_pair_corpus() {
    let rows: Vec<Value> = include_str!("../../../tests/fixtures/ordinal.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let mut hash = Sha256::new();
    let mut exceptions = Vec::new();
    let mut count = 0;
    for cp in 0..=0x10ffff {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let s: Utf16Text = c.to_string().as_str().into();
        let upper = TextCulture::invariant().to_upper(&s);
        let lower = TextCulture::to_lower_invariant(&s);
        let equal = ordinal_ignore_case_equal(&s, &upper);
        hash.update([equal as u8, ordinal_ignore_case_equal(&s, &lower) as u8]);
        if !equal {
            exceptions.push(cp);
        }
        count += 1;
    }
    assert_eq!(count, rows[0]["count"].as_i64().unwrap());
    assert_eq!(json!(exceptions), rows[0]["exceptions"]);
    assert_eq!(
        format!("{:x}", hash.finalize()),
        rows[0]["sha256"].as_str().unwrap()
    );
    for row in &rows[1..] {
        assert_eq!(
            ordinal_ignore_case_equal(&text(&row["a"]), &text(&row["b"])),
            row["equal"].as_bool().unwrap(),
            "{row}"
        );
    }
}
#[test]
fn dictionary_source_tests_and_shared_concurrent_use() {
    let english = WordDictionary::load_bundled_english().unwrap();
    assert!(english.count() > 40000);
    assert!(english.get_frequency(&"the".into()).unwrap() > 200);
    assert!(english.contains(&"Hello".into()).unwrap());
    let results = english.complete(&"th".into(), 5).unwrap();
    assert_eq!(results.len(), 5);
    assert_eq!(results[0].0, Utf16Text::from("the"));
    assert!(results.windows(2).all(|p| p[0].1 >= p[1].1));
    assert!(
        english
            .find_similar(&"helo".into(), 1.0, 10, None)
            .unwrap()
            .iter()
            .any(|m| m.word == Utf16Text::from("hello"))
    );
    assert!(
        english
            .find_similar(&"teh".into(), 1.0, 10, None)
            .unwrap()
            .iter()
            .any(|m| m.word == Utf16Text::from("the"))
    );
    let d = WordDictionary::from_text(&"# comment\nhallo 200\nwelt\n".into(), "en".into()).unwrap();
    assert_eq!(d.get_frequency(&"hallo".into()).unwrap(), 200);
    assert!(d.contains(&"welt".into()).unwrap());
    let shared = std::sync::Arc::new(WordDictionary::default());
    std::thread::scope(|scope| {
        for worker in 0..4 {
            let d = shared.clone();
            scope.spawn(move || {
                for i in 0..100 {
                    d.add(&format!("word{i}").as_str().into(), worker * 100 + i)
                        .unwrap();
                    let _ = d.complete(&"word".into(), 5).unwrap();
                }
            });
        }
    });
    assert_eq!(shared.count(), 100);
    assert_eq!(shared.max_frequency(), 399);
    for i in 0..100 {
        assert_eq!(
            shared
                .get_frequency(&format!("word{i}").as_str().into())
                .unwrap(),
            300 + i
        );
    }
}
