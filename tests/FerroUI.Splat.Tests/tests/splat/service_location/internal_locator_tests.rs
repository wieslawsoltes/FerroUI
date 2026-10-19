//! Ported from `Splat.Tests/ServiceLocation/InternalLocatorTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `InternalLocatorScope` at the start of
//! each test, whose `locator` is the locator under test. `FuncDependencyResolver` is not in
//! ferroui-splat, so the resolvers the tests set are `ModernDependencyResolver`s. `IsTypeOf<
//! InstanceGenericFirstDependencyResolver>` has no form on a `Rc<dyn IDependencyResolver>`: the
//! default resolver is checked by registering with it and resolving from it. A callback that sets
//! the locator again holds the locator weakly, as a strong reference would be a cycle. The
//! `Type`-based `GetService(typeof(ITestService))` is the generic `get_service` (the type only
//! names the service).

use crate::tests::common::{same, InternalLocatorScope};
use ferroui_splat::service_location::InternalLocator;
use ferroui_splat::{
    IDependencyResolver, ModernDependencyResolver, MutableDependencyResolverExt,
    ReadonlyDependencyResolverExt,
};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The marker service interface of the tests.
trait ITestService {}

/// The implementation of [`ITestService`].
struct TestService;

impl ITestService for TestService {}

/// A new resolver to set as the locator's.
fn new_resolver() -> Rc<dyn IDependencyResolver> {
    Rc::new(ModernDependencyResolver::new())
}

#[test]
fn constructor_should_initialize_with_default_resolver() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();

    let resolver = locator.internal();
    let service = Rc::new(TestService);
    resolver.register_constant::<dyn ITestService>(service.clone());

    assert!(resolver.get_service::<dyn ITestService>().is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn current_should_return_internal() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();

    assert!(same(&locator.current(), &locator.internal()));
}

#[test]
fn current_mutable_should_return_internal() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();

    assert!(same(&locator.current_mutable(), &locator.internal()));
}

#[test]
fn set_locator_should_update_internal() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let new_resolver = new_resolver();

    locator.set_locator(new_resolver.clone());

    assert!(same(&locator.internal(), &new_resolver));
}

#[test]
fn set_locator_should_invoke_registered_callbacks() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();

    let _subscription = locator.register_resolver_callback_changed(Rc::new(move || invoked.set(true)));
    callback_invoked.set(false); // Reset after initial registration

    locator.set_locator(new_resolver());

    assert!(callback_invoked.get());
}

#[test]
fn set_locator_with_suppressed_notifications_should_not_invoke_callbacks() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();

    let _subscription = locator.register_resolver_callback_changed(Rc::new(move || invoked.set(true)));
    callback_invoked.set(false); // Reset after initial registration

    let suppression = locator.suppress_resolver_callback_changed_notifications();
    locator.set_locator(new_resolver());

    assert!(!callback_invoked.get());
    suppression.dispose();
}

#[test]
fn register_resolver_callback_changed_should_invoke_callback_immediately() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();

    let _subscription = locator.register_resolver_callback_changed(Rc::new(move || invoked.set(true)));

    assert!(callback_invoked.get());
}

#[test]
fn register_resolver_callback_changed_when_disposed_should_remove_callback() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let call_count = Rc::new(Cell::new(0));
    let count = call_count.clone();

    let subscription = locator.register_resolver_callback_changed(Rc::new(move || count.set(count.get() + 1)));
    call_count.set(0); // Reset after initial registration

    subscription.dispose();

    locator.set_locator(new_resolver());

    assert_eq!(call_count.get(), 0);
}

#[test]
fn register_resolver_callback_changed_with_multiple_callbacks_should_invoke_all() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let callback1_invoked = Rc::new(Cell::new(false));
    let callback2_invoked = Rc::new(Cell::new(false));
    let invoked1 = callback1_invoked.clone();
    let invoked2 = callback2_invoked.clone();

    let _subscription1 = locator.register_resolver_callback_changed(Rc::new(move || invoked1.set(true)));
    let _subscription2 = locator.register_resolver_callback_changed(Rc::new(move || invoked2.set(true)));

    callback1_invoked.set(false);
    callback2_invoked.set(false);

    locator.set_locator(new_resolver());

    assert!(callback1_invoked.get());
    assert!(callback2_invoked.get());
}

#[test]
fn suppress_resolver_callback_changed_notifications_should_disable_callbacks() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    assert!(locator.are_resolver_callback_changed_notifications_enabled());

    let suppression = locator.suppress_resolver_callback_changed_notifications();
    assert!(!locator.are_resolver_callback_changed_notifications_enabled());
    suppression.dispose();

    assert!(locator.are_resolver_callback_changed_notifications_enabled());
}

#[test]
fn suppress_resolver_callback_changed_notifications_can_be_nested() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();

    let outer = locator.suppress_resolver_callback_changed_notifications();
    assert!(!locator.are_resolver_callback_changed_notifications_enabled());

    let inner = locator.suppress_resolver_callback_changed_notifications();
    assert!(!locator.are_resolver_callback_changed_notifications_enabled());
    inner.dispose();

    assert!(!locator.are_resolver_callback_changed_notifications_enabled());
    outer.dispose();

    assert!(locator.are_resolver_callback_changed_notifications_enabled());
}

#[test]
fn register_resolver_callback_changed_with_suppressed_notifications_should_not_invoke_immediately() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();

    let suppression = locator.suppress_resolver_callback_changed_notifications();
    let _subscription = locator.register_resolver_callback_changed(Rc::new(move || invoked.set(true)));

    assert!(!callback_invoked.get());
    suppression.dispose();

    // After suppression is lifted, callback is not retroactively invoked
    assert!(!callback_invoked.get());
}

#[test]
fn dispose_should_dispose_internal() {
    let _scope = InternalLocatorScope::new();
    let locator = InternalLocator::new();
    let resolver = locator.internal();

    locator.dispose();

    // The internal resolver is disposed; an InstanceGenericFirstDependencyResolver resolves nothing
    // once disposed.
    let result = resolver.get_service::<dyn ITestService>();
    assert!(result.is_none());
}

#[test]
fn dispose_called_multiple_times_should_not_throw() {
    let _scope = InternalLocatorScope::new();
    let locator = InternalLocator::new();

    locator.dispose();

    locator.dispose();
    assert!(locator.is_disposed());
}

#[test]
fn set_locator_should_not_cause_deadlock_with_reentrant_callback() {
    let scope = InternalLocatorScope::new();
    let locator = scope.locator.clone();
    let call_count = Rc::new(Cell::new(0));
    let count = call_count.clone();
    let weak_locator = Rc::downgrade(&locator);

    let _subscription = locator.register_resolver_callback_changed(Rc::new(move || {
        count.set(count.get() + 1);
        if count.get() != 1 {
            return;
        }

        // This would cause a deadlock if the implementation doesn't handle it
        if let Some(locator) = weak_locator.upgrade() {
            locator.set_locator(new_resolver());
        }
    }));

    // Should complete without deadlock
    assert!(call_count.get() > 0);
}

// Not ported: SetLocator_WithNullResolver_ShouldThrowArgumentNullException - argument-null validation, which Rust's types make impossible
// Not ported: RegisterResolverCallbackChanged_WithNullCallback_ShouldThrowNullReferenceException - argument-null validation, which Rust's types make impossible
// Not ported: Constructor_Callback_WhenCurrentMutableIsNull_ShouldExitEarly - a null resolver (`Internal = null!`), which Rust's types make impossible
