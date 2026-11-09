//! Decode saved clipboard wire values for unchanged C# reference tests only.
use ferroboard_core::clipboard::{
    OffsetDateTime, TimestampError, UtcOffset, format_clipboard_timestamp,
};
const EPOCH: i128 = 621_355_968_000_000_000;
pub fn timestamp(local_ticks: i64, minutes: i16) -> Result<OffsetDateTime, TimestampError> {
    if !(0..=3_155_378_975_999_999_999).contains(&local_ticks) || !(-840..=840).contains(&minutes) {
        return Err(TimestampError::OutOfRange);
    }
    let unix_nanos = (i128::from(local_ticks) - EPOCH - i128::from(minutes) * 600_000_000) * 100;
    let value = OffsetDateTime::from_unix_timestamp_nanos(unix_nanos)
        .map_err(|_| TimestampError::OutOfRange)?
        .checked_to_offset(
            UtcOffset::from_whole_seconds(i32::from(minutes) * 60)
                .map_err(|_| TimestampError::OutOfRange)?,
        )
        .ok_or(TimestampError::OutOfRange)?;
    format_clipboard_timestamp(value)?;
    Ok(value)
}
#[allow(dead_code)]
pub fn local_ticks(value: OffsetDateTime) -> i64 {
    i64::try_from(
        value.unix_timestamp_nanos() / 100
            + EPOCH
            + i128::from(value.offset().whole_seconds()) * 10_000_000,
    )
    .unwrap()
}
