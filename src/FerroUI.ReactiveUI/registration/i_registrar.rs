//! `IRegistrar` (`ReactiveUI.Core/Interfaces/IRegistrar.cs`).

use ferroui_splat::service_location::{erase_service, ErasedService, ServiceFactory, ServiceType};
use std::rc::Rc;

/// Registers services with a dependency injection container without reflection (`IRegistrar`):
/// what an [`IWantsToRegisterStuff`](super::IWantsToRegisterStuff) registers its services with.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): upstream's methods are generic in the type of the
/// service; a trait object cannot have generic methods, so the trait has the methods of the
/// erased services, keyed by their [`ServiceType`], as Splat's resolvers do, and the generic
/// methods of upstream are those of [`RegistrarExt`], which every registrar has.
pub trait IRegistrar {
    /// Registers the value that `factory` creates, once, as a constant of `service_type`
    /// (`RegisterConstant<TService>(Func<TService>, string?)`).
    fn register_constant_erased(&self, factory: Box<dyn FnOnce() -> ErasedService>, service_type: ServiceType, contract: Option<&str>);

    /// Registers `factory` as a lazy singleton of `service_type`: it creates the value the first
    /// time it is resolved (`RegisterLazySingleton<TService>(Func<TService>, string?)`).
    fn register_lazy_singleton_erased(&self, factory: ServiceFactory, service_type: ServiceType, contract: Option<&str>);

    /// Registers `factory`, which creates a value each time `service_type` is resolved
    /// (`Register<TService>(Func<TService>, string?)`).
    fn register_erased(&self, factory: ServiceFactory, service_type: ServiceType, contract: Option<&str>);
}

/// The generic methods of [`IRegistrar`] (`IRegistrar`'s `RegisterConstant<TService>`,
/// `RegisterLazySingleton<TService>` and `Register<TService>`). The service may be a trait object
/// (`registrar.register_constant::<dyn IViewLocator>(..)`).
pub trait RegistrarExt: IRegistrar {
    /// Registers the value that `factory` creates, once, as a constant
    /// (`RegisterConstant<TService>(Func<TService>)`).
    fn register_constant<S: ?Sized + 'static>(&self, factory: impl FnOnce() -> Rc<S> + 'static) {
        self.register_constant_with_contract(factory, None);
    }

    /// Registers the value that `factory` creates, once, as a constant under `contract`
    /// (`RegisterConstant<TService>(Func<TService>, string?)`).
    fn register_constant_with_contract<S: ?Sized + 'static>(&self, factory: impl FnOnce() -> Rc<S> + 'static, contract: Option<&str>) {
        self.register_constant_erased(Box::new(move || erase_service(factory())), ServiceType::of::<S>(), contract);
    }

    /// Registers `factory` as a lazy singleton (`RegisterLazySingleton<TService>(Func<TService>)`).
    fn register_lazy_singleton<S: ?Sized + 'static>(&self, factory: impl Fn() -> Rc<S> + 'static) {
        self.register_lazy_singleton_with_contract(factory, None);
    }

    /// Registers `factory` as a lazy singleton under `contract`
    /// (`RegisterLazySingleton<TService>(Func<TService>, string?)`).
    fn register_lazy_singleton_with_contract<S: ?Sized + 'static>(&self, factory: impl Fn() -> Rc<S> + 'static, contract: Option<&str>) {
        self.register_lazy_singleton_erased(Rc::new(move || Some(erase_service(factory()))), ServiceType::of::<S>(), contract);
    }

    /// Registers `factory`, which creates a value at each resolution (`Register<TService>(Func<TService>)`).
    fn register<S: ?Sized + 'static>(&self, factory: impl Fn() -> Rc<S> + 'static) {
        self.register_with_contract(factory, None);
    }

    /// Registers `factory` under `contract` (`Register<TService>(Func<TService>, string?)`).
    fn register_with_contract<S: ?Sized + 'static>(&self, factory: impl Fn() -> Rc<S> + 'static, contract: Option<&str>) {
        self.register_erased(Rc::new(move || Some(erase_service(factory()))), ServiceType::of::<S>(), contract);
    }
}

impl<R: IRegistrar + ?Sized> RegistrarExt for R {}
