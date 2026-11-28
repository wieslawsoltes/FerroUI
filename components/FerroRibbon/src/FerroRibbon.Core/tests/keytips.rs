use ferroribbon_core::{RibbonString, keytips::*, model::RibbonItemsSource};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};
fn corpus() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/keytips.json")).unwrap()
}
fn s(v: &str) -> Option<RibbonString> {
    Some(v.into())
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
fn values(v: impl IntoIterator<Item = Option<RibbonString>>) -> Value {
    Value::Array(v.into_iter().map(dump).collect())
}
fn strings(v: impl IntoIterator<Item = Option<RibbonString>>) -> Vec<String> {
    v.into_iter()
        .map(|v| v.unwrap().to_utf8().unwrap())
        .collect()
}
#[test]
fn request_value_copy_deconstruction_and_lossless_display_match_source() {
    use std::hash::{DefaultHasher, Hash, Hasher};
    fn hash(value: &RibbonKeyTipRequest) -> u64 {
        let mut state = DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    }
    for row in corpus()["records"].as_array().unwrap() {
        let request = RibbonKeyTipRequest::new(read(&row["label"]), read(&row["tip"]));
        let copy = request.clone();
        let (label, tip) = request.deconstruct();
        assert_eq!(dump(label), row["label"]);
        assert_eq!(dump(tip), row["tip"]);
        assert_eq!(dump(Some(request.to_ribbon_string())), row["display"]);
        assert_eq!(json!(request == copy), row["equal"]);
        // Numeric Rust/CLR hashes are deliberately not equated.
        assert_eq!(json!(hash(&request) == hash(&copy)), row["sameHash"]);
    }
    assert_eq!(
        RibbonKeyTipRequest::default(),
        RibbonKeyTipRequest::new(None, None)
    );
    for row in corpus()["matchEnums"].as_array().unwrap() {
        let value = row["value"].as_i64().unwrap() as i32;
        let result = RibbonKeyTipMatch::from_raw(value);
        assert_eq!(result.raw(), value);
        assert_eq!(json!(result.to_string()), row["display"]);
    }
}
#[test]
fn assignment_order_explicit_conflicts_normalization_and_large_scopes_match_source() {
    for row in corpus()["cases"].as_array().unwrap() {
        let requests = row["requests"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| RibbonKeyTipRequest::new(read(&v["label"]), read(&v["explicitTip"])))
            .collect::<Vec<_>>();
        let reserved = row["reserved"]
            .as_array()
            .map(|v| RibbonItemsSource::from_values(v.iter().map(read).collect()));
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonKeyTipAssigner::assign(Some(&requests), reserved)
        }));
        assert_eq!(result.is_err(), !row["error"].is_null(), "{}", row["name"]);
        if let Ok(result) = result {
            assert_eq!(values(result.iter()), row["result"], "{}", row["name"]);
            let alias = result.clone();
            assert!(std::rc::Rc::ptr_eq(&result, &alias));
        }
    }
}
#[test]
fn quick_access_sequence_and_mutable_list_identity_match_source() {
    for row in corpus()["qat"].as_array().unwrap() {
        let result =
            RibbonKeyTipAssigner::assign_quick_access(row["count"].as_i64().unwrap() as i32);
        assert_eq!(values(result.iter()), row["result"]);
        let alias = result.clone();
        assert!(Rc::ptr_eq(&result, &alias));
        alias.add(None);
        assert!(result.get(result.len() - 1).is_none());
    }
}
struct Requests {
    mode: String,
    calls: Cell<i32>,
    trace: Rc<RefCell<Vec<String>>>,
}
impl RibbonKeyTipRequests for Requests {
    fn count(&self) -> i32 {
        self.trace.borrow_mut().push("count".into());
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if self.mode == "shrink" && call > 1 {
            0
        } else if self.mode == "grow" && call == 1 {
            1
        } else {
            2
        }
    }
    fn get(&self, index: i32) -> RibbonKeyTipRequest {
        self.trace.borrow_mut().push(format!("get:{index}"));
        RibbonKeyTipRequest::new(s(if index == 0 { "Home" } else { "Help" }), None)
    }
}
#[test]
fn live_read_only_request_count_and_indexer_order_match_source() {
    for row in corpus()["requests"].as_array().unwrap() {
        let trace = Rc::new(RefCell::new(vec![]));
        let requests = Requests {
            mode: row["mode"].as_str().unwrap().into(),
            calls: Cell::new(0),
            trace: trace.clone(),
        };
        let result = catch_unwind(AssertUnwindSafe(|| {
            RibbonKeyTipAssigner::assign(Some(&requests), None)
        }));
        assert_eq!(result.is_err(), !row["error"].is_null());
        if let Ok(result) = result {
            assert_eq!(values(result.iter()), row["result"]);
        }
        assert_eq!(json!(*trace.borrow()), row["trace"], "{}", row["mode"]);
    }
}
type Target = Rc<RibbonString>;
fn target(v: &str) -> Option<Target> {
    Some(Rc::new(v.into()))
}
fn target_text(v: Option<Target>) -> Option<String> {
    v.map(|v| v.to_utf8().unwrap())
}
fn candidates(q: RibbonItemsSource<RibbonKeyTipEntry<Target>>) -> Value {
    json!(
        q.iter()
            .map(|e| json!({"tip":dump(e.tip),"Target":target_text(e.target)}))
            .collect::<Vec<_>>()
    )
}
#[test]
fn candidate_query_and_exhausted_iterator_retain_scope_until_released() {
    let data = corpus();
    let expected = &data["lifetime"];
    let scope = RibbonKeyTipScope::<Target>::new();
    let target = Rc::new(RibbonString::from("target"));
    let weak_scope = Rc::downgrade(&scope);
    let weak_target = Rc::downgrade(&target);
    scope.add(s("AB"), Some(target.clone()));
    let query = scope.candidates();
    drop(target);
    let alive = || {
        json!({"scope": weak_scope.upgrade().is_some(),
                          "target": weak_target.upgrade().is_some()})
    };
    assert_eq!(alive(), expected["query"]);
    let mut iterator = query.iter();
    drop(query);
    assert_eq!(alive(), expected["before"]);
    assert_eq!(json!(iterator.next().is_some()), expected["first"]);
    assert_eq!(json!(iterator.next().is_some()), expected["second"]);
    assert_eq!(alive(), expected["exhausted"]);
    drop(iterator);
    assert_eq!(alive(), expected["released"]);
}
#[test]
fn scopes_duplicate_targets_prefix_backspace_and_candidate_laziness_match_source() {
    let data = corpus();
    let expected = &data["scope"];
    let scope = RibbonKeyTipScope::<Target>::new();
    let entries = scope.entries();
    scope.add(s("f p"), target("format painter"));
    scope.add(s("FF"), target("font"));
    scope.add(s("v"), target("paste"));
    scope.add(s("V"), target("duplicate"));
    for row in expected["steps"].as_array().unwrap() {
        let action = row["action"].as_str().unwrap();
        let (result, selected, back) = if let Some(key) = action.strip_prefix("key:") {
            let (m, t) = scope.process(key.parse().unwrap());
            (Some(m.raw()), target_text(t), None)
        } else if action == "back" {
            (None, None, Some(scope.backspace()))
        } else {
            scope.reset();
            (None, None, None)
        };
        assert_eq!(json!(result), row["match"]);
        assert_eq!(json!(selected), row["target"]);
        assert_eq!(json!(back), row["back"]);
        assert_eq!(dump(Some(scope.typed())), row["typed"]);
        assert_eq!(candidates(scope.clone().candidates()), row["candidates"]);
    }
    assert_eq!(
        Rc::ptr_eq(&entries, &scope.entries()),
        expected["same"].as_bool().unwrap()
    );
    let mut current = scope.clone().candidates().iter();
    let first = current.next().unwrap().tip.unwrap().to_utf8().unwrap();
    assert_eq!(json!(first), expected["first"]);
    scope.process(b'f' as u16);
    let rest = current
        .map(|e| e.tip.unwrap().to_utf8().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(json!(rest), expected["rest"]);
    let before = scope.clone().candidates();
    let before = before.iter();
    scope.add(s("X"), target("later"));
    assert_eq!(
        json!(
            before
                .map(|e| e.tip.unwrap().to_utf8().unwrap())
                .collect::<Vec<_>>()
        ),
        expected["beforeValues"]
    );
    scope.reset();
    let mut active = scope.clone().candidates().iter();
    active.next();
    scope.add(s("Y"), target("latest"));
    assert!(catch_unwind(AssertUnwindSafe(|| active.next())).is_err());
    assert_eq!(expected["activeError"], "InvalidOperationException");
    let nav = RibbonKeyTipNavigator::new();
    nav.push(Some(scope.clone()));
    scope.process(b'f' as u16);
    nav.push(Some(scope.clone()));
    nav.push(None);
    for (i, row) in expected["stack"].as_array().unwrap().iter().enumerate() {
        if i > 0 {
            assert_eq!(json!(nav.pop()), row["remains"]);
        }
        assert_eq!(json!(nav.depth()), row["Depth"]);
        assert_eq!(json!(nav.is_active()), row["IsActive"]);
        assert_eq!(json!(nav.current().is_none()), row["current"]);
        assert_eq!(dump(Some(scope.typed())), row["typed"]);
    }
    nav.push(Some(scope.clone()));
    nav.clear();
    assert_eq!(nav.depth(), 0);
    assert!(!nav.is_active());
    assert!(nav.current().is_none());
    assert_eq!(dump(Some(scope.typed())), expected["cleared"]["typed"]);
}
#[test]
fn raw_backing_list_exposes_null_tuple_and_utf16_receiver_behavior() {
    let data = corpus();
    let expected = &data["raw"];
    let scope = RibbonKeyTipScope::<Target>::new();
    let entries = scope.entries();
    entries.add(RibbonKeyTipEntry {
        tip: s("A"),
        target: None,
    });
    entries.add(RibbonKeyTipEntry {
        tip: s("A"),
        target: target("second"),
    });
    let (m, t) = scope.process(b'a' as u16);
    assert_eq!(json!(m.raw()), expected["first"]);
    assert_eq!(json!(target_text(t)), expected["target"]);
    assert_eq!(dump(Some(scope.typed())), expected["typed"]);
    scope.reset();
    entries.clear();
    entries.add(RibbonKeyTipEntry {
        tip: Some(RibbonString::from_utf16(vec![0xd83d, 0xde00])),
        target: target("emoji"),
    });
    let (m, _) = scope.process(0xd83d);
    assert_eq!(json!(m.raw()), expected["high"]);
    assert_eq!(dump(Some(scope.typed())), expected["partial"]);
    let (m, t) = scope.process(0xde00);
    assert_eq!(json!(m.raw()), expected["low"]);
    assert_eq!(json!(target_text(t)), expected["emoji"]);
    scope.reset();
    entries.add(RibbonKeyTipEntry {
        tip: None,
        target: target("null tip"),
    });
    assert!(
        catch_unwind(AssertUnwindSafe(|| scope
            .candidates()
            .iter()
            .collect::<Vec<_>>()))
        .is_err()
    );
    assert_eq!(expected["error"], "NullReferenceException");
}
#[test]
fn target_validation_precedes_tip_validation_and_null_assignment_precedes_reserved() {
    for row in corpus()["errors"].as_array().unwrap() {
        let scope = RibbonKeyTipScope::<Target>::new();
        let input = row["target"].as_str().and_then(target);
        let result = catch_unwind(AssertUnwindSafe(|| scope.add(read(&row["tip"]), input)));
        let error = result.err().unwrap();
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap();
        if row["param"] == "target" {
            assert!(message.contains("ArgumentNull: target"));
        } else {
            assert!(message.contains("ArgumentException") && message.contains("(tip)"));
        }
        assert_eq!(
            scope.entries().len(),
            row["count"].as_u64().unwrap() as usize
        );
    }
    let called = Rc::new(Cell::new(false));
    let marker = called.clone();
    let reserved = RibbonItemsSource::new(move || {
        marker.set(true);
        Box::new(std::iter::empty())
    });
    assert!(
        catch_unwind(AssertUnwindSafe(|| RibbonKeyTipAssigner::assign(
            None,
            Some(reserved)
        )))
        .is_err()
    );
    assert!(!called.get());
}
#[test]
fn original_keytip_tests_and_regressions_remain_prefix_free() {
    fn assign(labels: &[&str]) -> Vec<String> {
        let q = labels
            .iter()
            .map(|v| RibbonKeyTipRequest::new(s(v), None))
            .collect::<Vec<_>>();
        strings(RibbonKeyTipAssigner::assign(Some(&q), None).iter())
    }
    fn prefix_free(tips: &[String]) {
        for (i, a) in tips.iter().enumerate() {
            for (j, b) in tips.iter().enumerate() {
                if i != j {
                    assert!(!b.starts_with(a), "{a} / {b}");
                }
            }
        }
    }
    assert_eq!(assign(&["Home", "Insert", "Design"]), ["H", "I", "D"]);
    prefix_free(&assign(&[
        "Paste",
        "Page Layout",
        "Print",
        "Picture",
        "Pen",
        "Paragraph",
    ]));
    let empty = assign(&["", "", ""]);
    assert_eq!(empty.len(), 3);
    prefix_free(&empty);
    assert!(empty.iter().all(|t| !t.is_empty()));
    let bold = assign(&vec!["Bold"; 80]);
    assert_eq!(bold.len(), 80);
    prefix_free(&bold);
    assert_eq!(assign(&["Éditer"]), ["E"]);
    let requests = [
        RibbonKeyTipRequest::new(s("Home"), s("H")),
        RibbonKeyTipRequest::new(s("Help"), None),
        RibbonKeyTipRequest::new(s("File menu"), None),
    ];
    let explicit = strings(
        RibbonKeyTipAssigner::assign(
            Some(&requests),
            Some(RibbonItemsSource::from_values(vec![s("F")])),
        )
        .iter(),
    );
    assert_eq!(explicit[0], "H");
    assert_ne!(explicit[1], "H");
    assert!(!explicit[2].starts_with('F'));
    for row in corpus()["cases"].as_array().unwrap() {
        if row["name"].as_str().unwrap().starts_with("large:") {
            let requests = row["requests"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| RibbonKeyTipRequest::new(read(&r["label"]), read(&r["explicitTip"])))
                .collect::<Vec<_>>();
            let tips = strings(RibbonKeyTipAssigner::assign(Some(&requests), None).iter());
            assert_eq!(tips.len(), requests.len());
            assert!(tips.iter().all(|t| !t.is_empty()));
            prefix_free(&tips);
        }
    }
    assert_eq!(
        strings(RibbonKeyTipAssigner::assign_quick_access(12).iter()),
        [
            "1", "2", "3", "4", "5", "6", "7", "8", "9", "09", "08", "07"
        ]
    );
    let qat = strings(RibbonKeyTipAssigner::assign_quick_access(80).iter());
    assert_eq!(qat.len(), 80);
    assert_eq!(qat[0], "1");
    assert_eq!(qat[9], "09");
    assert_eq!(qat[18], "0A");
    prefix_free(&qat);
    assert!(RibbonKeyTipAssigner::assign_quick_access(-3).is_empty());
    let scope = RibbonKeyTipScope::<Target>::new();
    scope.add(s("AB"), target("x"));
    scope.process(b'a' as u16);
    assert!(scope.backspace());
    assert!(!scope.backspace());
    let navigator = RibbonKeyTipNavigator::new();
    navigator.push(Some(scope));
    navigator.push(Some(RibbonKeyTipScope::new()));
    assert_eq!(navigator.depth(), 2);
    assert!(navigator.pop());
    assert!(!navigator.pop());
    assert!(!navigator.is_active());
}
