//! Source clock identity, synchronous null/throw/reentrant ordering and undo state.
#[path = "../../../tests/support/fixture_clock.rs"]
mod fixture_clock;
use ferroboard_core::{
    editor::*, engine::KeyboardEngine, layouts::KeyboardResources, text::Utf16Text,
};
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc, sync::Arc};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/editor-clock.jsonl")
        .lines()
        .map(|v| serde_json::from_str(v).unwrap())
        .collect()
}
fn text(v: &Value) -> Utf16Text {
    Utf16Text::from_units(
        v.as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u16)
            .collect::<Vec<_>>(),
    )
}
fn state(e: &EditorController) -> Value {
    json!({"text":e.text().units(),"start":e.selection_start(),"length":e.selection_length().unwrap(),"undo":e.can_undo(),"redo":e.can_redo()})
}
#[test]
fn native_default_clocks_are_owned_by_each_editor_and_engine() {
    let a = EditorController::new(Rc::new(TextBufferTarget::default()));
    let b = EditorController::new(Rc::new(TextBufferTarget::default()));
    let first = a.clock().unwrap();
    assert!(Rc::ptr_eq(&first, &a.clock().unwrap()));
    assert!(!Rc::ptr_eq(&first, &b.clock().unwrap()));
    let now = first();
    let _: std::time::SystemTime = now;
    let make = || {
        KeyboardEngine::new(None, Some(Rc::new(KeyboardResources::create_empty())), None).unwrap()
    };
    let ea = make();
    let eb = make();
    assert!(!Rc::ptr_eq(&ea.clock().unwrap(), &eb.clock().unwrap()));
    ea.dispose().unwrap();
    eb.dispose().unwrap();
}
#[derive(Debug)]
struct ProbeException;
fn apply(e: &EditorController, op: &str) -> EditorResult<Value> {
    match op {
        "empty" => e.commit_text(&"".into())?,
        "commit" => e.commit_text(&"x".into())?,
        "before" => e.replace_before_cursor(1, &"x".into())?,
        "word" => e.replace_current_word(&"x".into())?,
        "backward" => return e.delete_backward().map(|v| json!(v)),
        "wordBackward" => return e.delete_word_backward().map(|v| json!(v)),
        "forward" => return e.delete_forward().map(|v| json!(v)),
        "wordForward" => return e.delete_word_forward().map(|v| json!(v)),
        "zero" => e.delete_characters_before(0)?,
        "delete" => e.delete_characters_before(1)?,
        _ => panic!("operation"),
    };
    Ok(Value::Null)
}
#[test]
fn original_null_and_throwing_clock_operations_preserve_failure_identity_state_and_recovery() {
    for row in rows().into_iter().filter(|v| v["tag"] == "operation") {
        let original = text(&row["text"]);
        let target = Rc::new(TextBufferTarget::new(original.clone(), None));
        let position = match row["position"].as_i64().unwrap() {
            0 => 0,
            1 => original.len() / 2,
            2 => original.len(),
            _ => panic!("position"),
        };
        target.select(position as i32, if row["selected"] == true { 1 } else { 0 });
        let e = EditorController::new(target);
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let failure = Arc::new(ProbeException);
        let marker = failure.clone();
        let throwing = row["clock"] == "throw";
        let f: EditorClock = Rc::new(move || {
            count.set(count.get() + 1);
            if throwing {
                std::panic::panic_any(marker.clone());
            }
            fixture_clock::time(100_000_000)
        });
        let assigned = if row["clock"] == "null" {
            None
        } else {
            Some(f.clone())
        };
        e.set_clock_nullable(assigned.clone());
        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            apply(&e, row["op"].as_str().unwrap())
        })) {
            Ok(Ok(value)) => json!({"value":value}),
            Ok(Err(EditorError::NullReference)) => {
                json!({"error":"NullReferenceException","same":false})
            }
            Ok(Err(e)) => panic!("unexpected {e}"),
            Err(error) => {
                if let Some(error) = error.downcast_ref::<Arc<ProbeException>>() {
                    json!({"error":"ProbeException","same":Arc::ptr_eq(error,&failure)})
                } else {
                    std::panic::resume_unwind(error)
                }
            }
        };
        assert_eq!(calls.get(), row["calls"].as_i64().unwrap());
        let same = match (e.clock(), assigned) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(&a, &b),
            _ => false,
        };
        assert_eq!(same, row["clockSame"].as_bool().unwrap());
        assert_eq!(result, row["result"], "{row}");
        assert_eq!(state(&e), row["after"], "{row}");
        e.set_clock(Rc::new(|| fixture_clock::time(100_000_001)));
        e.commit_text(&"r".into()).unwrap();
        assert_eq!(state(&e), row["recovered"], "{row}");
        assert_eq!(e.undo().unwrap(), row["undo"].as_bool().unwrap());
        assert_eq!(state(&e), row["restored"], "{row}");
    }
}
#[test]
fn original_reentrant_clock_replacement_nulling_history_and_timeout_are_synchronous() {
    for row in rows().into_iter().filter(|v| v["tag"] == "reentrant") {
        let e = Rc::new(EditorController::new(Rc::new(TextBufferTarget::new(
            "a".into(),
            None,
        ))));
        let first = Rc::new(Cell::new(0));
        let second = Rc::new(Cell::new(0));
        let second_count = second.clone();
        let replacement: EditorClock = Rc::new(move || {
            second_count.set(second_count.get() + 1);
            fixture_clock::time(100_000_001)
        });
        let replace = replacement.clone();
        let weak = Rc::downgrade(&e);
        let action = row["action"].as_str().unwrap().to_string();
        let count = first.clone();
        let original: EditorClock = Rc::new(move || {
            count.set(count.get() + 1);
            let e = weak.upgrade().unwrap();
            match action.as_str() {
                "null" => e.set_clock_nullable(None),
                "replace" => e.set_clock(replace.clone()),
                "clear" => e.clear_history(),
                "timeout" => e.set_undo_group_timeout(std::time::Duration::ZERO),
                _ => panic!("action"),
            };
            fixture_clock::time(100_000_000)
        });
        e.set_clock(original.clone());
        e.commit_text(&"x".into()).unwrap();
        let first_state = state(&e);
        let result = match e.commit_text(&"y".into()) {
            Ok(()) => json!({"ok":true}),
            Err(EditorError::NullReference) => json!({"error":"NullReferenceException"}),
            Err(e) => panic!("unexpected {e}"),
        };
        let clock = e.clock();
        let actual = state(&e);
        let undo = e.undo().unwrap();
        assert_eq!(
            json!({"tag":"reentrant","action":row["action"],"first":first.get(),"second":second.get(),"clockNull":clock.is_none(),"clockOriginal":clock.as_ref().is_some_and(|v|Rc::ptr_eq(v,&original)),"clockReplacement":clock.as_ref().is_some_and(|v|Rc::ptr_eq(v,&replacement)),"firstState":first_state,"result":result,"state":actual,"undo":undo,"restored":state(&e)}),
            row
        );
    }
}
