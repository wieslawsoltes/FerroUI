//! `SingleInstanceViewAttribute` (ReactiveUI.Binding, `View/SingleInstanceViewAttribute.cs`).

/// A view created once and reused each time its view model's view is resolved
/// (`SingleInstanceViewAttribute`): an option of a view registration
/// ([`ViewRegistrationOptions`](crate::ViewRegistrationOptions)). Not for a view that may appear in
/// several places of the tree of views at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SingleInstanceView;
