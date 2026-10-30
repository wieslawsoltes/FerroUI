//! Shared native value conversion for the original UI-independent object contracts.
//! Keeps a single type registry and representation across recycling and model metadata.
mod value_bridge;
pub use value_bridge::{from_source_value, register_value_type, to_source_value};

#[cfg(test)]
mod value_bridge_tests;
