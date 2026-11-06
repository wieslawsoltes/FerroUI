use ferroboard_core::text::*;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

fn units(value: &Value) -> Option<Utf16Text> {
    value.as_array().map(|a| {
        Utf16Text::from_units(
            a.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn state(request: &SuggestionRequest) -> Value {
    json!({"word":request.word().map(|v|v.units().to_vec()),"preceding":request.preceding_text().map(|v|v.units().to_vec()),"max":request.max_count(),"allow":request.allow_auto_correct(),"culture":request.culture().map(|v|v.name()),"proximity":request.key_proximity().is_some()})
}
fn query(provider: &DictionarySuggestionProvider, request: Option<&SuggestionRequest>) -> Value {
    match provider.suggest_nullable(request) {
        Ok(values) => {
            json!({"value":values.snapshot().unwrap().into_iter().map(|v|v.expect("dictionary candidates are nonnull")).map(|v|json!({"text":v.text().as_ref().map(Utf16Text::units),"kind":v.kind().0,"confidence":format!("{:016x}",v.confidence().to_bits()),"auto":v.is_auto_commit(),"description":v.description().as_ref().map(|v|v.units()),"removable":v.is_removable()})).collect::<Vec<_>>()})
        }
        Err(SuggestionError::NullReference) => {
            json!({"error":"NullReferenceException","parameter":null})
        }
        Err(
            SuggestionError::Normalization(_)
            | SuggestionError::Dictionary(DictionaryError::Normalization(_)),
        ) => json!({"error":"ArgumentException","parameter":"strInput"}),
        Err(e) => panic!("unmeasured request error {e:?}"),
    }
}
#[derive(Default)]
struct Recorder {
    retained: RefCell<Option<SuggestionRequest>>,
    calls: Cell<i32>,
}
impl PartialEq for Recorder {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ISuggestionProvider for Recorder {
    fn suggest(
        &self,
        request: Option<&SuggestionRequest>,
    ) -> SuggestionResult<Option<SuggestionList>> {
        let result = {
            self.calls.set(self.calls.get() + 1);
            *self.retained.borrow_mut() = request.cloned();
            Ok(vec![])
        };
        result.map(|values| Some(SuggestionList::new(values)))
    }
    fn learn(&self, _: Option<&Utf16Text>, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
    fn forget(&self, _: Option<&Utf16Text>) -> SuggestionResult<()> {
        Ok(())
    }
}
#[test]
fn source_retained_requests_nulls_init_copies_and_reentrant_queries() {
    let dictionary = Arc::new(
        WordDictionary::from_words(
            [
                ("hello", 255),
                ("help", 180),
                ("hell", 10),
                ("I", 200),
                ("the", 250),
            ]
            .into_iter()
            .map(|(w, f)| (w.into(), f)),
            "".into(),
        )
        .unwrap(),
    );
    let provider = DictionarySuggestionProvider::new(dictionary, None);
    let original: KeyProximity = Rc::new(|_, _| 0.5);
    let replacement: KeyProximity = Rc::new(|_, _| 1.25);
    let mut matched = 0;
    let mut source_only = 0;
    for line in include_str!("../../../tests/fixtures/suggestion-request.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let actual = match row["tag"].as_str().unwrap() {
            "metadata" => {
                source_only += 1;
                continue;
            }
            "defaults" => json!({"tag":"defaults","state":state(&SuggestionRequest::default())}),
            "null-request" => json!({"tag":"null-request","result":query(&provider,None)}),
            "custom-null" => {
                let recorder = Recorder::default();
                let custom: &dyn ISuggestionProvider = &recorder;
                custom.suggest(None).unwrap();
                json!({"tag":"custom-null","calls":recorder.calls.get(),"retainedNull":recorder.retained.borrow().is_none()})
            }
            "init" => {
                let request = SuggestionRequest::from_values(SuggestionRequestValues {
                    word: Some("helo".into()),
                    preceding_text: Some("say ".into()),
                    culture: Some(TextCulture::get("en-US").unwrap()),
                    key_proximity: Some(original.clone()),
                    ..Default::default()
                });
                let recorder = Recorder::default();
                let custom: &dyn ISuggestionProvider = &recorder;
                custom.suggest(Some(&request)).unwrap();
                let copy = SuggestionRequest::from_values(request.values());
                let variant = row["variant"].as_u64().unwrap() as usize;
                match row["field"].as_str().unwrap() {
                    field @ ("Word" | "PrecedingText") => {
                        let value = match variant {
                            0 => None,
                            1 => Some("".into()),
                            2 => Some(Utf16Text::from_units(vec![
                                97, 0xd800, 0xd83d, 0xde00, 0xdfff,
                            ])),
                            _ => Some("changed".into()),
                        };
                        if field == "Word" {
                            request.set_word(value)
                        } else {
                            request.set_preceding_text(value)
                        }
                    }
                    "MaxCount" => request.set_max_count([i32::MIN, 0, 1, i32::MAX][variant]),
                    "AllowAutoCorrect" => request.set_allow_auto_correct(variant.is_multiple_of(2)),
                    "Culture" => request.set_culture(match variant {
                        0 => None,
                        1 => Some(TextCulture::invariant()),
                        2 => Some(TextCulture::get("tr").unwrap()),
                        _ => Some(TextCulture::get("en-US").unwrap()),
                    }),
                    "KeyProximity" => request.set_key_proximity(match variant {
                        0 => None,
                        1 => Some(original.clone()),
                        _ => Some(replacement.clone()),
                    }),
                    field => panic!("unmeasured field {field}"),
                }
                let retained = recorder.retained.borrow();
                let retained = retained.as_ref().unwrap();
                json!({"tag":"init","field":row["field"],"variant":variant,"retainedSame":request.ptr_eq(retained),"copySame":request.ptr_eq(&copy),"copyProximitySame":Rc::ptr_eq(&copy.key_proximity().unwrap(),&original),"state":state(&request),"retained":state(retained),"copy":state(&copy)})
            }
            "query" => {
                let values = &row["state"];
                let request = SuggestionRequest::from_values(SuggestionRequestValues {
                    word: units(&values["word"]),
                    preceding_text: units(&values["preceding"]),
                    max_count: values["max"].as_i64().unwrap() as i32,
                    allow_auto_correct: values["allow"].as_bool().unwrap(),
                    culture: values["culture"]
                        .as_str()
                        .map(|c| TextCulture::get(c).unwrap()),
                    key_proximity: None,
                });
                json!({"tag":"query","state":state(&request),"result":query(&provider,Some(&request))})
            }
            "reentrant" => {
                let request = SuggestionRequest::from_values(SuggestionRequestValues {
                    word: Some("helo".into()),
                    preceding_text: Some("say ".into()),
                    ..Default::default()
                });
                let calls = Rc::new(RefCell::new(Vec::<[u16; 2]>::new()));
                let replacements = Rc::new(Cell::new(0));
                let next: KeyProximity = {
                    let count = replacements.clone();
                    Rc::new(move |_, _| {
                        count.set(count.get() + 1);
                        1.25
                    })
                };
                let weak = request.downgrade();
                let trace = calls.clone();
                let action = row["action"].as_str().unwrap().to_string();
                let once = Cell::new(false);
                request.set_key_proximity(Some(Rc::new(move |a, b| {
                    trace.borrow_mut().push([a, b]);
                    if !once.replace(true) {
                        let request = weak.upgrade().expect("request retained through callback");
                        match action.as_str() {
                            "Word" => request.set_word(Some("HELO".into())),
                            "PrecedingText" => request.set_preceding_text(None),
                            "MaxCount" => request.set_max_count(1),
                            "AllowAutoCorrect" => request.set_allow_auto_correct(false),
                            "Culture" => request.set_culture(None),
                            "KeyProximity" => request.set_key_proximity(Some(next.clone())),
                            _ => unreachable!(),
                        }
                    }
                    0.5
                })));
                let first = query(&provider, Some(&request));
                let first_calls = std::mem::take(&mut *calls.borrow_mut());
                let second = query(&provider, Some(&request));
                json!({"tag":"reentrant","action":row["action"],"first":first,"firstCalls":first_calls,"second":second,"secondCalls":*calls.borrow(),"replacementCalls":replacements.get(),"state":state(&request)})
            }
            tag => panic!("unmeasured tag {tag}"),
        };
        assert_eq!(actual, row, "request corpus row {matched}");
        matched += 1;
    }
    assert_eq!((matched, source_only), (96, 6));
}
