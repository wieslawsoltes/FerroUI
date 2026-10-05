use super::{
    IActivityApplicationLifetime, IControlledApplicationLifetime, ISetupApplicationLifetime,
    ISingleViewApplicationLifetime,
};
use std::any::Any;

/// The lifetime of an application: the object that decides when the
/// application starts and ends and what its main view is.
///
/// The more specific lifetime contracts are reached through the `as_*`
/// methods, which a lifetime overrides for every contract it implements, or
/// through [`as_any`](Self::as_any) for a concrete lifetime type.
pub trait IApplicationLifetime {
    /// The lifetime as its concrete type.
    fn as_any(&self) -> &dyn Any;

    /// The lifetime as a controlled lifetime, if it is one.
    fn as_controlled_application_lifetime(&self) -> Option<&dyn IControlledApplicationLifetime> {
        None
    }

    /// The lifetime as a single view lifetime, if it is one.
    fn as_single_view_application_lifetime(&self) -> Option<&dyn ISingleViewApplicationLifetime> {
        None
    }

    /// The lifetime as a single top-level lifetime, if it is one.
    fn as_single_top_level_application_lifetime(&self) -> Option<&dyn super::ISingleTopLevelApplicationLifetime> {
        None
    }

    /// The lifetime as an activity lifetime, if it is one.
    fn as_activity_application_lifetime(&self) -> Option<&dyn IActivityApplicationLifetime> {
        None
    }

    /// The lifetime as a lifetime that takes part in the setup of the
    /// application, if it is one.
    fn as_setup_application_lifetime(&self) -> Option<&dyn ISetupApplicationLifetime> {
        None
    }

    /// The lifetime as a classic desktop style lifetime, if it is one.
    fn as_classic_desktop_style_application_lifetime(
        &self,
    ) -> Option<&dyn super::IClassicDesktopStyleApplicationLifetime> {
        None
    }
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IApplicationLifetime {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
