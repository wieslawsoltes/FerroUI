//! Independent native calendar/error consumer; no private fixture adapters.
use ferroboard_core::clipboard::{
    ClipboardClock, ClipboardError, ClipboardHistory, OffsetDateTime, TimeZone, TimestampError,
    UtcOffset,
};
use std::rc::Rc;
#[test]
fn native_clipboard_calendar_clock_and_fallible_persistence_work_downstream()
-> Result<(), Box<dyn std::error::Error>> {
    let history = ClipboardHistory::default();
    let point = OffsetDateTime::from_unix_timestamp(1_700_000_000)?
        .to_offset(UtcOffset::from_hms(-3, -30, 0)?);
    let clock: ClipboardClock = Rc::new(move || point);
    history.set_clock(clock.clone());
    assert!(Rc::ptr_eq(&clock, &history.clock()));
    let item = history.add(Some(&"consumer".into()))?.unwrap();
    history.set_pinned(&item, true);
    let data = history.export(true)?;
    let copy = ClipboardHistory::default();
    copy.import(&data)?;
    let restored = copy.primary().unwrap();
    assert_eq!(restored.timestamp(), point);
    assert_eq!(restored.timestamp().offset(), point.offset());
    item.set_timestamp(point.replace_nanosecond(1)?);
    let error = history.export(true).unwrap_err();
    assert!(matches!(
        error,
        ClipboardError::Timestamp(TimestampError::UnsupportedPrecision)
    ));
    assert!(
        std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<TimestampError>()
            .is_some()
    );
    Ok(())
}

#[test]
fn caller_owned_native_zone_and_explicit_offset_precedence_work_downstream()
-> Result<(), Box<dyn std::error::Error>> {
    let zone = TimeZone::posix("EST5EDT,M3.2.0,M11.1.0")?;
    let history = ClipboardHistory::default();
    history.set_expire_after(None);
    history.import_with_time_zone(
        r#"[{"text":"local","timestamp":"2024-11-03T01:30:00"},{"text":"explicit","timestamp":"2024-11-03T01:30:00+05:45"}]"#,
        &zone,
    )?;
    drop(zone);
    let items = history.items()?;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].timestamp().offset().whole_seconds(), -18_000);
    assert_eq!(items[1].timestamp().offset().whole_seconds(), 20_700);
    assert_eq!(
        history.export(false)?,
        r#"[{"text":"local","timestamp":"2024-11-03T01:30:00-05:00","pinned":false},{"text":"explicit","timestamp":"2024-11-03T01:30:00+05:45","pinned":false}]"#
    );
    Ok(())
}
