//! Decode unchanged C# timing traces into a native test timeline.
//! The Unix origin is test-only: all compared quantities are elapsed intervals.
use std::time::{Duration, SystemTime, UNIX_EPOCH};
pub fn time(ticks: i64) -> SystemTime {
    let ticks = u64::try_from(ticks).expect("negative source fixture timestamp");
    UNIX_EPOCH + Duration::new(ticks / 10_000_000, ((ticks % 10_000_000) * 100) as u32)
}
#[allow(dead_code)]
pub fn ticks(time: SystemTime) -> i64 {
    i64::try_from(
        time.duration_since(UNIX_EPOCH)
            .expect("fixture clock predates origin")
            .as_nanos()
            / 100,
    )
    .unwrap()
}
