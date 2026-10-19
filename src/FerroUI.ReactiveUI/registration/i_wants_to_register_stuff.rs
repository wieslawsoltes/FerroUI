//! `IWantsToRegisterStuff` (`ReactiveUI.Core/Interfaces/IWantsToRegisterStuff.cs`).

use super::IRegistrar;

/// A type that registers services with ReactiveUI's dependency resolver (`IWantsToRegisterStuff`),
/// through the builder:
///
/// ```
/// use ferroui_reactiveui::{IRegistrar, IWantsToRegisterStuff, RegistrarExt};
/// use std::rc::Rc;
///
/// struct Greeting(&'static str);
///
/// struct MyRegistrations;
///
/// impl IWantsToRegisterStuff for MyRegistrations {
///     fn register(&self, registrar: &dyn IRegistrar) {
///         registrar.register_constant(|| Rc::new(Greeting("hello")));
///     }
/// }
/// ```
///
/// Assembly scanning, which upstream used to find these types, is not ported (reflection); a
/// builder is given them with `with_registration` or `with_platform_module`.
pub trait IWantsToRegisterStuff {
    /// Registers the services with `registrar` (`Register(IRegistrar)`).
    fn register(&self, registrar: &dyn IRegistrar);
}
