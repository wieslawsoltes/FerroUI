//! `IViewModule` (`ReactiveUI.Shared/Interfaces/IViewModule.cs`).

use crate::DefaultViewLocator;

/// A module that maps view models to their views, for the view location that needs no
/// reflection (`IViewModule`): the views of a feature, registered together.
pub trait IViewModule {
    /// Maps the views of the module with `locator` (`RegisterViews(DefaultViewLocator)`).
    fn register_views(&self, locator: &DefaultViewLocator);
}
