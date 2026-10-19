//! `Registrations` (`ReactiveUI.Shared/Registration/Registrations.cs`).

use super::{IRegistrar, IWantsToRegisterStuff, RegistrarExt};
use crate::{CanActivateViewFetcher, DefaultViewLocator, IActivationForViewFetcher, IViewLocator};
use std::rc::Rc;

/// The registrations of ReactiveUI that every platform shares (`Registrations`): the default view
/// locator and the fetcher of the activation of the views that can activate. The builder applies
/// them with its core services.
#[derive(Debug, Default, Clone, Copy)]
pub struct Registrations;

impl IWantsToRegisterStuff for Registrations {
    fn register(&self, registrar: &dyn IRegistrar) {
        // `RegisterPlatformServices`.
        registrar.register_constant::<dyn IViewLocator>(|| Rc::new(DefaultViewLocator::new()) as Rc<dyn IViewLocator>);
        registrar.register_constant::<dyn IActivationForViewFetcher>(|| {
            Rc::new(CanActivateViewFetcher::new()) as Rc<dyn IActivationForViewFetcher>
        });
    }
}
