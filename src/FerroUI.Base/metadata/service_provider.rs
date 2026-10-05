//! The service provider contract used by markup: the equivalent of the
//! runtime library's `IServiceProvider`.

use std::any::{Any, TypeId};
use std::rc::Rc;

/// Retrieves service objects by the type of their handle.
///
/// A service is identified by the Rust type of the handle it is used
/// through, normally the handle of a contract (`Rc<dyn IProvideValueTarget>`,
/// `Rc<dyn IRootObjectProvider>`, `Rc<dyn INameScope>`). The provider returns
/// the handle boxed as [`Any`]; the box holds exactly a value of the
/// requested type.
///
/// ```ignore
/// let target: Option<Rc<dyn IProvideValueTarget>> = service_provider.get_service_of();
/// ```
pub trait IServiceProvider {
    /// Gets the service whose handle type is `service_type`, or `None` if
    /// the provider has no such service. The returned box holds a value of
    /// exactly the type identified by `service_type`.
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>>;
}

impl<'a> dyn IServiceProvider + 'a {
    /// Gets the service used through the handle type `T`
    /// (`GetService(typeof(T))` followed by the cast to `T`).
    pub fn get_service_of<T: Clone + 'static>(&self) -> Option<T> {
        self.get_service(TypeId::of::<T>())?.downcast_ref::<T>().cloned()
    }
}

/// Service provider handles compare by identity, so that they can be held
/// in untyped values.
impl<'a> PartialEq for dyn IServiceProvider + 'a {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

/// Boxes a service handle for [`IServiceProvider::get_service`] if it is the
/// requested one. Meant for implementations:
///
/// ```ignore
/// fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
///     service(service_type, || self.name_scope.clone())
///         .or_else(|| service(service_type, || self.root_object_provider()))
/// }
/// ```
pub fn service<T: 'static>(service_type: TypeId, handle: impl FnOnce() -> T) -> Option<Rc<dyn Any>> {
    if service_type == TypeId::of::<T>() {
        Some(Rc::new(handle()))
    } else {
        None
    }
}

/// A service provider without services.
pub struct EmptyServiceProvider;

impl IServiceProvider for EmptyServiceProvider {
    fn get_service(&self, _service_type: TypeId) -> Option<Rc<dyn Any>> {
        None
    }
}
