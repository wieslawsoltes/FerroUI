use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonString,
    collections::{RibbonArray, RibbonReadOnlyList, RibbonReadOnlyListImpl},
    model::{RibbonItemsSource, RibbonIterator, RibbonObject},
    search::*,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/search.json")).unwrap()
}
fn read(v: &Value) -> Option<RibbonString> {
    v.as_array().map(|v| {
        RibbonString::from_utf16(
            v.iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>(),
        )
    })
}
fn dump(v: Option<RibbonString>) -> Value {
    v.map_or(Value::Null, |v| json!(v.as_utf16()))
}
fn s(v: &str) -> Option<RibbonString> {
    Some(v.into())
}
fn strings(v: &Value) -> RibbonSearchStrings {
    RibbonFixedItems::new(v.as_array().unwrap().iter().map(read).collect()) as RibbonReadOnlyList<_>
}
fn entry(v: &Value) -> Option<Rc<RibbonSearchEntry>> {
    (!v.is_null()).then(|| {
        RibbonSearchEntry::with_options(
            read(&v["id"]),
            read(&v["label"]),
            RibbonSearchEntryOptions {
                path: read(&v["path"]),
                description: read(&v["description"]),
                keywords: (!v["keywords"].is_null()).then(|| strings(&v["keywords"])),
                shortcut: read(&v["shortcut"]),
                is_enabled: v["IsEnabled"].as_bool().unwrap(),
                ..Default::default()
            },
        )
    })
}
fn entry_dump(e: Option<Rc<RibbonSearchEntry>>) -> Value {
    e.map_or(Value::Null,|e|json!({"id":dump(e.id()),"label":dump(e.label()),"path":dump(e.path()),"description":dump(e.description()),"keywords":e.keywords().map(|k|k.iter().map(dump).collect::<Vec<_>>()),"shortcut":dump(e.shortcut()),"IsEnabled":e.is_enabled()}))
}
fn results(v: RibbonSearchResults) -> Value {
    json!(v.iter().map(|r|{let r=r.unwrap();json!({"entry":entry_dump(r.entry()),"score":format!("{:016X}",r.score().to_bits())})}).collect::<Vec<_>>())
}
struct ComparerGuard(Option<Rc<RibbonSearchLabelComparerProvider>>);
impl Drop for ComparerGuard {
    fn drop(&mut self) {
        RibbonSearchEngine::set_label_comparer_provider(self.0.take());
    }
}
// Oracle keys isolate the original ranking algorithm from the separate missing
// platform collation service. This comparer is confined to tests.
fn comparer(row: &Value) -> ComparerGuard {
    let keys = row["keys"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                read(&r["label"]).unwrap().as_utf16().to_vec(),
                r["key"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u8)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    ComparerGuard(RibbonSearchEngine::set_label_comparer_provider(Some(
        Rc::new(move || {
            let keys = keys.clone();
            Rc::new(move |a, b| {
                keys.get(a.as_utf16())
                    .expect("unknown oracle label")
                    .cmp(keys.get(b.as_utf16()).expect("unknown oracle label"))
            })
        }),
    )))
}
#[test]
fn normalization_preserves_source_decomposition_folding_separators_and_invalid_utf16() {
    for row in corpus()["unicode"]["strings"].as_array().unwrap() {
        let input = read(&row["input"]);
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonSearchEngine::normalize(input.as_ref())
        }));
        assert_eq!(result.is_err(), !row["error"].is_null(), "{row}");
        if let Ok(result) = result {
            assert_eq!(dump(Some(result)), row["output"], "{row}");
        }
    }
    assert_eq!(
        RibbonSearchEngine::normalize(s("Łącz").as_ref()),
        RibbonString::from("lacz")
    );
    assert_eq!(
        RibbonSearchEngine::normalize(s("Straße").as_ref()),
        RibbonString::from("strasse")
    );
    assert_eq!(
        RibbonSearchEngine::normalize(s("Sort & Filter!").as_ref()),
        RibbonString::from("sort filter")
    );
}
#[test]
fn entry_score_tiers_weights_terms_and_failure_order_match_source_bits() {
    for row in corpus()["scores"]["entries"].as_array().unwrap() {
        let entry = entry(&row["entry"]);
        let terms = strings(&row["terms"]);
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonSearchEngine::score_entry(entry.as_deref(), Some(&terms))
        }));
        assert_eq!(result.is_err(), !row["error"].is_null(), "{row}");
        if let Ok(score) = result {
            assert_eq!(
                json!(format!("{:016X}", score.to_bits())),
                row["score"],
                "{row}"
            );
        }
    }
}
#[test]
fn original_search_queries_recent_boost_disabled_penalty_limits_and_result_order_match_source() {
    let data = corpus();
    let _guard = comparer(&data["collation"][0]);
    // Source SearchTests catalog is frozen in every successful standard result.
    let mut by_id = HashMap::new();
    for row in data["scores"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .take(7 * 12)
        .step_by(12)
    {
        let e = entry(&row["entry"]).unwrap();
        by_id.insert(e.id().unwrap().to_utf8().unwrap(), e);
    }
    let ordered = [
        "paste",
        "format-painter",
        "bold",
        "font-color",
        "insert-table",
        "pivot",
        "page-color",
    ]
    .map(|id| Some(by_id[id].clone()))
    .to_vec();
    for row in data["search"].as_array().unwrap() {
        let engine = RibbonSearchEngine::new();
        engine.set_entries(Some(RibbonItemsSource::from_values(ordered.clone())));
        engine.set_recent(Some(RibbonItemsSource::from_values(
            row["recent"].as_array().unwrap().iter().map(read).collect(),
        )));
        let query = read(&row["query"]);
        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.search(query.as_ref(), row["max"].as_i64().unwrap() as i32)
        }));
        assert_eq!(result.is_err(), !row["error"].is_null(), "{row}");
        if let Ok(result) = result {
            assert_eq!(results(result), row["results"], "{row}");
        }
    }
}
#[test]
fn culture_tie_breaking_uses_source_keys_and_stable_entry_order_for_eight_cultures() {
    for row in corpus()["collation"].as_array().unwrap() {
        let _guard = comparer(row);
        let engine = RibbonSearchEngine::new();
        engine.set_entries(Some(RibbonItemsSource::from_values(
            row["entries"]
                .as_array()
                .unwrap()
                .iter()
                .map(entry)
                .collect(),
        )));
        assert_eq!(
            results(engine.search(s("tie").as_ref(), 100)),
            row["results"],
            "{}",
            row["culture"]
        );
    }
}
fn snapshot(engine: &RibbonSearchEngine) -> Value {
    json!({"entries":engine.entries().iter().map(entry_dump).collect::<Vec<_>>(),"recent":engine.recent().iter().map(dump).collect::<Vec<_>>(),"MaxRecent":engine.max_recent()})
}
fn throw_after<T: Clone + 'static>(items: Vec<T>) -> RibbonItemsSource<T> {
    RibbonItemsSource::new(move || {
        Box::new(
            items
                .clone()
                .into_iter()
                .chain(std::iter::once_with(|| panic!("InvalidOperation: input"))),
        )
    })
}
#[test]
fn source_index_and_recent_mutations_keep_identity_partial_failures_and_null_rules() {
    let engine = RibbonSearchEngine::new();
    let entries = engine.entries();
    let recent = engine.recent();
    for row in corpus()["state"].as_array().unwrap() {
        let action = row["action"].as_str().unwrap();
        let result = catch_unwind(AssertUnwindSafe(|| match action {
            "replace" => engine.set_entries(Some(RibbonItemsSource::from_values(vec![
                Some(RibbonSearchEntry::new(s("a"), s("A"))),
                Some(RibbonSearchEntry::new(s("b"), s("B"))),
                Some(RibbonSearchEntry::new(s("a"), s("ignored"))),
                Some(RibbonSearchEntry::new(s("A"), s("case"))),
                Some(RibbonSearchEntry::new(None, s("null"))),
            ]))),
            "add" => engine.add(Some(RibbonSearchEntry::new(s("a"), s("A2")))),
            "add-null-id" => engine.add(Some(RibbonSearchEntry::new(None, s("null2")))),
            "null-add" => engine.add(None),
            "recent" => {
                engine.set_max_recent(2);
                engine.set_recent(Some(RibbonItemsSource::from_values(vec![
                    s("a"),
                    s("b"),
                    s("a"),
                    s("c"),
                    None,
                    s(""),
                    s(" A "),
                ])));
            }
            "mark" => engine.mark_used(s("b")),
            "null-set-recent" => engine.set_recent(None),
            "set-limit" => engine.set_max_recent(0),
            "mark-empty" => engine.mark_used(s("")),
            "mark-null" => engine.mark_used(None),
            "mark-space" => engine.mark_used(s(" ")),
            "negative-limit" => {
                engine.set_max_recent(-1);
                engine.set_recent(Some(RibbonItemsSource::from_values(vec![s("a"), s("b")])));
            }
            "partial-recent" => {
                engine.set_max_recent(1);
                engine.set_recent(Some(throw_after(vec![s("a"), s("b"), s("c")])));
            }
            "partial-entries" => engine.set_entries(Some(throw_after(vec![
                Some(RibbonSearchEntry::new(s("x"), s("X"))),
                Some(RibbonSearchEntry::new(s("y"), s("Y"))),
            ]))),
            "self-entries" => {
                let entries = entries.clone();
                engine.set_entries(Some(RibbonItemsSource::new(move || {
                    Box::new(entries.iter())
                })));
            }
            "self-recent" => {
                let recent = recent.clone();
                engine.set_recent(Some(RibbonItemsSource::new(move || {
                    Box::new(recent.iter())
                })));
            }
            "null-set-entries" => engine.set_entries(None),
            "first-null" => engine.set_entries(Some(RibbonItemsSource::from_values(vec![None]))),
            "after-null" => engine.add(Some(RibbonSearchEntry::new(s("z"), s("Z")))),
            "null-after-good" => engine.set_entries(Some(RibbonItemsSource::from_values(vec![
                Some(RibbonSearchEntry::new(s("ok"), s("OK"))),
                None,
            ]))),
            _ => panic!("unmapped {action}"),
        }));
        assert_eq!(result.is_err(), !row["error"].is_null(), "{action}");
        assert_eq!(snapshot(&engine), row["state"], "{action}");
        assert_eq!(
            json!(Rc::ptr_eq(&entries, &engine.entries())),
            row["sameEntries"]
        );
        assert_eq!(
            json!(Rc::ptr_eq(&recent, &engine.recent())),
            row["sameRecent"]
        );
    }
}
struct Terms {
    mode: String,
    calls: Cell<usize>,
    trace: RefCell<Vec<String>>,
}
impl RibbonReadOnlyListImpl<Option<RibbonString>> for Terms {
    fn count(&self) -> i32 {
        self.trace.borrow_mut().push("count".into());
        self.calls.set(self.calls.get() + 1);
        if self.mode == "zero"
            || (self.calls.get() > 1 && (self.mode == "denom-zero" || self.mode == "empty-zero"))
        {
            0
        } else if self.mode == "denom-negative" && self.calls.get() > 1 {
            -2
        } else if self.mode == "denom" && self.calls.get() > 1 {
            2
        } else {
            1
        }
    }
    fn get(&self, index: i32) -> Option<RibbonString> {
        self.trace.borrow_mut().push(format!("get:{index}"));
        s("x")
    }
    fn iter_owned(self: Rc<Self>) -> RibbonIterator<Option<RibbonString>> {
        self.trace.borrow_mut().push("enumerate".into());
        if self.mode == "empty" || self.mode == "empty-zero" {
            Box::new(std::iter::empty())
        } else {
            Box::new(std::iter::once(s("x")))
        }
    }
}
#[test]
fn score_terms_count_enumeration_and_keyword_snapshot_match_source_order() {
    for row in corpus()["live"].as_array().unwrap() {
        let native = Rc::new(Terms {
            mode: row["mode"].as_str().unwrap().into(),
            calls: Cell::new(0),
            trace: RefCell::new(vec![]),
        });
        let terms = native.clone() as RibbonReadOnlyList<_>;
        let entry = RibbonSearchEntry::new(s("x"), s("X"));
        let score = RibbonSearchEngine::score_entry(Some(&entry), Some(&terms));
        assert_eq!(json!(format!("{:016X}", score.to_bits())), row["score"]);
        assert_eq!(json!(*native.trace.borrow()), row["trace"]);
    }
    struct Changing(RibbonArray<Option<RibbonString>>);
    impl RibbonReadOnlyListImpl<Option<RibbonString>> for Changing {
        fn count(&self) -> i32 {
            1
        }
        fn get(&self, _: i32) -> Option<RibbonString> {
            s("x")
        }
        fn iter_owned(self: Rc<Self>) -> RibbonIterator<Option<RibbonString>> {
            let values = self.0.clone();
            Box::new(std::iter::once_with(move || {
                values.set(0, s("Y"));
                s("x")
            }))
        }
    }
    let data = corpus();
    let expected = &data["keywordSnapshot"];
    let array = RibbonFixedItems::new(vec![s("X")]);
    let terms = Rc::new(Changing(array.clone())) as RibbonReadOnlyList<_>;
    let entry = RibbonSearchEntry::with_options(
        s("k"),
        s("Q"),
        RibbonSearchEntryOptions {
            keywords: Some(array.clone() as RibbonReadOnlyList<_>),
            ..Default::default()
        },
    );
    assert_eq!(
        json!(format!(
            "{:016X}",
            RibbonSearchEngine::score_entry(Some(&entry), Some(&terms)).to_bits()
        )),
        expected["first"]
    );
    assert_eq!(
        json!(format!(
            "{:016X}",
            RibbonSearchEngine::score_entry(Some(&entry), Some(&terms)).to_bits()
        )),
        expected["second"]
    );
    assert_eq!(
        array.get(0).unwrap().to_utf8().unwrap(),
        expected["keywords"][0]
    );
}
#[test]
fn recent_entry_query_starts_late_reads_current_index_and_invalidates_on_recent_mutation() {
    let data = corpus();
    let expected = &data["recentQuery"];
    let engine = RibbonSearchEngine::new();
    engine.set_entries(Some(RibbonItemsSource::from_values(vec![
        Some(RibbonSearchEntry::new(s("a"), s("A"))),
        Some(RibbonSearchEntry::new(s("b"), s("B"))),
    ])));
    engine.set_recent(Some(RibbonItemsSource::from_values(vec![s("a"), s("b")])));
    let query = engine.clone().get_recent_entries();
    let before = query.iter();
    engine.mark_used(s("b"));
    assert_eq!(
        json!(
            before
                .map(|e| e.label().unwrap().to_utf8().unwrap())
                .collect::<Vec<_>>()
        ),
        expected["before"]
    );
    let mut iterator = query.iter();
    let first = iterator.next().unwrap().label().unwrap().to_utf8().unwrap();
    engine.add(Some(RibbonSearchEntry::new(s("a"), s("changed"))));
    let second = iterator.next().unwrap().label().unwrap().to_utf8().unwrap();
    assert_eq!(json!(first), expected["first"]);
    assert_eq!(json!(second), expected["second"]);
    let mut active = query.iter();
    active.next();
    engine.mark_used(s("a"));
    assert!(catch_unwind(AssertUnwindSafe(|| active.next())).is_err());
    assert_eq!(expected["error"]["type"], "InvalidOperationException");
}
#[test]
fn null_raw_entries_labels_recent_ids_and_zero_take_preserve_source_failures() {
    for row in corpus()["edgeSearch"].as_array().unwrap() {
        let engine = RibbonSearchEngine::new();
        match row["mode"].as_str().unwrap() {
            "null-label" => engine.add(Some(RibbonSearchEntry::with_options(
                s("n"),
                None,
                RibbonSearchEntryOptions {
                    keywords: Some(RibbonFixedItems::new(vec![s("x")]) as RibbonReadOnlyList<_>),
                    ..Default::default()
                },
            ))),
            "invalid-label" => engine.add(Some(RibbonSearchEntry::with_options(
                s("n"),
                Some(RibbonString::from_utf16(vec![0xd800])),
                RibbonSearchEntryOptions {
                    keywords: Some(RibbonFixedItems::new(vec![s("x")]) as RibbonReadOnlyList<_>),
                    ..Default::default()
                },
            ))),
            "null-entry" => engine.entries().add(None),
            "raw-recent" => {
                engine.add(Some(RibbonSearchEntry::new(None, s("X"))));
                engine.recent().add(None);
            }
            _ => unreachable!(),
        }
        let query = row["query"].as_str().unwrap().into();
        let result = catch_unwind(AssertUnwindSafe(|| {
            engine.search(Some(&query), row["max"].as_i64().unwrap() as i32)
        }));
        assert_eq!(result.is_err(), !row["error"].is_null(), "{row}");
        if let Ok(result) = result {
            assert_eq!(results(result), row["results"], "{row}");
        }
    }
}
#[test]
fn record_copies_share_keyword_and_payload_identity_and_result_nan_equality() {
    let data = corpus();
    let expected = &data["records"];
    let keywords = RibbonFixedItems::new(vec![s("x")]);
    let payload = Rc::new(12_i32);
    let options = RibbonSearchEntryOptions {
        keywords: Some(keywords.clone() as RibbonReadOnlyList<_>),
        target: Some(RibbonObject::reference(payload.clone())),
        ..Default::default()
    };
    let entry = RibbonSearchEntry::with_options(s("id"), s("Label"), options.clone());
    let copy = entry.clone_record();
    let (id, label, path, description, kw, shortcut, target, enabled) = entry.deconstruct();
    assert_eq!(dump(id), expected["id"]);
    assert_eq!(dump(label), expected["label"]);
    assert_eq!(dump(path), expected["path"]);
    assert_eq!(dump(description), expected["description"]);
    assert_eq!(dump(shortcut), expected["shortcut"]);
    assert_eq!(json!(enabled), expected["enabled"]);
    assert!(Rc::ptr_eq(&kw.unwrap(), &entry.keywords().unwrap()));
    assert!(target.unwrap().ptr_eq(&entry.target().unwrap()));
    assert_eq!(json!(entry == copy), expected["equal"]);
    assert_eq!(json!(!Rc::ptr_eq(&entry, &copy)), expected["newRecord"]);
    assert_eq!(
        json!(Rc::ptr_eq(
            &entry.keywords().unwrap(),
            &(keywords as RibbonReadOnlyList<_>)
        )),
        expected["keywords"]
    );
    assert!(Rc::ptr_eq(
        &entry.target().unwrap().payload().downcast::<i32>().unwrap(),
        &payload
    ));
    let other = RibbonSearchEntry::with_options(
        s("id"),
        s("Label"),
        RibbonSearchEntryOptions {
            keywords: Some(RibbonFixedItems::new(vec![s("x")]) as RibbonReadOnlyList<_>),
            ..options.clone()
        },
    );
    let same = RibbonSearchEntry::with_options(s("id"), s("Label"), options);
    assert_eq!(json!(entry == other), expected["distinctKeywordsEqual"]);
    assert_eq!(json!(entry == same), expected["sharedKeywordsEqual"]);
    let result = RibbonSearchResult::new(Some(entry.clone()), f64::NAN);
    let copy = result.clone_record();
    assert_eq!(json!(result == copy), expected["resultEqual"]);
    assert_eq!(json!(!Rc::ptr_eq(&result, &copy)), expected["resultNew"]);
    assert!(Rc::ptr_eq(&result.entry().unwrap(), &entry));
}
#[test]
fn missing_culture_comparer_fails_explicitly_instead_of_returning_ordinal_ranking() {
    let _guard = ComparerGuard(RibbonSearchEngine::set_label_comparer_provider(None));
    let engine = RibbonSearchEngine::new();
    let options = RibbonSearchEntryOptions {
        keywords: Some(RibbonFixedItems::new(vec![s("tie")]) as RibbonReadOnlyList<_>),
        ..Default::default()
    };
    engine.add(Some(RibbonSearchEntry::with_options(
        s("a"),
        s("áb"),
        options.clone(),
    )));
    engine.add(Some(RibbonSearchEntry::with_options(
        s("z"),
        s("zb"),
        options,
    )));
    let error = catch_unwind(AssertUnwindSafe(|| engine.search(s("tie").as_ref(), 12)))
        .err()
        .unwrap();
    let text = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(text.contains("current-culture search collation"));
}

#[test]
fn native_replace_entries_accepts_standard_iterators_and_retains_source_index_rules() {
    let engine = RibbonSearchEngine::new();
    let entries = engine.entries();
    let old = RibbonSearchEntry::new(s("old"), s("Old"));
    engine.add(Some(old));
    let first = RibbonSearchEntry::new(s("same"), s("First"));
    let duplicate = RibbonSearchEntry::new(s("same"), s("Duplicate"));
    let next = RibbonSearchEntry::new(s("next"), s("Next"));
    let mut input = vec![first.clone(), duplicate, next.clone()].into_iter();
    let seen = engine.clone();
    let mut first_poll = true;
    engine.replace_entries(std::iter::from_fn(move || {
        if first_poll {
            assert!(
                seen.entries().is_empty(),
                "clear occurs before the first iterator poll"
            );
            first_poll = false;
        }
        input.next()
    }));
    assert!(
        Rc::ptr_eq(&entries, &engine.entries()),
        "collection identity survives replacement"
    );
    assert_eq!(entries.count(), 2);
    assert!(Rc::ptr_eq(entries.get(0).as_ref().unwrap(), &first));
    assert!(Rc::ptr_eq(entries.get(1).as_ref().unwrap(), &next));
    engine.replace_entries(std::iter::empty());
    assert_eq!(entries.count(), 0);
}
