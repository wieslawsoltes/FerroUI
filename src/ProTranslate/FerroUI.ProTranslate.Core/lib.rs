//! Framework-independent, application-scoped localization and globalization.
//!
//! Services are cheap `Rc` handles, intentionally bound to the owning thread.
//! Callbacks run synchronously after state changes, without internal borrows.
//! Dropping a subscription disconnects it, and observable strings hold services
//! without services retaining their observers.

mod calendar;
mod culture;
mod date_data;
mod datetime;
mod error;
mod event;
mod formatting;
mod globalization;
mod locale_data;
mod measurement;
mod provider;
mod region;
mod service;

pub use culture::*;
pub use datetime::{LocalizedDateTime, LocalizedNaiveDateTime};
pub use error::*;
pub use event::Subscription;
pub use formatting::*;
pub use globalization::*;
pub use measurement::*;
pub use provider::*;
pub use region::{DefaultRegionProfileProvider, RegionProfile, RegionProfileProvider};
pub use service::*;

#[cfg(test)]
mod tests;
