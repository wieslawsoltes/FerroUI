//! Native iOS launcher services; the keyboard and presentation remain in shared crates.
#[cfg(target_vendor = "apple")]
pub mod lifecycle;
#[cfg(target_vendor = "apple")]
pub mod storage;
