//! `RxAppBuilderTest` (`ReactiveUI.Tests/RxAppBuilderTest.cs`).
//!
//! `IsTypeOf<ReactiveUIBuilder>()` is the type of the value the function returns.

use ferroui_reactiveui::*;
use ferroui_splat::service_location::{ErasedService, LazyDisposal, RegistrationCallback, ServiceFactory, ServiceType};
use ferroui_splat::{IMutableDependencyResolver, IReadonlyDependencyResolver};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A resolver that resolves and stores nothing (`TestResolver`).
struct TestResolver;

struct NoOpDisposable;

impl IDisposable for NoOpDisposable {
    fn dispose(&self) {}
}

impl IReadonlyDependencyResolver for TestResolver {
    fn get_service_erased(&self, _service_type: ServiceType, _contract: Option<&str>) -> Option<ErasedService> {
        None
    }

    fn get_services_erased(&self, _service_type: ServiceType, _contract: Option<&str>) -> Vec<ErasedService> {
        Vec::new()
    }
}

impl IMutableDependencyResolver for TestResolver {
    fn has_registration_erased(&self, _service_type: ServiceType, _contract: Option<&str>) -> bool {
        false
    }

    fn register_erased(&self, _factory: ServiceFactory, _service_type: ServiceType, _contract: Option<&str>) {
        // The test resolver stores no registrations.
    }

    fn register_constant_erased(
        &self,
        _value: Option<ErasedService>,
        _disposal: Option<Rc<dyn Fn()>>,
        _service_type: ServiceType,
        _contract: Option<&str>,
    ) {
        // The test resolver stores no registrations.
    }

    fn register_lazy_singleton_erased(
        &self,
        _factory: ServiceFactory,
        _disposal: Option<LazyDisposal>,
        _service_type: ServiceType,
        _contract: Option<&str>,
    ) {
        // The test resolver stores no registrations.
    }

    fn unregister_current_erased(&self, _service_type: ServiceType, _contract: Option<&str>) {
        // The test resolver stores no registrations to remove.
    }

    fn unregister_all_erased(&self, _service_type: ServiceType, _contract: Option<&str>) {
        // The test resolver stores no registrations to remove.
    }

    fn service_registration_callback_erased(
        &self,
        _service_type: ServiceType,
        _contract: Option<&str>,
        _callback: RegistrationCallback,
    ) -> Rc<dyn IDisposable> {
        Rc::new(NoOpDisposable)
    }
}

// Not ported: CreateReactiveUIBuilder_NullResolver_Throws - argument validation

#[test]
fn create_reactive_ui_builder_returns_builder() {
    let builder: ReactiveUIBuilder = RxAppBuilder::create_reactive_ui_builder();
    assert!(builder.current().is_some());
}

#[test]
fn create_reactive_ui_builder_with_resolver_returns_builder() {
    let resolver = Rc::new(TestResolver);
    let builder: ReactiveUIBuilder = resolver.create_reactive_ui_builder();
    assert!(builder.current().is_some());
}
