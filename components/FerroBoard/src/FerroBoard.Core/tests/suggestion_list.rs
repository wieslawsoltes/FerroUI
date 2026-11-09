use ferroboard_core::{editor::*, engine::*, layouts::*, settings::*, text::*};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
type Log = Rc<RefCell<Vec<String>>>;
struct Provider {
    enabled: Cell<bool>,
    value: RefCell<Option<SuggestionList>>,
    log: Log,
}
impl PartialEq for Provider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for Provider {
    fn suggest(&self, _: Option<&SuggestionRequest>) -> SuggestionResult<Option<SuggestionList>> {
        if !self.enabled.get() {
            return Ok(Some(SuggestionList::default()));
        }
        self.log.borrow_mut().push("suggest".into());
        Ok(self.value.borrow().clone())
    }
    fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
}
struct Custom {
    profile: String,
    log: Log,
    engine: Weak<KeyboardEngine>,
    values: Vec<Rc<Suggestion>>,
}
impl SuggestionSource for Custom {
    fn len(&self) -> SuggestionResult<usize> {
        self.log.borrow_mut().push("len".into());
        if self.profile == "count-fault" {
            Err(SuggestionError::Custom("count failure".into()))
        } else {
            Ok(3)
        }
    }
    fn get(&self, index: usize) -> SuggestionResult<SuggestionValue> {
        self.log.borrow_mut().push(format!("index:{index}"));
        Err(SuggestionError::Custom("indexed access unavailable".into()))
    }
    fn iter(&self) -> SuggestionResult<SuggestionIter<'_>> {
        self.log.borrow_mut().push("iter".into());
        if self.profile == "get-fault" {
            return Err(SuggestionError::Custom("iterator creation failed".into()));
        }
        Ok(Box::new(Cursor {
            profile: self.profile.clone(),
            log: self.log.clone(),
            engine: self.engine.clone(),
            values: self.values.clone(),
            index: 0,
        }))
    }
}
struct Cursor {
    profile: String,
    log: Log,
    engine: Weak<KeyboardEngine>,
    values: Vec<Rc<Suggestion>>,
    index: usize,
}
impl Iterator for Cursor {
    type Item = SuggestionResult<SuggestionValue>;
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        if index >= self.values.len() {
            return None;
        }
        self.index += 1;
        self.log.borrow_mut().push(format!("next:{index}"));
        if (self.profile == "move-first" && index == 0)
            || (matches!(
                self.profile.as_str(),
                "move-second" | "move-and-dispose" | "current-second"
            ) && index == 1)
        {
            return Some(Err(SuggestionError::Custom("iteration failed".into())));
        }
        if index == 1 {
            let engine = self.engine.upgrade().unwrap();
            match self.profile.as_str() {
                "detach" => engine.detach().unwrap(),
                "disable" => engine.settings().set_show_suggestions(false),
                _ => {}
            }
        }
        Some(Ok(Some(self.values[index].clone())))
    }
}
impl Drop for Cursor {
    fn drop(&mut self) {
        self.log.borrow_mut().push("drop".into());
    }
}
#[test]
fn native_provider_results_preserve_source_candidate_order_partial_failure_and_reentrancy() {
    let mut matched = 0;
    let mut retired_runtime_rows = 0;
    for line in include_str!("../../../tests/fixtures/suggestion-list.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let profile = row["profile"].as_str().unwrap();
        // Managed capacity/CopyTo and negative collection counts are not native contracts.
        // Keep the oracle unchanged; compare keyboard outcomes separately from runtime facets.
        if profile.starts_with("collection") || profile == "negative-count" {
            retired_runtime_rows += 1;
            continue;
        }
        let max = row["max"].as_i64().unwrap() as i32;
        let clipboard = row["clipboard"].as_bool().unwrap();
        let reported = row["reported"].as_i64().unwrap() as i32;
        let log = Log::default();
        let settings = Rc::new(KeyboardSettings::default());
        settings.set_auto_capitalization(false);
        settings.set_auto_correct(false);
        settings.set_show_suggestions(true);
        settings.set_next_word_prediction(true);
        settings.set_emoji_suggestions(false);
        settings.set_clipboard_suggestion(clipboard);
        settings.set_glide_typing(false);
        let engine = KeyboardEngine::new_explicit(
            settings.clone(),
            Rc::new(KeyboardResources::create_empty()),
            LanguageModels::new(),
        )
        .unwrap();
        let target = Rc::new(TextBufferTarget::new(
            if clipboard { "".into() } else { "ab".into() },
            None,
        ));
        target.select(if clipboard { 0 } else { 2 }, 0);
        let a = Suggestion::new("first".into(), SuggestionKind::Completion);
        a.set_confidence(0.8);
        let first = Rc::new(a);
        let b = Suggestion::new("second".into(), SuggestionKind::Correction);
        b.set_confidence(0.7);
        let second = Rc::new(b);
        let custom = Rc::new(Custom {
            profile: profile.into(),
            log: log.clone(),
            engine: Rc::downgrade(&engine),
            values: vec![first.clone(), second.clone(), first.clone()],
        });
        let provider = Rc::new(Provider {
            enabled: Cell::new(false),
            value: RefCell::new(None),
            log: log.clone(),
        });
        let p = provider.clone();
        engine.set_suggestion_provider_factory(Some(Rc::new(move |_| Ok(Some(p.clone())))));
        if clipboard {
            engine.record_clipboard(&"clip".into()).unwrap()
        }
        engine.attach(target).unwrap();
        settings.set_suggestion_count(max);
        let before = engine.suggestions();
        let events = Rc::new(RefCell::new(Vec::<Value>::new()));
        let trace = events.clone();
        engine.suggestions_changed().add(Rc::new(move |e| {
            trace.borrow_mut().push(
                json!({"attached":e.target().is_some(),"count":e.suggestions().len().unwrap()}),
            )
        }));
        *provider.value.borrow_mut() = match profile {
            "null" => None,
            "array" => Some(SuggestionList::from_nullable(vec![
                Some(first.clone()),
                Some(second.clone()),
                Some(first.clone()),
            ])),
            "list" => Some(SuggestionList::new(vec![
                first.clone(),
                second.clone(),
                first.clone(),
            ])),
            "null-first" => Some(SuggestionList::from_nullable(vec![
                None,
                Some(first.clone()),
            ])),
            "null-middle" => Some(SuggestionList::from_nullable(vec![
                Some(first.clone()),
                None,
                Some(second.clone()),
            ])),
            _ => Some(SuggestionList::from_custom(custom.clone())),
        };
        provider.enabled.set(true);
        log.borrow_mut().clear();
        events.borrow_mut().clear();
        let error = engine.update_suggestions().err().map(|e| match e {
            EngineError::NullReference => "NullReferenceException",
            e => panic!("unmeasured error {e:?}"),
        });
        let saved = engine.suggestions();
        let direct_count = provider.value.borrow().as_ref().map(|v| match v.len() {
            Ok(n) => json!({"value":n}),
            Err(_) => json!({"error":"ApplicationException"}),
        });
        let values:Vec<Value>=saved.snapshot().unwrap().iter().map(|v|v.as_ref().map_or(Value::Null,|v|json!({"text":v.text().expect("source list corpus candidates have nonnull text").to_utf8().unwrap(),"kind":v.kind().0,"sameFirst":Rc::ptr_eq(v,&first),"sameSecond":Rc::ptr_eq(v,&second)}))).collect();
        let actual = json!({"profile":profile,"max":max,"clipboard":clipboard,"reported":reported,
            "directCount":direct_count,"error":error,"events":*events.borrow(),
            "attached":engine.target().is_some(),"enabled":settings.show_suggestions(),"values":values,
            "sameProvider":provider.value.borrow().as_ref().is_some_and(|v|v.ptr_eq(&saved)),
            "sameBefore":saved.ptr_eq(&before),"sameCurrent":saved.ptr_eq(&engine.suggestions())});
        let expected: serde_json::Map<String, Value> = actual
            .as_object()
            .unwrap()
            .keys()
            .map(|key| (key.clone(), row[key].clone()))
            .collect();
        assert_eq!(
            actual,
            Value::Object(expected),
            "provider result case{matched}"
        );
        if !matches!(
            profile,
            "null" | "array" | "list" | "null-first" | "null-middle"
        ) {
            assert_eq!(
                log.borrow()
                    .iter()
                    .filter(|event| event.as_str() == "drop")
                    .count(),
                usize::from(profile != "get-fault")
            );
        }
        engine.dispose().unwrap();
        matched += 1;
    }
    assert_eq!(matched, 90);
    assert_eq!(retired_runtime_rows, 126);
}

#[test]
fn native_iterator_results_preserve_order_duplicates_identity_and_nullable_values() {
    let first = Rc::new(Suggestion::new("first".into(), SuggestionKind::Completion));
    let second = Rc::new(Suggestion::new("second".into(), SuggestionKind::Correction));
    let visited = Cell::new(0);
    let source = [first.clone(), second.clone(), first.clone()];
    let values: SuggestionList = source
        .into_iter()
        .inspect(|_| visited.set(visited.get() + 1))
        .collect();
    assert_eq!(visited.get(), 3);
    assert_eq!(values.len().unwrap(), 3);
    for (index, expected) in [first.clone(), second.clone(), first.clone()]
        .into_iter()
        .enumerate()
    {
        assert!(Rc::ptr_eq(&values.get(index).unwrap().unwrap(), &expected));
    }
    let shared = values.clone();
    assert!(values.ptr_eq(&shared));
    let nullable: SuggestionList = [Some(first.clone()), None, Some(first)]
        .into_iter()
        .collect();
    assert_eq!(nullable.len().unwrap(), 3);
    assert!(nullable.get(1).unwrap().is_none());
    let empty: SuggestionList = std::iter::empty::<Rc<Suggestion>>().collect();
    assert_eq!(empty.len().unwrap(), 0);
}

#[test]
fn native_fallible_iterator_failure_is_returned_before_result_publication() {
    let candidate = Rc::new(Suggestion::new("first".into(), SuggestionKind::Completion));
    let dropped = Rc::downgrade(&candidate);
    let result: SuggestionResult<SuggestionList> = [
        Ok(candidate),
        Err(SuggestionError::Custom("provider input".into())),
    ]
    .into_iter()
    .collect();
    assert!(matches!(result, Err(SuggestionError::Custom(message)) if message == "provider input"));
    assert!(
        dropped.upgrade().is_none(),
        "partial native iterator results release candidates on failure"
    );
}

#[test]
fn native_provider_iterator_errors_preserve_clipboard_and_allow_recovery() {
    #[derive(PartialEq)]
    struct IteratorProvider {
        fail: Cell<bool>,
        candidate: Rc<Suggestion>,
    }
    impl ISuggestionProvider for IteratorProvider {
        fn suggest(
            &self,
            _: Option<&SuggestionRequest>,
        ) -> SuggestionResult<Option<SuggestionList>> {
            [Ok(self.candidate.clone())]
                .into_iter()
                .chain(
                    self.fail
                        .get()
                        .then(|| Err(SuggestionError::Custom("input failed".into()))),
                )
                .collect::<SuggestionResult<SuggestionList>>()
                .map(Some)
        }
        fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
            Ok(())
        }
        fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
            Ok(())
        }
    }
    let settings = Rc::new(KeyboardSettings::default());
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_emoji_suggestions(false);
    settings.set_glide_typing(false);
    settings.set_clipboard_suggestion(true);
    settings.set_next_word_prediction(true);
    let engine = KeyboardEngine::new_explicit(
        settings,
        Rc::new(KeyboardResources::create_empty()),
        LanguageModels::new(),
    )
    .unwrap();
    let candidate = Rc::new(Suggestion::new("native".into(), SuggestionKind::Prediction));
    let provider = Rc::new(IteratorProvider {
        fail: Cell::new(true),
        candidate: candidate.clone(),
    });
    let retained = provider.clone();
    engine.set_suggestion_provider_factory(Some(Rc::new(move |_| Ok(Some(retained.clone())))));
    engine.record_clipboard(&"clipboard".into()).unwrap();
    engine
        .attach(Rc::new(TextBufferTarget::new("".into(), None)))
        .unwrap();
    assert_eq!(engine.suggestions().len().unwrap(), 1);
    assert_eq!(
        engine.suggestions().get(0).unwrap().unwrap().kind(),
        SuggestionKind::Clipboard
    );
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    engine.suggestions_changed().add(Rc::new(move |e| {
        observed.borrow_mut().push(e.suggestions().len().unwrap())
    }));
    provider.fail.set(false);
    engine.update_suggestions().unwrap();
    assert_eq!(engine.suggestions().len().unwrap(), 2);
    assert!(Rc::ptr_eq(
        &engine.suggestions().get(1).unwrap().unwrap(),
        &candidate
    ));
    provider.fail.set(true);
    engine.update_suggestions().unwrap();
    assert_eq!(engine.suggestions().len().unwrap(), 1);
    assert_eq!(*events.borrow(), [2, 1]);
    engine.dispose().unwrap();
}

#[test]
fn shared_native_values_preserve_identity_and_snapshot_iteration_without_borrow_guards() {
    let first = Rc::new(Suggestion::new("first".into(), SuggestionKind::Completion));
    let values = Rc::new(RefCell::new(vec![Some(first.clone()), None]));
    let list = SuggestionList::from_shared(values.clone());
    let alias = SuggestionList::from_shared(values.clone());
    assert!(list.ptr_eq(&alias));
    assert!(Rc::ptr_eq(&list.shared_values().unwrap(), &values));
    let mut snapshot = list.iter().unwrap();
    values.borrow_mut().clear();
    assert!(alias.is_empty().unwrap());
    assert!(Rc::ptr_eq(
        &snapshot.next().unwrap().unwrap().unwrap(),
        &first
    ));
    assert!(snapshot.next().unwrap().unwrap().is_none());
    assert!(snapshot.next().is_none());
    assert!(matches!(
        list.get(0),
        Err(SuggestionError::SuggestionIndex(0))
    ));
    assert!(!SuggestionList::default().ptr_eq(&SuggestionList::default()));
}
