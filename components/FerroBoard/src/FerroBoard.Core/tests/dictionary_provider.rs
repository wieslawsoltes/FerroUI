use ferroboard_core::text::*;
use serde_json::{Value, json};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
    sync::Arc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/dictionary-provider.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn request(word: &Value) -> SuggestionRequest {
    let r = SuggestionRequest::default();
    r.set_word(Some(Utf16Text::from_units(
        word.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )));
    r
}
fn result(value: SuggestionResult<SuggestionList>) -> Value {
    match value {
        Ok(values) => {
            json!({"value":values.snapshot().unwrap().into_iter().map(|s|{let s=s.unwrap();json!({"text":s.text().as_ref().map(Utf16Text::units),"kind":s.kind().0,"confidence":format!("{:016x}",s.confidence().to_bits()),"auto":s.is_auto_commit(),"removable":s.is_removable()})}).collect::<Vec<_>>()})
        }
        Err(SuggestionError::NullReference) => json!({"error":"NullReferenceException"}),
        Err(SuggestionError::NullDefaultPredictions) => json!({"error":"ArgumentNullException"}),
        Err(error) => panic!("unexpected source query error: {error:?}"),
    }
}
#[test]
fn nullable_dictionary_construction_queries_learning_and_reference_identity_match_source() {
    let rows = rows();
    for row in rows.iter().filter(|v| v["tag"] == "construction") {
        let dictionary = row["hasDictionary"].as_bool().unwrap().then(|| {
            Arc::new(
                WordDictionary::from_json(
                    "{\"hello\":200,\"help\":150,\"world\":180}",
                    "en".into(),
                )
                .unwrap(),
            )
        });
        let user = row["hasUser"]
            .as_bool()
            .unwrap()
            .then(UserDictionary::default);
        let p = DictionarySuggestionProvider::new_nullable(dictionary.clone(), user.clone());
        assert_eq!(
            match (p.dictionary(), dictionary.as_ref()) {
                (None, None) => true,
                (Some(a), Some(b)) => Arc::ptr_eq(&a, b),
                _ => false,
            },
            row["dictionarySame"]
        );
        assert_eq!(
            user.as_ref()
                .is_some_and(|user| user.ptr_eq(&p.user_dictionary())),
            row["userSame"]
        );
        assert_eq!(
            p.max_correction_distance(),
            row["distance"].as_f64().unwrap()
        );
        assert_eq!(
            p.auto_correct_threshold(),
            row["threshold"].as_f64().unwrap()
        );
        assert_eq!(
            p.rare_word_frequency_gap(),
            row["gap"].as_i64().unwrap() as i32
        );
        let defaults = p.default_predictions().unwrap();
        assert_eq!(
            json!(
                defaults
                    .iter()
                    .unwrap()
                    .map(|v| v.unwrap().as_ref().map(|s| s.units().to_vec()))
                    .collect::<Vec<_>>()
            ),
            row["predictions"]
        );
        for query in rows.iter().filter(|v| {
            v["tag"] == "query"
                && v["hasDictionary"] == row["hasDictionary"]
                && v["hasUser"] == row["hasUser"]
        }) {
            assert_eq!(
                result(p.suggest(&request(&query["word"]))),
                query["result"],
                "{query}"
            );
        }
        let learned = rows
            .iter()
            .find(|v| {
                v["tag"] == "learn"
                    && v["hasDictionary"] == row["hasDictionary"]
                    && v["hasUser"] == row["hasUser"]
            })
            .unwrap();
        p.learn(&"custom".into(), Some(&"hello".into()));
        p.learn(&"custom".into(), Some(&"hello".into()));
        p.forget(&"world".into());
        assert_eq!(
            p.user_dictionary().get_count(&"custom".into()),
            learned["count"]
        );
        assert_eq!(
            p.user_dictionary().is_blocked(&"world".into()),
            learned["blocked"]
        );
        assert_eq!(p.user_dictionary().export(), learned["export"]);
        let r = SuggestionRequest::default();
        r.set_preceding_text(Some("hello ".into()));
        assert_eq!(result(p.suggest(&r)), learned["learned"]);
    }
}
#[test]
fn nullable_default_entries_and_query_precedence_match_source_without_normalization() {
    for row in rows().iter().filter(|v| v["tag"] == "nullable-predictions") {
        let p = DictionarySuggestionProvider::new_nullable(None, None);
        let list = Rc::new(PredictionListBuffer::from_nullable(vec![
            None,
            Some(Utf16Text::from_units(vec![0xd800])),
            Some("".into()),
            Some("😀".into()),
        ]));
        let erased: PredictionList = list.clone();
        p.set_default_predictions(Some(erased.clone()));
        assert!(Rc::ptr_eq(&erased, &p.default_predictions().unwrap()));
        let r = SuggestionRequest::default();
        r.set_max_count(row["max"].as_i64().unwrap() as i32);
        assert_eq!(result(p.suggest(&r)), row["before"]);
        list.set(0, "changed".into()).unwrap();
        assert_eq!(result(p.suggest(&r)), row["after"]);
    }
    let row = rows()
        .into_iter()
        .find(|v| v["tag"] == "null-predictions")
        .unwrap();
    let p = DictionarySuggestionProvider::new_nullable(None, None);
    p.set_default_predictions(None);
    let r = SuggestionRequest::default();
    assert_eq!(result(p.suggest(&r)), row["empty"]);
    r.set_preceding_text(Some("hello ".into()));
    assert_eq!(result(p.suggest(&r)), row["context"]);
    r.set_word(Some("he".into()));
    assert_eq!(result(p.suggest(&r)), row["nonempty"]);
}
#[test]
fn unconstrained_tuning_retains_exact_floating_bits_and_integer_extremes() {
    for row in rows().iter().filter(|v| v["tag"] == "tuning") {
        let bits = u64::from_str_radix(row["bits"].as_str().unwrap(), 16).unwrap();
        let p = DictionarySuggestionProvider::new_nullable(None, None);
        p.set_max_correction_distance(f64::from_bits(bits));
        p.set_auto_correct_threshold(f64::from_bits(bits));
        p.set_rare_word_frequency_gap(i32::MIN);
        assert_eq!(
            format!("{:016x}", p.max_correction_distance().to_bits()),
            row["distance"]
        );
        assert_eq!(
            format!("{:016x}", p.auto_correct_threshold().to_bits()),
            row["threshold"]
        );
        assert_eq!(p.rare_word_frequency_gap(), row["gap"]);
    }
}
#[test]
fn default_replacement_releases_native_captures_outside_provider_borrow() {
    struct Probe {
        provider: Weak<DictionarySuggestionProvider>,
        dropped: Rc<Cell<bool>>,
    }
    impl PartialEq for Probe {
        fn eq(&self, other: &Self) -> bool {
            std::ptr::eq(self, other)
        }
    }
    impl PredictionSource for Probe {
        fn len(&self) -> SuggestionResult<usize> {
            Ok(0)
        }
        fn get(&self, index: usize) -> SuggestionResult<Option<Utf16Text>> {
            Err(SuggestionError::PredictionIndex(index))
        }
    }
    impl Drop for Probe {
        fn drop(&mut self) {
            let provider = self.provider.upgrade().unwrap();
            assert!(provider.default_predictions().is_none());
            provider.set_default_predictions(Some(Rc::new(PredictionListBuffer::new(vec![
                "reentered".into(),
            ]))));
            self.dropped.set(true);
        }
    }
    let provider = Rc::new(DictionarySuggestionProvider::new_nullable(None, None));
    let dropped = Rc::new(Cell::new(false));
    provider.set_default_predictions(Some(Rc::new(Probe {
        provider: Rc::downgrade(&provider),
        dropped: dropped.clone(),
    })));
    provider.set_default_predictions(None);
    assert!(dropped.get());
    assert_eq!(
        provider.default_predictions().unwrap().get(0).unwrap(),
        Some("reentered".into())
    );
}
