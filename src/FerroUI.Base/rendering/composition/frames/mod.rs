//! Shared producer frames. Native storage is immutable while leased; GPU reads
//! retain leases through actual completion, independently of UI/job lifetimes.
mod lifetime;
mod metadata;
mod policy;
pub use lifetime::*;
pub use metadata::*;
pub use policy::*;
#[cfg(test)]
mod tests;
