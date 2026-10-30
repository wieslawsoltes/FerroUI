//! FerroUI metadata for the original ReactiveUI docking model library.
/// Identity-preserving views of the original model classes.
pub mod reactive_metadata;
mod register_types;
pub use register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod construction_tests;
