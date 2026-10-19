//! `ReactiveUIBuilderCoreTests` (`ReactiveUI.Builder.Tests/ReactiveUIBuilderCoreTests.cs`).
//!
//! Each test runs in its executor: the default one builds the core services
//! ([`builder_test`]), and the executors of the class configure the builder their own way.

use super::executors::{builder_test, execute_test, reset_state};
use ferroui_reactiveui::*;
use ferroui_splat::{AppLocator, IMutableDependencyResolver, MutableDependencyResolverExt};
use std::cell::Cell;
use std::rc::Rc;

ferroui_system::ferro_thread_local! {
    /// `CustomServiceRegistered` of the executors of the class.
    static CUSTOM_SERVICE_REGISTERED: Cell<bool> = const { Cell::new(false) };
}

fn custom_service_registered() -> bool {
    CUSTOM_SERVICE_REGISTERED.with(Cell::get)
}

/// The service that the custom registrations register (`string`).
fn registered_string() -> Option<String> {
    AppLocator::get_service::<String>().map(|value| (*value).clone())
}

#[test]
fn create_builder_should_return_builder_instance() {
    builder_test(|| {
        // `IsTypeOf<ReactiveUIBuilder>()`: the type of the value the function returns.
        let builder: ReactiveUIBuilder = RxAppBuilder::create_reactive_ui_builder();
        assert!(builder.current().is_some());
    });
}

#[test]
fn with_core_services_should_register_core_services() {
    builder_test(|| {
        assert!(AppLocator::get_service::<dyn IViewLocator>().is_some());
        assert!(AppLocator::get_service::<dyn IActivationForViewFetcher>().is_some());
    });
}

/// `WithCustomRegistrationExecutor`.
fn with_custom_registration_executor() {
    CUSTOM_SERVICE_REGISTERED.with(|flag| flag.set(false));
    let _ = RxAppBuilder::create_reactive_ui_builder()
        .with_custom_registration(|resolver: &dyn IMutableDependencyResolver| {
            resolver.register_constant(Rc::new("TestValue".to_owned()));
            CUSTOM_SERVICE_REGISTERED.with(|flag| flag.set(true));
        })
        .with_core_services()
        .build_app();
}

#[test]
fn with_custom_registration_should_execute_custom_action() {
    execute_test(reset_state, with_custom_registration_executor, || {
        assert!(custom_service_registered());
        assert_eq!(registered_string().as_deref(), Some("TestValue"));
    });
}

#[test]
fn build_should_always_register_core_services() {
    builder_test(|| {
        assert!(AppLocator::get_service::<dyn IViewLocator>().is_some());
    });
}

// Not ported: WithCustomRegistration_With_Null_Action_Should_Throw - argument validation
// Not ported: WithViewsFromAssembly_Should_Register_Views - reflection
// Not ported: WithViewsFromAssembly_With_Null_Assembly_Should_Throw - reflection

/// `FluentChainingExecutor`.
fn fluent_chaining_executor() {
    CUSTOM_SERVICE_REGISTERED.with(|flag| flag.set(false));
    let _ = RxAppBuilder::create_reactive_ui_builder()
        .with_core_services()
        .with_custom_registration(|resolver: &dyn IMutableDependencyResolver| {
            resolver.register_constant(Rc::new("Test".to_owned()));
            CUSTOM_SERVICE_REGISTERED.with(|flag| flag.set(true));
        })
        .build_app();
}

#[test]
fn builder_should_support_fluent_chaining() {
    execute_test(reset_state, fluent_chaining_executor, || {
        assert!(custom_service_registered());
        assert_eq!(registered_string().as_deref(), Some("Test"));

        assert!(AppLocator::get_service::<dyn IViewLocator>().is_some());
    });
}

// Not ported: CreateReactiveUIBuilder_With_Null_Resolver_Should_Throw - argument validation

#[test]
fn build_app_should_return_reactive_instance() {
    builder_test(|| {
        // The executor built the application; the locator has a resolver.
        assert!(AppLocator::get_service::<dyn IViewLocator>().is_some());
    });
}

#[test]
fn build_should_return_reactive_instance_and_initialize_reactive_ui() {
    builder_test(|| {
        RxAppBuilder::reset_for_testing();

        let builder = RxAppBuilder::create_reactive_ui_builder();
        let _ = builder.with_core_services();

        let app: &dyn IReactiveUIInstance = builder.build();

        RxAppBuilder::ensure_initialized();

        assert!(app.current().is_some());
    });
}

#[test]
fn build_app_should_use_build_implementation() {
    builder_test(|| {
        RxAppBuilder::reset_for_testing();

        let builder = RxAppBuilder::create_reactive_ui_builder();
        let app: &dyn IReactiveUIInstance = builder.with_core_services().build_app();

        RxAppBuilder::ensure_initialized();

        assert!(app.current().is_some());
    });
}

// Not ported: ForPlatforms_With_Null_Array_Should_Throw - argument validation
