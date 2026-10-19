//! Ported from `Splat.Builder.Tests/MockModule.cs` of Splat.
//! Adapted: the helper is crate-visible, for the sibling test files.

use ferroui_splat::{IModule, IMutableDependencyResolver};

/// A module that registers nothing (`MockModule`).
pub(crate) struct MockModule;

impl IModule for MockModule {
    fn configure(&self, _resolver: &dyn IMutableDependencyResolver) {
        // This is a mock module for testing purposes.
        // It does not need to do anything specific.
        // In a real scenario, you would register services here.
    }
}
