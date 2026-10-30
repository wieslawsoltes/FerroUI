//! ReactiveUI services for the framework-independent docking model.
//!
//! Services follow FerroUI's reactive thread affinity and use the shared System,
//! Rx, and ReactiveUI ports. They have no dependency on controls or windowing.

/// Window lifecycle cleanup services.
pub mod lifecycle;
/// ReactiveUI routing models and docking navigation.
pub mod navigation;
/// Busy, confirmation, and dialog overlays.
pub mod overlays;
#[cfg(test)]
mod test_environment;
/// Default dispatcher implementations.
pub mod threading;
