#[path = "../../../tests/support/fixture_clipboard_time.rs"]
mod fixture_clipboard_time;
#[path = "../../../tests/support/local_future.rs"]
mod native_future;
use ferroboard_core::{clipboard::*, text::Utf16Text};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn rows() -> Vec<Value> {
    include_str!("../../../tests/fixtures/clipboard.jsonl")
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn item(i: &ClipboardItem) -> Value {
    json!({"text":i.text.units(),"ticks":fixture_clipboard_time::local_ticks(i.timestamp()),"offset":i.timestamp().offset().whole_minutes(),"pinned":i.is_pinned()})
}
fn error(e: ClipboardError) -> &'static str {
    match e {
        ClipboardError::LimitIndex => "ArgumentOutOfRangeException",
        ClipboardError::MissingProperty => "KeyNotFoundException",
        ClipboardError::Json(ferroboard_core::serialization::JsonBridgeError::Syntax {
            ..
        }) => "JsonReaderException",
        ClipboardError::Json(_) => "InvalidOperationException",
        ClipboardError::Timestamp(_) => "FormatException",
    }
}
fn result(r: Result<Value, ClipboardError>) -> Value {
    match r {
        Ok(v) => json!({"value":v}),
        Err(e) => json!({"error":error(e)}),
    }
}
#[test]
fn unchanged_source_history_operations_exports_partial_imports_and_notifications() {
    let h = ClipboardHistory::default();
    let now = Rc::new(Cell::new(
        fixture_clipboard_time::timestamp(638448066451234567, 0).unwrap(),
    ));
    let calls = Rc::new(Cell::new(0));
    let events = Rc::new(Cell::new(0));
    let clock = now.clone();
    let n = calls.clone();
    h.set_clock(Rc::new(move || {
        n.set(n.get() + 1);
        clock.get()
    }));
    let n = events.clone();
    h.changed().add(Rc::new(move |_| n.set(n.get() + 1)));
    let mut held = Vec::new();
    for row in rows() {
        let actual = match row["kind"].as_str().unwrap() {
            "operation" => {
                let op = row["op"].as_array().unwrap();
                match op[0].as_str().unwrap() {
                    "add" => h.add(Some(&op[1].as_str().unwrap().into())).map(|i| {
                        i.map_or(Value::Null, |i| {
                            let v = item(&i);
                            held.push(i);
                            v
                        })
                    }),
                    "pin" => {
                        h.set_pinned(
                            &held[op[1].as_u64().unwrap() as usize],
                            op[2].as_bool().unwrap(),
                        );
                        Ok(json!(true))
                    }
                    "max" => {
                        h.set_max_items(op[1].as_i64().unwrap() as i32);
                        Ok(json!(true))
                    }
                    "advance" => {
                        now.set(
                            fixture_clipboard_time::timestamp(
                                fixture_clipboard_time::local_ticks(now.get())
                                    + op[1].as_i64().unwrap(),
                                0,
                            )
                            .unwrap(),
                        );
                        Ok(json!(true))
                    }
                    "items" => h
                        .items()
                        .map(|v| json!(v.iter().map(|i| item(i)).collect::<Vec<_>>())),
                    "primary" => Ok(h.primary().map_or(Value::Null, |i| item(&i))),
                    "clear" => {
                        h.clear(op[1].as_bool().unwrap());
                        Ok(json!(true))
                    }
                    "remove" => {
                        h.remove(&held[op[1].as_u64().unwrap() as usize]);
                        Ok(json!(true))
                    }
                    _ => unreachable!(),
                }
            }
            "import" => {
                h.set_max_items(30);
                h.import(row["json"].as_str().unwrap()).map(|_| json!(true))
            }
            _ => continue,
        };
        assert_eq!(result(actual), row["result"], "{row}");
        let state = json!({"primary":h.primary().map(|i|item(&i)),"export":h.export(false).unwrap(),"pinned":h.export(true).unwrap(),"events":events.get(),"calls":calls.get()});
        assert_eq!(state, row["state"], "{row}");
    }
}
#[test]
fn source_timestamp_formatter_range_and_roundtrip() {
    for row in rows().iter().filter(|r| r["kind"] == "timestamp") {
        let t = fixture_clipboard_time::timestamp(
            row["ticks"].as_i64().unwrap(),
            row["minutes"].as_i64().unwrap() as i16,
        );
        match t {
            Ok(t) => {
                let h = ClipboardHistory::default();
                h.set_clock(Rc::new(move || t));
                h.add(Some(&"time".into())).unwrap();
                assert_eq!(
                    json!({"value":h.export(false).unwrap()}),
                    row["result"],
                    "{row}"
                );
                let parsed =
                    parse_clipboard_timestamp(&format_clipboard_timestamp(t).unwrap()).unwrap();
                assert_eq!(
                    fixture_clipboard_time::local_ticks(parsed),
                    fixture_clipboard_time::local_ticks(t)
                );
                assert_eq!(parsed.offset().whole_minutes(), t.offset().whole_minutes());
            }
            Err(_) => assert!(row["result"].get("error").is_some()),
        }
    }
}
#[test]
fn service_is_immediately_completed_and_history_preserves_identity() {
    let service = InMemoryClipboardService::default();
    let api: &dyn IClipboardService = &service;
    let before = api.get_text_async();
    let set = api.set_text_async(Some("😀".into()));
    native_future::ready(set).unwrap();
    assert!(native_future::ready(before).unwrap().is_none());
    assert_eq!(
        native_future::ready(api.get_text_async()).unwrap(),
        Some(Utf16Text::from("😀"))
    );
    native_future::ready(api.set_text_async(None)).unwrap();
    assert!(
        native_future::ready(api.get_text_async())
            .unwrap()
            .is_none()
    );
    let h = ClipboardHistory::default();
    let a = h.add(Some(&"a".into())).unwrap().unwrap();
    let b = h.add(Some(&"a".into())).unwrap().unwrap();
    assert!(Rc::ptr_eq(&a, &b));
    assert_eq!(a.id.get_version_num(), 4);
    assert!(!a.id.is_nil());
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    h.changed().add(Rc::new(move |h| {
        log.borrow_mut().push(h.export(false).unwrap())
    }));
    h.set_pinned(&a, true);
    h.clear(false);
    assert_eq!(h.items().unwrap().len(), 1);
    let copy = ClipboardHistory::default();
    copy.import(&h.export(false).unwrap()).unwrap();
    assert!(copy.items().unwrap()[0].is_pinned());
    assert_ne!(copy.primary().unwrap().id, a.id);
    assert_eq!(events.borrow().len(), 2);
}

#[test]
fn source_reentrant_clock_callbacks_preserve_partial_state_and_event_order() {
    for row in rows().into_iter().filter(|r| r["kind"] == "reentrant") {
        let h = Rc::new(ClipboardHistory::default());
        h.set_expire_after(None);
        let t = parse_clipboard_timestamp("2024-01-01T00:00:00Z").unwrap();
        h.set_clock(Rc::new(move || t));
        let a = h.add(Some(&"a".into())).unwrap().unwrap();
        let log = Rc::new(RefCell::new(Vec::new()));
        let events = log.clone();
        h.changed().add(Rc::new(move |h| {
            events
                .borrow_mut()
                .push(format!("event:{}", h.export(false).unwrap()))
        }));
        let once = Cell::new(false);
        let weak = Rc::downgrade(&h);
        let events = log.clone();
        let mode = row["mode"].as_str().unwrap().to_owned();
        h.set_clock(Rc::new(move || {
            let h = weak.upgrade().unwrap();
            events
                .borrow_mut()
                .push(format!("clock:{}", h.export(false).unwrap()));
            if !once.replace(true) {
                match mode.as_str() {
                    "clear" => h.clear(true),
                    "nested" => {
                        h.add(Some(&"nested".into())).unwrap();
                    }
                    "throw" => panic!("custom clock"),
                    _ => unreachable!(),
                }
            }
            fixture_clipboard_time::timestamp(fixture_clipboard_time::local_ticks(t) + 1, 0)
                .unwrap()
        }));
        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            h.add(Some(&"a".into()))
        })) {
            Ok(r) => self::result(r.map(|i| json!(i.unwrap().text.to_utf8().unwrap()))),
            Err(_) => json!({"error":"ApplicationException"}),
        };
        assert_eq!(result, row["result"]);
        assert_eq!(json!(*log.borrow()), row["log"]);
        assert_eq!(h.export(false).unwrap(), row["export"].as_str().unwrap());
        assert_eq!(
            json!(fixture_clipboard_time::local_ticks(a.timestamp())),
            row["retained"]
        );
    }
}

#[test]
fn timestamp_rejects_unavailable_native_zone_and_range_without_fallback() {
    assert_eq!(
        parse_clipboard_timestamp_in_zone("2024-01-01", &jiff::tz::TimeZone::unknown())
            .unwrap_err(),
        TimestampError::UnsupportedLocalTime
    );
    assert_eq!(
        parse_clipboard_timestamp_in_zone("2024-01-01T12:30:00", &jiff::tz::TimeZone::unknown())
            .unwrap_err(),
        TimestampError::UnsupportedLocalTime
    );
    for t in [i64::MIN, i64::MAX] {
        assert_eq!(
            fixture_clipboard_time::timestamp(t, 840).unwrap_err(),
            TimestampError::OutOfRange
        );
    }
}

#[test]
fn upstream_clipboard_history_limits_pins_and_expires() {
    let now = Rc::new(Cell::new(
        OffsetDateTime::now_utc().replace_nanosecond(0).unwrap(),
    ));
    let history = ClipboardHistory::default();
    history.set_max_items(2);
    history.set_expire_after(Some(std::time::Duration::from_secs(600)));
    let clock = now.clone();
    history.set_clock(Rc::new(move || clock.get()));
    let a = history.add(Some(&"a".into())).unwrap().unwrap();
    history.set_pinned(&a, true);
    for text in ["b", "c", "d"] {
        history.add(Some(&text.into())).unwrap();
    }
    assert_eq!(
        history
            .items()
            .unwrap()
            .iter()
            .map(|i| i.text.to_utf8().unwrap())
            .collect::<Vec<_>>(),
        ["a", "d", "c"]
    );
    now.set(
        fixture_clipboard_time::timestamp(
            fixture_clipboard_time::local_ticks(now.get()) + 6_600_000_000,
            0,
        )
        .unwrap(),
    );
    assert_eq!(
        history
            .items()
            .unwrap()
            .iter()
            .map(|i| i.text.to_utf8().unwrap())
            .collect::<Vec<_>>(),
        ["a"]
    );
    let copy = ClipboardHistory::default();
    copy.import(&history.export(false).unwrap()).unwrap();
    let items = copy.items().unwrap();
    assert_eq!(items.len(), 1);
    assert!(items[0].is_pinned());
}

#[test]
fn native_history_errors_propagate_with_question_mark_and_typed_causes() {
    use ferroboard_core::serialization::JsonBridgeError;
    fn import(json: &str) -> Result<(), Box<dyn std::error::Error>> {
        ClipboardHistory::default().import(json)?;
        Ok(())
    }
    let json = import("[").unwrap_err();
    assert!(json.downcast_ref::<ClipboardError>().is_some());
    assert!(matches!(
        json.source().unwrap().downcast_ref::<JsonBridgeError>(),
        Some(JsonBridgeError::Syntax { .. })
    ));
    assert!(!json.to_string().is_empty());
    let timestamp = import(r#"[{"text":"a","timestamp":"invalid"}]"#).unwrap_err();
    assert_eq!(
        timestamp.source().unwrap().downcast_ref::<TimestampError>(),
        Some(&TimestampError::InvalidFormat)
    );
    let missing = import("[{}]").unwrap_err();
    assert!(matches!(
        missing.downcast_ref::<ClipboardError>(),
        Some(ClipboardError::MissingProperty)
    ));
    assert!(missing.source().is_none());
}
