//! Decode nonnegative 100ns quantities from unchanged C# fixtures into std Duration.
//! Negative CLR spans are deliberately not part of the native API.
pub fn duration(ticks: i64) -> std::time::Duration {
    let ticks = u64::try_from(ticks).expect("negative fixture span requires explicit retirement");
    std::time::Duration::new(ticks / 10_000_000, ((ticks % 10_000_000) * 100) as u32)
}
