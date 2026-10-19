//! `IRoutableViewModel` (`ReactiveUI.Shared/Interfaces/IRoutableViewModel.cs`).

use crate::{IReactiveObject, IScreen};
use std::any::Any;
use std::rc::Rc;

/// A view model that takes part in the navigation of a [`RoutingState`](crate::RoutingState)
/// (`IRoutableViewModel`).
///
/// A routable view model names itself with a segment of a path, for diagnostics and breadcrumbs,
/// and knows the screen that hosts it, so that it can navigate further. It is shared as an
/// `Rc<dyn IRoutableViewModel>`; two of them are equal when they are the same object, as .NET
/// compares references.
///
/// The trait has `Any` as a supertrait, so that the stack can be searched for a view model of a
/// type ([`RoutingStateMixins::find_view_model_in_stack`](crate::RoutingStateMixins::find_view_model_in_stack)).
pub trait IRoutableViewModel: IReactiveObject + Any {
    /// A token that names the view model, such as `"login"` or `"user"` (`UrlPathSegment`).
    fn url_path_segment(&self) -> Option<String>;

    /// The screen that hosts the view model (`HostScreen`).
    // Deviation (DEVIATIONS.md, ReactiveUI): upstream's `HostScreen` is not nullable; here it is an
    // `Option`. The screen owns the router, whose stack owns the view model, so a view model that
    // held its screen by an `Rc` would make a cycle of strong references; a view model holds it by
    // a `Weak` and returns `None` once the screen is gone (and the tests of upstream that return
    // `null!` return `None`).
    fn host_screen(&self) -> Option<Rc<dyn IScreen>>;
}

impl PartialEq for dyn IRoutableViewModel {
    /// The same view model (reference equality, as for the objects of .NET).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

impl std::fmt::Debug for dyn IRoutableViewModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IRoutableViewModel")
            .field("url_path_segment", &self.url_path_segment())
            .field("address", &(self as *const Self as *const ()))
            .finish()
    }
}
