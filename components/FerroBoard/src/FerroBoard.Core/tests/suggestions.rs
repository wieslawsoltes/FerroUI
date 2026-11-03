use ferroboard_core::{serialization::JsonBridgeError, text::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
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
    include_str!("../../../tests/fixtures/suggestions.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
fn float(v: &Value) -> f64 {
    f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
}
fn suggestions(values: SuggestionList) -> Value {
    json!(values.snapshot().unwrap().into_iter().map(|v|v.expect("dictionary candidates are nonnull")).map(|s|json!({"text":s.text().as_ref().map(Utf16Text::units),"kind":s.kind().0,"confidence":format!("{:016x}",s.confidence().to_bits()),"auto":s.is_auto_commit(),"description":s.description().as_ref().map(|s|s.units()),"removable":s.is_removable()})).collect::<Vec<_>>())
}
fn result(r: SuggestionResult<SuggestionList>) -> Value {
    match r {
        Ok(v) => {
            json!({"value":suggestions(v)})
        }
        Err(e) => {
            let error = match e {
                SuggestionError::Normalization(_)
                | SuggestionError::Dictionary(DictionaryError::Normalization(_)) => {
                    "ArgumentException"
                }
                SuggestionError::Dictionary(DictionaryError::CompletionIndex) => {
                    "ArgumentOutOfRangeException"
                }
                SuggestionError::Dictionary(DictionaryError::CollectionModified) => {
                    "InvalidOperationException"
                }
                SuggestionError::Dictionary(DictionaryError::Json(JsonBridgeError::Syntax {
                    ..
                })) => "JsonReaderException",
                SuggestionError::NullDefaultPredictions => "ArgumentNullException",
                SuggestionError::PredictionIndex(_) => "IndexOutOfRangeException",
                SuggestionError::Custom(_) => "ApplicationException",
                e => panic!("unexpected {e:?}"),
            };
            json!({"error":error})
        }
    }
}
fn dictionary(rows: &[Value]) -> Arc<WordDictionary> {
    Arc::new(
        WordDictionary::from_words(
            rows[0]["words"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| (text(&p["word"]), p["frequency"].as_i64().unwrap() as i32)),
            "en".into(),
        )
        .unwrap(),
    )
}
#[test]
fn source_suggestion_queries_and_tuning_match_exact_scores_and_ranking() {
    let rows = rows();
    let dict = dictionary(&rows);
    let mut p = DictionarySuggestionProvider::new(dict.clone(), None);
    let mut count = 0;
    for row in &rows {
        match row["kind"].as_str().unwrap() {
            "profile" => {
                let user = UserDictionary::default();
                user.import(row["user"].as_str().unwrap()).unwrap();
                user.set_learn_threshold(row["threshold"].as_i64().unwrap() as i32);
                p = DictionarySuggestionProvider::new(dict.clone(), Some(user));
            }
            "query" => {
                count += 1;
                let request = SuggestionRequest::from_values(SuggestionRequestValues {
                    word: Some(text(&row["word"])),
                    preceding_text: Some(text(&row["preceding"])),
                    max_count: row["max"].as_i64().unwrap() as i32,
                    allow_auto_correct: row["allow"].as_bool().unwrap(),
                    culture: Some(TextCulture::get(row["culture"].as_str().unwrap()).unwrap()),
                    ..Default::default()
                });
                assert_eq!(
                    result(p.suggest(&request)),
                    row["result"],
                    "query {count} {row}"
                );
            }
            "tuning" => {
                let p = DictionarySuggestionProvider::new(dict.clone(), None);
                p.set_auto_correct_threshold(float(&row["threshold"]));
                p.set_max_correction_distance(float(&row["distance"]));
                p.set_rare_word_frequency_gap(row["gap"].as_i64().unwrap() as i32);
                assert_eq!(
                    result(
                        p.suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
                            word: Some(text(&row["word"])),
                            ..Default::default()
                        }))
                    ),
                    row["result"],
                    "tuning {row}"
                );
            }
            _ => {}
        }
    }
    assert!(count > 1000);
}
#[test]
fn source_proximity_callbacks_can_reenter_and_change_live_provider_state() {
    for row in rows().iter().filter(|r| r["kind"] == "callback") {
        let d = Arc::new(
            WordDictionary::from_words(
                [
                    ("hello".into(), 255),
                    (
                        "help".into(),
                        if row["mode"] == "negative-infinite" {
                            0
                        } else {
                            180
                        },
                    ),
                    ("hell".into(), 10),
                ],
                "en".into(),
            )
            .unwrap(),
        );
        let p = Rc::new(DictionarySuggestionProvider::new(d, None));
        let log = Rc::new(RefCell::new(Vec::new()));
        let calls = log.clone();
        let once = Cell::new(false);
        let mode = row["mode"].as_str().unwrap().to_owned();
        let provider = p.clone();
        let proximity: KeyProximity = Rc::new(move |a, b| {
            calls.borrow_mut().push([a, b]);
            if !once.replace(true) {
                match mode.as_str() {
                    "configure" => {
                        provider.set_auto_correct_threshold(100.0);
                        provider.set_rare_word_frequency_gap(-2);
                    }
                    "learn" => {
                        provider.learn(&"helo".into(), None);
                        provider.learn(&"helo".into(), None);
                    }
                    "block" => provider.forget(&"hello".into()),
                    "nested" => {
                        provider
                            .suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
                                word: Some("help".into()),
                                ..Default::default()
                            }))
                            .unwrap();
                    }
                    "dictionary-update" => provider
                        .dictionary()
                        .unwrap()
                        .add(&"hello".into(), i32::MAX)
                        .unwrap(),
                    "dictionary-insert" => {
                        provider.dictionary().unwrap().add(&"x".into(), 50).unwrap()
                    }
                    "throw" => panic!("custom callback"),
                    _ => {}
                }
            }
            match mode.as_str() {
                "negative" => -0.3,
                "negative-infinite" => f64::NEG_INFINITY,
                "nan" => f64::NAN,
                "infinite" => f64::INFINITY,
                _ => 0.2,
            }
        });
        let actual = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            p.suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
                word: Some("helo".into()),
                key_proximity: Some(proximity),
                ..Default::default()
            }))
        })) {
            Ok(r) => result(r),
            Err(_) => json!({"error":"ApplicationException"}),
        };
        assert_eq!(actual, row["result"], "{row}");
        assert_eq!(json!(*log.borrow()), row["calls"]);
        assert_eq!(p.user_dictionary().export(), row["user"].as_str().unwrap());
    }
}
#[test]
fn learning_filters_utf16_letters_and_default_prediction_identity() {
    let rows = rows();
    let p = DictionarySuggestionProvider::new(dictionary(&rows), None);
    for row in rows.iter().filter(|r| r["kind"] == "learn") {
        p.learn(&text(&row["word"]), Some(&"prev".into()));
        assert_eq!(
            p.user_dictionary().export(),
            row["user"].as_str().unwrap(),
            "{row}"
        );
    }
    let p = DictionarySuggestionProvider::new(dictionary(&rows), None);
    let row = rows.iter().find(|r| r["kind"] == "defaults").unwrap();
    let list = Rc::new(PredictionListBuffer::new(vec![
        "x".into(),
        "x".into(),
        "The".into(),
        "😀".into(),
    ]));
    let erased: PredictionList = list.clone();
    p.set_default_predictions(Some(erased.clone()));
    assert!(Rc::ptr_eq(&p.default_predictions().unwrap(), &erased));
    let request = SuggestionRequest::from_values(SuggestionRequestValues {
        max_count: 5,
        ..Default::default()
    });
    assert_eq!(suggestions(p.suggest(&request).unwrap()), row["before"]);
    list.set(0, "changed".into()).unwrap();
    list.push("last".into());
    assert_eq!(suggestions(p.suggest(&request).unwrap()), row["after"]);
    p.forget(&"changed".into());
    assert_eq!(suggestions(p.suggest(&request).unwrap()), row["blocked"]);
    p.set_default_predictions(None);
    assert_eq!(
        result(p.suggest(&SuggestionRequest::default())),
        row["nullResult"]
    );
    assert_eq!(
        result(
            p.suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
                word: Some("the".into()),
                ..Default::default()
            }))
        ),
        row["nonEmpty"]
    );
}
#[test]
fn original_provider_tests_and_bundled_english_corpus() {
    let p = DictionarySuggestionProvider::new(
        Arc::new(WordDictionary::load_bundled_english().unwrap()),
        None,
    );
    for row in rows().iter().filter(|r| r["kind"] == "english") {
        assert_eq!(
            result(
                p.suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
                    word: Some(text(&row["word"])),
                    ..Default::default()
                }))
            ),
            row["result"],
            "{row}"
        );
    }
    let result = p
        .suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
            word: Some("becuase".into()),
            ..Default::default()
        }))
        .unwrap();
    assert!(
        result
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|v| v.expect("dictionary candidates are nonnull"))
            .any(|s| s.text() == Some(Utf16Text::from("because")) && s.is_auto_commit())
    );
    let result = p
        .suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
            word: Some("Tha".into()),
            ..Default::default()
        }))
        .unwrap();
    assert!(
        result
            .snapshot()
            .unwrap()
            .into_iter()
            .map(|v| v.expect("dictionary candidates are nonnull"))
            .all(|s| s.text().unwrap().units()[0] == b'T' as u16)
    );
}
#[test]
fn models_defaults_record_equality_and_custom_provider_dispatch() {
    let request = SuggestionRequest::default();
    assert!(request.word().unwrap().is_empty());
    assert!(request.preceding_text().unwrap().is_empty());
    assert_eq!(request.max_count(), 3);
    assert!(request.allow_auto_correct());
    assert!(request.key_proximity().is_none());
    assert_eq!(request.culture(), Some(TextCulture::invariant()));
    let a = Suggestion::from_values(SuggestionValues {
        confidence: f64::NAN,
        ..SuggestionValues::new("x".into(), SuggestionKind(999))
    });
    assert_eq!(a, a.clone());
    assert_eq!(SuggestionKind::Clipboard.0, 6);
    struct Custom {
        calls: RefCell<Vec<&'static str>>,
        candidate: Rc<Suggestion>,
    }
    impl PartialEq for Custom {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    impl ISuggestionProvider for Custom {
        fn suggest(
            &self,
            _: Option<&SuggestionRequest>,
        ) -> SuggestionResult<Option<SuggestionList>> {
            let result = {
                self.calls.borrow_mut().push("suggest");
                Ok(vec![self.candidate.clone()])
            };
            result.map(|values| Some(SuggestionList::new(values)))
        }
        fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
            self.calls.borrow_mut().push("learn");
            Ok(())
        }
        fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
            self.calls.borrow_mut().push("forget");
            Err(SuggestionError::Custom("declined".into()))
        }
    }
    let custom = Custom {
        calls: RefCell::new(Vec::new()),
        candidate: Rc::new(Suggestion::new("custom".into(), SuggestionKind::Completion)),
    };
    let provider: &dyn ISuggestionProvider = &custom;
    assert!(Rc::ptr_eq(
        &provider
            .suggest(Some(&request))
            .unwrap()
            .unwrap()
            .get(0)
            .unwrap()
            .unwrap(),
        &custom.candidate
    ));
    provider.learn(Some(&"word".into()), None).unwrap();
    assert!(matches!(
        provider.forget(Some(&"word".into())),
        Err(SuggestionError::Custom(_))
    ));
    assert_eq!(*custom.calls.borrow(), ["suggest", "learn", "forget"]);
    let dictionary = Arc::new(WordDictionary::default());
    let user = UserDictionary::default();
    let p = DictionarySuggestionProvider::new(dictionary.clone(), Some(user.clone()));
    assert!(Arc::ptr_eq(&p.dictionary().unwrap(), &dictionary));
    assert!(p.user_dictionary().ptr_eq(&user));
    assert_eq!(p.max_correction_distance(), 2.0);
    assert_eq!(p.auto_correct_threshold(), 0.55);
    assert_eq!(p.rare_word_frequency_gap(), 70);
    let provider: &dyn ISuggestionProvider = &p;
    provider.learn(Some(&"word".into()), None).unwrap();
    assert_eq!(user.get_count(&"word".into()), 1);
    provider.forget(Some(&"word".into())).unwrap();
    assert!(user.is_blocked(&"word".into()));
}

#[test]
fn provider_learning_matches_source_for_every_utf16_unit() {
    use sha2::{Digest, Sha256};
    let rows = rows();
    let dictionary = dictionary(&rows);
    let expected = rows.iter().find(|r| r["kind"] == "learn-bmp").unwrap();
    let mut hash = Sha256::new();
    for unit in 0..=u16::MAX {
        let p = DictionarySuggestionProvider::new(dictionary.clone(), None);
        let word = Utf16Text::from_units(vec![b'1' as u16, unit]);
        p.learn(&word, None);
        hash.update([p.user_dictionary().get_count(&word) as u8]);
    }
    assert_eq!(
        format!("{:x}", hash.finalize()),
        expected["sha256"].as_str().unwrap()
    );
}
#[test]
fn native_default_prediction_iteration_preserves_source_results_and_replacement() {
    struct TracedList {
        log: Rc<RefCell<Vec<String>>>,
        provider: std::rc::Weak<DictionarySuggestionProvider>,
        throws: bool,
    }
    struct Iter {
        log: Rc<RefCell<Vec<String>>>,
        provider: std::rc::Weak<DictionarySuggestionProvider>,
        index: std::cell::Cell<usize>,
        throws: bool,
    }
    impl Iterator for Iter {
        type Item = SuggestionResult<PredictionValue>;
        fn next(&mut self) -> Option<Self::Item> {
            let index = self.index.get();
            if index == 0 {
                self.log.borrow_mut().push("start".into());
            }
            if index >= 3 {
                return None;
            }
            self.index.set(index + 1);
            self.log.borrow_mut().push(format!("next:{index}"));
            self.provider
                .upgrade()
                .unwrap()
                .set_default_predictions(None);
            if self.throws && index == 1 {
                return Some(Err(SuggestionError::Custom("iterator".into())));
            }
            Some(Ok(Some(format!("item{index}").as_str().into())))
        }
    }
    impl Drop for Iter {
        fn drop(&mut self) {
            self.log.borrow_mut().push("dispose".into());
        }
    }
    impl PartialEq for TracedList {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    impl PredictionSource for TracedList {
        fn len(&self) -> SuggestionResult<usize> {
            self.log.borrow_mut().push("count".into());
            Ok(3)
        }
        fn get(&self, index: usize) -> SuggestionResult<Option<Utf16Text>> {
            self.log.borrow_mut().push(format!("index:{index}"));
            Ok(Some(format!("item{index}").as_str().into()))
        }
        fn iter(&self) -> SuggestionResult<PredictionIter<'_>> {
            Ok(Box::new(Iter {
                log: self.log.clone(),
                provider: self.provider.clone(),
                index: Cell::new(0),
                throws: self.throws,
            }))
        }
    }
    let rows = rows();
    let dictionary = dictionary(&rows);
    for row in rows.iter().filter(|r| r["kind"] == "custom-list") {
        let p = Rc::new(DictionarySuggestionProvider::new(dictionary.clone(), None));
        let log = Rc::new(RefCell::new(Vec::new()));
        let list: PredictionList = Rc::new(TracedList {
            log: log.clone(),
            provider: Rc::downgrade(&p),
            throws: row["mode"] == "throw",
        });
        p.set_default_predictions(Some(list.clone()));
        assert!(Rc::ptr_eq(&p.default_predictions().unwrap(), &list));
        assert_eq!(
            result(
                p.suggest(&SuggestionRequest::from_values(SuggestionRequestValues {
                    max_count: 2,
                    ..Default::default()
                }))
            ),
            row["result"]
        );
        assert_eq!(json!(*log.borrow()), row["log"]);
        assert!(p.default_predictions().is_none());
    }
}

#[test]
fn source_candidate_models_and_default_collection_members() {
    let rows = rows();
    let row = rows.iter().find(|r| r["kind"] == "models").unwrap();
    let p = DictionarySuggestionProvider::new(dictionary(&rows), None);
    let list = p.default_predictions().unwrap();
    assert_eq!(json!(list.len().unwrap()), row["defaultCount"]);
    assert_eq!(
        list.get(0).unwrap().unwrap().to_utf8().unwrap(),
        row["first"].as_str().unwrap()
    );
    let r = list.get(3).map(|_| SuggestionList::new(Vec::new()));
    assert_eq!(result(r), row["end"]);
    let candidate = Suggestion::new("x".into(), SuggestionKind(999));
    assert_eq!(
        suggestions(SuggestionList::new(vec![Rc::new(candidate)])),
        json!([row["candidate"].clone()])
    );
    let zero = Suggestion::new("x".into(), SuggestionKind::Typed);
    assert_eq!(
        zero,
        Suggestion::from_values(SuggestionValues {
            confidence: -0.0,
            ..zero.values()
        })
    );
    let kinds = [
        ("Typed", SuggestionKind::Typed),
        ("Completion", SuggestionKind::Completion),
        ("Correction", SuggestionKind::Correction),
        ("Prediction", SuggestionKind::Prediction),
        ("Emoji", SuggestionKind::Emoji),
        ("Glide", SuggestionKind::Glide),
        ("Clipboard", SuggestionKind::Clipboard),
    ];
    assert_eq!(
        json!(
            kinds
                .iter()
                .map(|(name, kind)| json!({"name":name,"value":kind.0}))
                .collect::<Vec<_>>()
        ),
        row["kinds"]
    );
}

#[test]
fn native_prediction_iteration_snapshots_handles_before_mutation() {
    let list = PredictionListBuffer::new(vec!["a".into(), "b".into()]);
    let mut iter = list.iter().unwrap();
    assert_eq!(iter.next().unwrap().unwrap(), Some(Utf16Text::from("a")));
    list.set(1, "c".into()).unwrap();
    assert_eq!(iter.next().unwrap().unwrap(), Some(Utf16Text::from("b")));
    assert_eq!(list.get(1).unwrap(), Some(Utf16Text::from("c")));
}
