//! Ported from `Splat.Builder.Tests/AppBuilderTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `AppBuilderScope` at the start of each
//! test. `IsSameReferenceAs(builder)` is the identity of the `&dyn IAppBuilder` a method returns
//! with the builder; `IsNotNull` of the builder `CreateSplatBuilder` returns checks that it holds
//! the resolver of the locator. A registration that records its call shares a `Cell` with the test.

use super::mock_module::MockModule;
use crate::tests::common::{same, AppBuilderScope};
use ferroui_splat::service_location::InternalLocator;
use ferroui_splat::{AppBuilder, AppLocator, IAppBuilder, IAppInstance, IMutableDependencyResolver};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// `IsSameReferenceAs(builder)` of the builder a method returns.
fn same_builder(result: &dyn IAppBuilder, builder: &AppBuilder) -> bool {
    std::ptr::addr_eq(result, builder)
}

#[test]
fn constructor_sets_using_builder_true() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let _ = AppBuilder::new(resolver.current_mutable());
    assert!(AppBuilder::using_builder());
    resolver.dispose();
}

#[test]
fn create_splat_builder_returns_builder() {
    let _scope = AppBuilderScope::new();
    let builder = AppBuilder::create_splat_builder();
    assert!(builder.current().is_some_and(|current| same(&current, &AppLocator::current())));
}

#[test]
fn reset_builder_state_for_tests_resets_static_state() {
    let _scope = AppBuilderScope::new();
    AppBuilder::reset_builder_state_for_tests();
    assert!(!AppBuilder::has_been_built());
    assert!(!AppBuilder::using_builder());
}

#[test]
fn use_current_splat_locator_changes_resolver_provider() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = AppBuilder::new(resolver.current_mutable());
    let result = builder.use_current_splat_locator();
    assert!(same_builder(result, &builder));
    resolver.dispose();
}

#[test]
fn using_module_adds_module() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = AppBuilder::new(resolver.current_mutable());
    let result = builder.using_module(Rc::new(MockModule));
    assert!(same_builder(result, &builder));
    resolver.dispose();
}

#[test]
fn with_custom_registration_adds_action() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = AppBuilder::new(resolver.current_mutable());
    let result = builder.with_custom_registration(Rc::new(|_: &dyn IMutableDependencyResolver| {}));
    assert!(same_builder(result, &builder));
    resolver.dispose();
}

#[test]
fn with_core_services_returns_self() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = AppBuilder::new(resolver.current_mutable());
    let result = builder.with_core_services();
    assert!(same_builder(result, &builder));
    resolver.dispose();
}

#[test]
fn build_applies_registrations() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = AppBuilder::new(resolver.current_mutable());
    let called = Rc::new(Cell::new(false));
    let registration_called = called.clone();
    let _ = builder.with_custom_registration(Rc::new(move |_: &dyn IMutableDependencyResolver| {
        registration_called.set(true);
    }));
    let _ = builder.build();
    assert!(called.get());
    resolver.dispose();
}

#[test]
fn build_does_nothing_if_already_built() {
    let _scope = AppBuilderScope::new();
    let resolver = InternalLocator::new();
    let builder = AppBuilder::new(resolver.current_mutable());
    let _ = builder.build(); // sets has_been_built
    let called = Rc::new(Cell::new(false));
    let registration_called = called.clone();
    let _ = builder.with_custom_registration(Rc::new(move |_: &dyn IMutableDependencyResolver| {
        registration_called.set(true);
    }));
    let _ = builder.build(); // should not call registration again
    assert!(!called.get());
    resolver.dispose();
}

// Not ported: ConstructorThrowsOnNullResolver - argument-null validation, which Rust's types make impossible
// Not ported: UsingModuleThrowsOnNullModule - argument-null validation, which Rust's types make impossible
// Not ported: WithCustomRegistrationThrowsOnNullAction - argument-null validation, which Rust's types make impossible
