//! Per-host and global overlay services.

/// Root models exposing overlay services and reload behavior.
pub mod controls;
/// Host overlay service composition and resolution.
pub mod hosting;
/// Overlay service implementations.
pub mod services;

/// Source dependency resolver registrations for overlay services.
pub mod dependency_injection;
