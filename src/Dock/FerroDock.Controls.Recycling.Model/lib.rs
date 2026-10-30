//! Framework-independent contracts for cached and recycled control content.

mod i_control_recycling;
mod i_control_recycling_id_provider;

pub use i_control_recycling::IControlRecycling;
pub use i_control_recycling_id_provider::IControlRecyclingIdProvider;

#[cfg(test)]
mod control_recycling_tests;
#[cfg(test)]
mod dummy_control_recycling;
