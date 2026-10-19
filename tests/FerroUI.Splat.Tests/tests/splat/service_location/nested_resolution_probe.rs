//! Ported from `Splat.Tests/ServiceLocation/NestedResolutionProbe.cs` of Splat.
//! Adapted: the resolver is a `&dyn IReadonlyDependencyResolver` (upstream an
//! `IDependencyResolver`; only the lookup is needed), the service is an `Option<Rc<T>>`, and the
//! `Service` property is the method `service`.

use ferroui_splat::{IReadonlyDependencyResolver, ReadonlyDependencyResolverExt};
use std::rc::Rc;

/// Captures the result of a nested generic service lookup performed during its own construction
/// (`NestedResolutionProbe<T>`).
pub(crate) struct NestedResolutionProbe<T: ?Sized + 'static> {
    service: Option<Rc<T>>,
}

impl<T: ?Sized + 'static> NestedResolutionProbe<T> {
    /// A probe that resolves `T` from `resolver` while it is constructed.
    pub(crate) fn new(resolver: &dyn IReadonlyDependencyResolver) -> Self {
        Self { service: resolver.get_service::<T>() }
    }

    /// The service resolved by the nested generic lookup performed during construction
    /// (`Service`).
    pub(crate) fn service(&self) -> Option<&Rc<T>> {
        self.service.as_ref()
    }
}
