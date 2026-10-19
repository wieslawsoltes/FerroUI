//! `ExcludeFromViewRegistrationAttribute` (ReactiveUI.Binding,
//! `View/ExcludeFromViewRegistrationAttribute.cs`).

/// Leaves a view out of the view dispatch (`ExcludeFromViewRegistrationAttribute`): an option of a
/// view registration ([`ViewRegistrationOptions`](crate::ViewRegistrationOptions)), with which
/// [`ViewRegistrations`](crate::ViewRegistrations) skips the view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ExcludeFromViewRegistration;
