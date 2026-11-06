use ferroboard_core::clipboard::{
    ClipboardError, ClipboardHistory, TimestampError, parse_clipboard_timestamp,
    parse_clipboard_timestamp_in_zone,
};
use jiff::tz::TimeZone;
use serde_json::{Value, json};
use std::{cell::Cell, rc::Rc, time::Duration};

fn zone(name: &str) -> TimeZone {
    let data: &[u8] = match name {
        "UTC" => include_bytes!("../../../tests/fixtures/clipboard-time-zones/UTC.tzif"),
        "America/New_York" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/America-New_York.tzif")
        }
        "Europe/Lisbon" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/Europe-Lisbon.tzif")
        }
        "Australia/Sydney" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/Australia-Sydney.tzif")
        }
        "Asia/Kathmandu" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/Asia-Kathmandu.tzif")
        }
        "Europe/Dublin" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/Europe-Dublin.tzif")
        }
        "Africa/Casablanca" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/Africa-Casablanca.tzif")
        }
        "Pacific/Apia" => {
            include_bytes!("../../../tests/fixtures/clipboard-time-zones/Pacific-Apia.tzif")
        }
        _ => panic!("unknown frozen time zone"),
    };
    TimeZone::tzif(name, data).unwrap()
}

// The native wire boundary rejects sub-minute local offsets rather than emulating
// TimeZoneInfo's historical rule quantization/TZif interpretation. Keep each known
// source/native difference explicit; do not silently skip new mismatches.
fn unrepresentable_historical_local(zone: &str, text: &str) -> bool {
    matches!(
        (zone, text),
        ("America/New_York", "0001-01-01")
            | (
                "Europe/Lisbon" | "Europe/Dublin" | "Africa/Casablanca",
                "0001-01-01" | "1900-01-01T12:00:00"
            )
            | ("Asia/Kathmandu" | "Pacific/Apia", "1900-01-01T12:00:00")
    )
}

#[test]
fn unchanged_history_imports_match_native_zones_with_explicit_historical_limits() {
    let mut count = 0;
    let mut historical = 0;
    for line in include_str!("../../../tests/fixtures/clipboard-local-time.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let name = row["zone"].as_str().unwrap();
        let text = row["text"].as_str().unwrap();
        let history = ClipboardHistory::default();
        history.set_expire_after(None);
        history.add(Some(&"before".into())).unwrap();
        let clock_calls = Rc::new(Cell::new(0));
        let called = clock_calls.clone();
        history.set_clock(Rc::new(move || {
            called.set(called.get() + 1);
            parse_clipboard_timestamp("2024-01-01T00:00:00Z").unwrap()
        }));
        let events = Rc::new(Cell::new(0));
        let seen = events.clone();
        let token = history
            .changed()
            .add(Rc::new(move |_| seen.set(seen.get() + 1)));
        let input = json!([
            {"text":"prefix","timestamp":"2024-01-01T00:00:00Z"},
            {"text":"local","timestamp":text,"pinned":true}
        ])
        .to_string();
        let result = history.import_with_time_zone(&input, &zone(name));
        assert_eq!(
            clock_calls.get(),
            0,
            "timestamp conversion must not fall back to Clock"
        );
        if unrepresentable_historical_local(name, text) {
            assert_eq!(row["success"], true);
            assert!(
                matches!(
                    result,
                    Err(ClipboardError::Timestamp(TimestampError::OutOfRange))
                ),
                "{row}: {result:?}"
            );
            assert_eq!(events.get(), 0);
            let items = history.items().unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].text.to_utf8().unwrap(), "prefix");
            historical += 1;
        } else {
            assert_eq!(
                result.is_ok(),
                row["success"].as_bool().unwrap(),
                "{row}: {result:?}"
            );
            assert_eq!(
                history.export(false).unwrap(),
                row["export"].as_str().unwrap(),
                "{row}"
            );
            assert_eq!(events.get(), row["events"].as_u64().unwrap(), "{row}");
            let items = history.items().unwrap();
            let actual: Vec<_> = items.iter().map(|item| json!({
                "text":item.text.to_utf8().unwrap(),"offsetSeconds":item.timestamp().offset().whole_seconds(),
                "unixTicks":i64::try_from(item.timestamp().unix_timestamp_nanos()/100).unwrap(),"pinned":item.is_pinned()
            })).collect();
            let expected: Vec<_> = row["items"].as_array().unwrap().iter().map(|item| json!({
                "text":item["text"],"offsetSeconds":item["offsetSeconds"],"unixTicks":item["unixTicks"],"pinned":item["pinned"]
            })).collect();
            assert_eq!(actual, expected, "{row}");
        }
        assert!(history.changed().remove(token));
        count += 1;
    }
    assert_eq!((count, historical), (192, 9));
}

#[test]
fn explicit_offsets_do_not_require_local_zone_discovery() {
    let unknown = TimeZone::unknown();
    let text = "2024-02-29T12:30:45.1234567890123456+05:45";
    let a = parse_clipboard_timestamp(text).unwrap();
    let b = parse_clipboard_timestamp_in_zone(text, &unknown).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.offset(), b.offset());
    assert_eq!(a.nanosecond(), 123_456_700);
    assert_eq!(
        parse_clipboard_timestamp_in_zone("2024-02-29", &unknown),
        Err(TimestampError::UnsupportedLocalTime)
    );
}

#[test]
fn default_local_parser_uses_the_native_system_zone_or_reports_unavailability() {
    let text = "2024-02-29T12:30:45";
    match TimeZone::try_system() {
        Ok(zone) => {
            let actual = parse_clipboard_timestamp(text).unwrap();
            let expected = parse_clipboard_timestamp_in_zone(text, &zone).unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual.offset(), expected.offset());
        }
        Err(_) => assert_eq!(
            parse_clipboard_timestamp(text),
            Err(TimestampError::UnsupportedLocalTime)
        ),
    }
}

#[test]
fn local_fold_import_preserves_the_instant_for_strict_expiry() {
    let history = ClipboardHistory::default();
    history.set_expire_after(Some(Duration::from_secs(3600)));
    let end = parse_clipboard_timestamp("2024-11-03T07:30:00Z").unwrap();
    history.set_clock(Rc::new(move || end));
    history
        .import_with_time_zone(
            r#"[{"text":"ordinary","timestamp":"2024-11-03T01:30:00"}]"#,
            &zone("America/New_York"),
        )
        .unwrap();
    let items = history.items().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].timestamp(),
        parse_clipboard_timestamp("2024-11-03T06:30:00Z").unwrap()
    );
    history.set_clock(Rc::new(move || end + Duration::from_nanos(100)));
    assert!(history.items().unwrap().is_empty());
}
