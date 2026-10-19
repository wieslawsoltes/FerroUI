//! `IScreen` (`ReactiveUI.Shared/Interfaces/IScreen.cs`).

use crate::RoutingState;
use std::rc::Rc;

/// An object that hosts a navigation stack through a [`RoutingState`] (`IScreen`).
///
/// An application usually has one screen (its shell or main view model) that owns the router;
/// the view models it navigates to take the screen so that they can navigate further without
/// knowing the views.
pub trait IScreen {
    /// The router of the screen, which coordinates the navigation of the view models attached to
    /// it (`Router`).
    fn router(&self) -> Rc<RoutingState>;
}
