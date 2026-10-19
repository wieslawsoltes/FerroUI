//! `RxBindingBuilderTests` (ReactiveUI.Binding.Tests, `Builder/RxBindingBuilderTests.cs`).

use crate::mocks::panic_message;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::*;
use ferroui_splat::{
    IAppInstance, IMutableDependencyResolver, LazyDisposal, ModernDependencyResolver, RegistrationCallback, ServiceFactory, ServiceType,
};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// A resolver that registers nothing and cannot resolve (`MutableOnlyResolver`).
struct MutableOnlyResolver;

impl IMutableDependencyResolver for MutableOnlyResolver {
    fn has_registration_erased(&self, _service_type: ServiceType, _contract: Option<&str>) -> bool {
        false
    }

    fn register_erased(&self, _factory: ServiceFactory, _service_type: ServiceType, _contract: Option<&str>) {}

    fn register_constant_erased(
        &self,
        _value: Option<ferroui_splat::ErasedService>,
        _disposal: Option<Rc<dyn Fn()>>,
        _service_type: ServiceType,
        _contract: Option<&str>,
    ) {
    }

    fn register_lazy_singleton_erased(&self, _factory: ServiceFactory, _disposal: Option<LazyDisposal>, _service_type: ServiceType, _contract: Option<&str>) {}

    fn unregister_current_erased(&self, _service_type: ServiceType, _contract: Option<&str>) {}

    fn unregister_all_erased(&self, _service_type: ServiceType, _contract: Option<&str>) {}

    fn service_registration_callback_erased(&self, _service_type: ServiceType, _contract: Option<&str>, _callback: RegistrationCallback) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn create_reactive_ui_binding_builder_returns_builder() {
    RxBindingBuilder::reset_for_testing();

    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();

    assert!(builder.current().is_some());
}

#[test]
fn ensure_initialized_not_initialized_throws_invalid_operation_exception() {
    RxBindingBuilder::reset_for_testing();

    let message = panic_message(RxBindingBuilder::ensure_initialized);

    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn ensure_initialized_after_build_app_does_not_throw() {
    RxBindingBuilder::reset_for_testing();

    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();
    let _ = builder.build_app();

    RxBindingBuilder::ensure_initialized();
}

#[test]
fn reset_for_testing_resets_initialization_state() {
    RxBindingBuilder::reset_for_testing();

    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();
    let _ = builder.build_app();

    // Does not panic.
    RxBindingBuilder::ensure_initialized();

    RxBindingBuilder::reset_for_testing();

    // Panics again.
    let message = panic_message(RxBindingBuilder::ensure_initialized);
    assert!(message.contains("InvalidOperationException"));
}

#[test]
fn with_core_services_called_twice_is_idempotent() {
    RxBindingBuilder::reset_for_testing();

    let builder = RxBindingBuilder::create_reactive_ui_binding_builder();
    let _ = builder.with_core_services();
    // The second call registers nothing.
    let _ = builder.with_core_services();
    let _ = builder.build_app();

    RxBindingBuilder::ensure_initialized();
}

#[test]
fn create_reactive_ui_binding_builder_with_resolver_returns_builder() {
    RxBindingBuilder::reset_for_testing();

    let resolver = Rc::new(ModernDependencyResolver::new());
    let builder = resolver.create_reactive_ui_binding_builder();

    assert!(builder.current().is_some());
}

// Not ported: CreateReactiveUIBindingBuilder_NullResolver_ThrowsArgumentNullException - argument validation

#[test]
fn create_reactive_ui_binding_builder_resolver_not_readonly_falls_back_to_app_locator() {
    RxBindingBuilder::reset_for_testing();

    let resolver: Rc<dyn IMutableDependencyResolver> = Rc::new(MutableOnlyResolver);
    let builder = resolver.create_reactive_ui_binding_builder();

    assert!(builder.current().is_some());
}
