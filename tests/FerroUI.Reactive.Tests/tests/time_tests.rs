//! Ported from `Tests.System.Reactive/Tests/TimeTests.cs` of Rx.NET.
//! Adapted: `TimeSpan` is `Duration` (123.45 seconds is `Duration::from_millis(123_450)`) and
//! `DateTimeOffset` is `SystemTime` (`new DateTimeOffset()` is the epoch, `UtcNow` is
//! `SystemTime::now()`, `AddDays(1)` adds 86 400 seconds); `GetHashCode` is the `Hash` of the
//! standard hasher; a `null` string is `None` of `Option<String>`; `ToString` is `Display`, which
//! formats the interval with its `Debug` form; comparisons with values of other types
//! (`Equals("x")`) have no form in Rust and are left out.

use ferroui_reactive::{TimeInterval, Timestamped};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, SystemTime};

fn hash_code<T: Hash>(value: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn seconds_123_45() -> Duration {
    Duration::from_millis(123_450)
}

fn seconds_123_56() -> Duration {
    Duration::from_millis(123_560)
}

fn one_day() -> Duration {
    Duration::from_secs(24 * 60 * 60)
}

#[test]
fn time_interval_ctor_properties() {
    let ti = TimeInterval::new(42, seconds_123_45());
    assert_eq!(ti.value, 42);
    assert_eq!(ti.interval, seconds_123_45());
}

#[test]
fn time_interval_equals() {
    let ti = TimeInterval::new(42, seconds_123_45());
    assert!(ti.eq(&ti));

    let t2 = TimeInterval::new(43, seconds_123_45());
    assert!(!ti.eq(&t2));

    let t3 = TimeInterval::new(42, seconds_123_56());
    assert!(!ti.eq(&t3));

    let t4 = TimeInterval::new(42, seconds_123_45());
    assert!(ti.eq(&t4));
}

#[test]
fn time_interval_get_hash_code() {
    let ti = TimeInterval::new(None::<String>, seconds_123_45());
    assert!(hash_code(&ti) != 0);
    assert_eq!(hash_code(&ti), hash_code(&ti));

    let t2 = TimeInterval::new(Some(String::new()), seconds_123_45());
    assert_ne!(hash_code(&t2), hash_code(&ti));
}

#[test]
#[allow(clippy::nonminimal_bool)]
fn time_interval_equals_operators() {
    let ti = TimeInterval::new(42, seconds_123_45());
    let t2 = TimeInterval::new(43, seconds_123_45());
    assert!(!(ti == t2));
    assert!(!(t2 == ti));
    assert!(ti != t2);
    assert!(t2 != ti);

    let t3 = TimeInterval::new(42, seconds_123_56());
    assert!(!(ti == t3));
    assert!(!(t3 == ti));
    assert!(ti != t3);
    assert!(t3 != ti);

    let t4 = TimeInterval::new(42, seconds_123_45());
    assert!(ti == t4);
    assert!(t4 == ti);
    assert!(!(ti != t4));
    assert!(!(t4 != ti));
}

#[test]
fn time_interval_to_string() {
    let ti = TimeInterval::new(42, seconds_123_45());
    assert!(ti.to_string().contains(&42.to_string()));
    assert!(ti.to_string().contains(&format!("{:?}", seconds_123_45())));
}

#[test]
fn timestamped_create() {
    let o = SystemTime::now();
    let ti = Timestamped::new(42, o);
    assert_eq!(ti.value, 42);
    assert_eq!(ti.timestamp, o);
}

#[test]
fn timestamped_ctor_properties() {
    let o = SystemTime::UNIX_EPOCH;
    let ti = Timestamped::new(42, o);
    assert_eq!(ti.value, 42);
    assert_eq!(ti.timestamp, o);
}

#[test]
fn timestamped_equals() {
    let ti = Timestamped::new(42, SystemTime::UNIX_EPOCH);
    assert!(ti.eq(&ti));

    let t2 = Timestamped::new(43, SystemTime::UNIX_EPOCH);
    assert!(!ti.eq(&t2));

    let t3 = Timestamped::new(42, SystemTime::UNIX_EPOCH + one_day());
    assert!(!ti.eq(&t3));

    let t4 = Timestamped::new(42, SystemTime::UNIX_EPOCH);
    assert!(ti.eq(&t4));
}

#[test]
fn timestamped_get_hash_code() {
    let ti = Timestamped::new(None::<String>, SystemTime::UNIX_EPOCH);
    assert!(hash_code(&ti) != 0);
    assert_eq!(hash_code(&ti), hash_code(&ti));

    let t2 = Timestamped::new(Some(String::new()), SystemTime::UNIX_EPOCH);
    assert_ne!(hash_code(&t2), hash_code(&ti));
}

#[test]
#[allow(clippy::nonminimal_bool)]
fn timestamped_equals_operators() {
    let o = SystemTime::UNIX_EPOCH;

    let ti = Timestamped::new(42, o);
    let t2 = Timestamped::new(43, o);
    assert!(!(ti == t2));
    assert!(!(t2 == ti));
    assert!(ti != t2);
    assert!(t2 != ti);

    let t3 = Timestamped::new(42, o + one_day());
    assert!(!(ti == t3));
    assert!(!(t3 == ti));
    assert!(ti != t3);
    assert!(t3 != ti);

    let t4 = Timestamped::new(42, o);
    assert!(ti == t4);
    assert!(t4 == ti);
    assert!(!(ti != t4));
    assert!(!(t4 != ti));
}

// Not ported: Timestamped_ToString - Timestamped<T> does not implement Display (ToString, `value@timestamp` upstream)
