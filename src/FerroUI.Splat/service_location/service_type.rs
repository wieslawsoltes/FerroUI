//! The key and the type-erased values of a registration.

use ferroui_system::IDisposable;
use std::any::{Any, TypeId};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// The type of a service, the key of its registrations (the `Type` of a registration upstream,
/// `typeof(T)` of the generic forms).
///
/// It is the [`TypeId`] of the service type, which may be unsized (`dyn IViewLocator`), with the
/// name of the type for messages. Two values are equal when their types are.
#[derive(Clone, Copy)]
pub struct ServiceType {
    id: TypeId,
    name: &'static str,
}

impl ServiceType {
    /// The service type `S`.
    pub fn of<S: ?Sized + 'static>() -> Self {
        Self { id: TypeId::of::<S>(), name: std::any::type_name::<S>() }
    }

    /// The identifier of the type.
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// The name of the type, as `std::any::type_name` gives it.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl PartialEq for ServiceType {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ServiceType {}

impl Hash for ServiceType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for ServiceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// A service, type-erased: an `Rc<dyn Any>` that holds the `Rc<S>` of the service (the `object` a
/// resolver hands out upstream). [`erase_service`] makes one and [`unerase_service`] reads it back.
pub type ErasedService = Rc<dyn Any>;

/// The factory of a registration, type-erased (`Func<object?>`): it returns the service, or `None`
/// (upstream's `null`).
pub type ServiceFactory = Rc<dyn Fn() -> Option<ErasedService>>;

/// The callback of a registration (`Action<IDisposable>`): it receives a disposable that ends the
/// subscription when the callback disposes it.
pub type RegistrationCallback = Rc<dyn Fn(&dyn IDisposable)>;

/// What disposing a resolver does with the value of a lazy singleton that was created (upstream
/// disposes it when it is `IDisposable`).
pub type LazyDisposal = Rc<dyn Fn(&ErasedService)>;

/// The service `value`, type-erased.
pub fn erase_service<S: ?Sized + 'static>(value: Rc<S>) -> ErasedService {
    Rc::new(value)
}

/// The `Rc<S>` an erased service holds, or `None` when it holds a service of another type.
pub fn unerase_service<S: ?Sized + 'static>(value: &ErasedService) -> Option<Rc<S>> {
    (**value).downcast_ref::<Rc<S>>().cloned()
}
