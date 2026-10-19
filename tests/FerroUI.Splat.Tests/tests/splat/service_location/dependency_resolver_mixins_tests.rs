//! Ported from `Splat.Tests/ServiceLocation/DependencyResolverMixinsTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `AppLocatorScope` at the start of each
//! test; a `using` block is the disposal of the scope `with_resolver` returns, and
//! `WithResolver(suppressResolverCallback: b)` is `with_resolver_suppressing(b)`. The
//! `Register<ITestService, TestService>()` of `Register_TypeBased_ShouldCreateNewInstanceEachTime`
//! is a factory closure that creates a new `TestService` (`Register<TService, TImplementation>` is
//! not ported). `IsEqualTo` of resolvers and services is the identity of the `Rc`.

use crate::tests::common::{same, AppLocatorScope};
use ferroui_splat::{
    AppLocator, DependencyResolverMixins, InstanceGenericFirstDependencyResolver,
    MutableDependencyResolverExt,
};
use std::cell::Cell;
use std::rc::Rc;

/// The marker service interface of the tests.
trait ITestService {}

/// The implementation of [`ITestService`].
struct TestService;

impl ITestService for TestService {}

#[test]
fn with_resolver_should_temporarily_override_resolver() {
    let _scope = AppLocatorScope::new();
    let original_resolver = AppLocator::get_locator();
    let test_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());

    let resolver_scope = test_resolver.with_resolver();
    assert!(same(&AppLocator::get_locator(), &test_resolver));
    resolver_scope.dispose();

    assert!(same(&AppLocator::get_locator(), &original_resolver));
}

#[test]
fn with_resolver_with_suppress_true_should_suppress_callbacks() {
    let _scope = AppLocatorScope::new();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();
    let _subscription = AppLocator::register_resolver_callback_changed(move || invoked.set(true));
    callback_invoked.set(false); // Reset

    let test_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());

    // Callback should not be invoked when suppressed
    test_resolver.with_resolver_suppressing(true).dispose();

    assert!(!callback_invoked.get());
}

#[test]
fn with_resolver_with_suppress_false_should_invoke_callbacks() {
    let _scope = AppLocatorScope::new();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();
    let _subscription = AppLocator::register_resolver_callback_changed(move || invoked.set(true));
    callback_invoked.set(false); // Reset

    let test_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());

    // Callback should be invoked
    test_resolver.with_resolver_suppressing(false).dispose();

    assert!(callback_invoked.get());
}

#[test]
fn with_resolver_nested_inside_a_suppressed_scope_stays_suppressed() {
    let _scope = AppLocatorScope::new();
    let notifications = Rc::new(Cell::new(0));
    let counter = notifications.clone();
    let subscription = AppLocator::register_resolver_callback_changed(move || counter.set(counter.get() + 1));
    notifications.set(0); // Reset after the immediate registration callback.

    let outer_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());
    let inner_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());

    let outer_scope = outer_resolver.with_resolver_suppressing(true);
    let inner_scope = inner_resolver.with_resolver_suppressing(false);
    assert!(!AppLocator::are_resolver_callback_changed_notifications_enabled());
    inner_scope.dispose();
    outer_scope.dispose();

    assert_eq!(notifications.get(), 0);
    subscription.dispose();
}

#[test]
fn with_resolver_disposed_twice_raises_the_change_notification_once() {
    let _scope = AppLocatorScope::new();
    let notifications = Rc::new(Cell::new(0));
    let counter = notifications.clone();
    let subscription = AppLocator::register_resolver_callback_changed(move || counter.set(counter.get() + 1));
    notifications.set(0); // Reset after the immediate registration callback.

    let original_resolver = AppLocator::get_locator();
    let test_resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());

    let resolver_scope = test_resolver.with_resolver_suppressing(false);
    resolver_scope.dispose();
    resolver_scope.dispose();

    const ENTER_AND_EXIT: i32 = 2;
    assert_eq!(notifications.get(), ENTER_AND_EXIT);
    assert!(same(&AppLocator::get_locator(), &original_resolver));
    subscription.dispose();
}

#[test]
fn register_type_based_should_create_new_instance_each_time() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    resolver.register::<dyn ITestService>(|| Some(Rc::new(TestService) as Rc<dyn ITestService>));

    let service1 = AppLocator::get_service::<dyn ITestService>().expect("a service");
    let service2 = AppLocator::get_service::<dyn ITestService>().expect("a service");

    assert!(!same(&service1, &service2));
}

// Not ported: WithResolver_WithNullResolver_ShouldThrow - argument-null validation, which Rust's types make impossible
// Not ported: RegisterConstant_TypeBased_WithNullResolver_ShouldThrow - argument-null validation, which Rust's types make impossible
// Not ported: RegisterConstant_TypeBased_ShouldRegisterInstance - the `Type`-based `RegisterConstant(object?, Type)` mixin, which is not ported
// Not ported: RegisterConstant_TypeBased_WithContract_ShouldRegisterInstance - the `Type`-based `RegisterConstant(object?, Type, string)` mixin, which is not ported
// Not ported: RegisterLazySingleton_TypeBased_WithNullResolver_ShouldThrow - argument-null validation, which Rust's types make impossible
// Not ported: RegisterLazySingleton_TypeBased_ShouldRegisterLazily - the `Type`-based `RegisterLazySingleton(Func<object?>, Type)` mixin, which is not ported
// Not ported: RegisterLazySingleton_TypeBased_WithContract_ShouldRegisterLazily - the `Type`-based `RegisterLazySingleton(Func<object?>, Type, string)` mixin, which is not ported
