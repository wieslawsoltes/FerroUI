//! Native calendar API and explicit clipboard persistence constraints.
use ferroboard_core::clipboard::{
    ClipboardClock, ClipboardError, ClipboardHistory, OffsetDateTime, TimestampError, UtcOffset,
    format_clipboard_timestamp, parse_clipboard_timestamp,
};
use std::{
    cell::Cell,
    rc::{Rc, Weak},
    time::Duration,
};

#[test]
fn native_values_preserve_offsets_instants_wire_precision_and_expiry() {
    let point = OffsetDateTime::from_unix_timestamp_nanos(1_700_000_000_123_456_700).unwrap();
    let offset = UtcOffset::from_hms(5, 45, 0).unwrap();
    let local = point.to_offset(offset);
    assert_eq!(point, local);
    let clock: ClipboardClock = Rc::new(move || local);
    let history = ClipboardHistory::default();
    history.set_clock(clock.clone());
    assert!(Rc::ptr_eq(&clock, &history.clock()));
    history.set_expire_after(Some(Duration::ZERO));
    let item = history.add(Some(&"native".into())).unwrap().unwrap();
    assert_eq!(item.timestamp(), point);
    assert_eq!(item.timestamp().offset(), offset);
    let json = history.export(false).unwrap();
    let restored = ClipboardHistory::default();
    restored.import(&json).unwrap();
    let actual = restored.primary().unwrap().timestamp();
    assert_eq!(actual, point);
    assert_eq!(actual.offset(), offset);
    assert_eq!(restored.export(false).unwrap(), json);
    let next = point + Duration::from_nanos(100);
    history.set_clock(Rc::new(move || next));
    assert!(history.items().unwrap().is_empty());
    let source = "2024-02-29T23:59:58.1234567890123456+05:45";
    assert_eq!(
        format_clipboard_timestamp(parse_clipboard_timestamp(source).unwrap()).unwrap(),
        "2024-02-29T23:59:58.1234567+05:45"
    );
}

#[test]
fn unrepresentable_native_values_return_typed_errors_without_partial_success() {
    let history = ClipboardHistory::default();
    history.set_clock(Rc::new(|| OffsetDateTime::UNIX_EPOCH));
    let item = history.add(Some(&"data".into())).unwrap().unwrap();
    for (value, expected) in [
        (
            OffsetDateTime::UNIX_EPOCH.replace_nanosecond(1).unwrap(),
            TimestampError::UnsupportedPrecision,
        ),
        (
            OffsetDateTime::UNIX_EPOCH.to_offset(UtcOffset::from_whole_seconds(1).unwrap()),
            TimestampError::OutOfRange,
        ),
        (
            OffsetDateTime::UNIX_EPOCH.to_offset(UtcOffset::from_hms(15, 0, 0).unwrap()),
            TimestampError::OutOfRange,
        ),
        (
            OffsetDateTime::UNIX_EPOCH.replace_year(0).unwrap(),
            TimestampError::OutOfRange,
        ),
    ] {
        item.set_timestamp(value);
        assert!(
            matches!(history.export(false), Err(ClipboardError::Timestamp(actual)) if actual == expected)
        );
        assert_eq!(item.timestamp(), value);
        // Exporting pins does not inspect an excluded ordinary card.
        assert_eq!(history.export(true).unwrap(), "[]");
    }
    let default = ClipboardHistory::default();
    let now = default.clock()();
    assert_eq!(now.offset(), UtcOffset::UTC);
    assert!(now.nanosecond().is_multiple_of(100));
    assert!(format_clipboard_timestamp(now).is_ok());
}

#[test]
fn replacing_clock_releases_captures_after_publishing_and_outside_borrows() {
    struct Capture {
        history: Weak<ClipboardHistory>,
        observed: Rc<Cell<bool>>,
    }
    impl Drop for Capture {
        fn drop(&mut self) {
            let history = self.history.upgrade().unwrap();
            assert_eq!(
                history.clock()(),
                OffsetDateTime::UNIX_EPOCH + Duration::from_secs(1)
            );
            history.set_clock(Rc::new(|| {
                OffsetDateTime::UNIX_EPOCH + Duration::from_secs(2)
            }));
            self.observed.set(true);
        }
    }
    let history = Rc::new(ClipboardHistory::default());
    let observed = Rc::new(Cell::new(false));
    let capture = Capture {
        history: Rc::downgrade(&history),
        observed: observed.clone(),
    };
    history.set_clock(Rc::new(move || {
        let _ = &capture;
        OffsetDateTime::UNIX_EPOCH
    }));
    history.set_clock(Rc::new(|| {
        OffsetDateTime::UNIX_EPOCH + Duration::from_secs(1)
    }));
    assert!(observed.get());
    assert_eq!(
        history.clock()(),
        OffsetDateTime::UNIX_EPOCH + Duration::from_secs(2)
    );
}
