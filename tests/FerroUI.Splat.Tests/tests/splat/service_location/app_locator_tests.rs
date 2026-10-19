//! Ported from `Splat.Tests/ServiceLocation/AppLocatorTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `AppLocatorScope` at the start of each
//! test; the service interface is the trait object `dyn ITestService`. `IsTypeOf<
//! InstanceGenericFirstDependencyResolver>` has no form on a `Rc<dyn IReadonlyDependencyResolver>`:
//! the tests check that the resolver is the one the locator holds and has the services the scope
//! registered. `FuncDependencyResolver` is not in ferroui-splat, so the resolver that
//! `SetLocator_ShouldUpdateResolver` sets is a `ModernDependencyResolver`; the `Type`-based
//! `Register(Func<object?>, Type)` that two tests register with is the generic `register` (the
//! type only names the service). `IsEqualTo` of services is the identity of the `Rc`.

use crate::tests::common::{same, AppLocatorScope};
use ferroui_splat::{
    AppLocator, IDependencyResolver, ILogger, ModernDependencyResolver, MutableDependencyResolverExt,
    ReadonlyDependencyResolverExt,
};
use std::cell::Cell;
use std::rc::Rc;

/// The marker service interface of the tests.
trait ITestService {}

/// The implementation of [`ITestService`].
struct TestService;

impl ITestService for TestService {}

#[test]
fn current_should_return_readonly_resolver() {
    let _scope = AppLocatorScope::new();

    let resolver = AppLocator::current();

    assert!(same(&resolver, &AppLocator::get_locator()));
    assert!(resolver.get_service::<dyn ILogger>().is_some());
}

#[test]
fn current_mutable_should_return_mutable_resolver() {
    let _scope = AppLocatorScope::new();

    let resolver = AppLocator::current_mutable();

    assert!(same(&resolver, &AppLocator::get_locator()));
    assert!(resolver.has_registration::<dyn ILogger>());
}

#[test]
fn get_locator_should_return_resolver() {
    let _scope = AppLocatorScope::new();

    let resolver = AppLocator::get_locator();

    assert!(same(&resolver, &AppLocator::current()));
    assert!(resolver.has_registration::<dyn ILogger>());
}

#[test]
fn set_locator_should_update_resolver() {
    let _scope = AppLocatorScope::new();
    let new_resolver: Rc<dyn IDependencyResolver> = Rc::new(ModernDependencyResolver::new());

    AppLocator::set_locator(new_resolver.clone());

    assert!(same(&AppLocator::get_locator(), &new_resolver));
}

#[test]
fn register_should_register_service() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);
    let registered = service.clone();
    AppLocator::register::<dyn ITestService>(move || Some(registered.clone() as Rc<dyn ITestService>));

    let resolved = AppLocator::get_service::<dyn ITestService>();

    assert!(resolved.is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn register_with_contract_should_register_service() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);
    let registered = service.clone();
    AppLocator::register_with_contract::<dyn ITestService>(
        move || Some(registered.clone() as Rc<dyn ITestService>),
        "test",
    );

    let resolved = AppLocator::get_service_with_contract::<dyn ITestService>("test");

    assert!(resolved.is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn register_constant_should_register_instance() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);
    AppLocator::register_constant::<dyn ITestService>(service.clone());

    let resolved = AppLocator::get_service::<dyn ITestService>();

    assert!(resolved.is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn register_constant_with_contract_should_register_instance() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);
    AppLocator::register_constant_with_contract::<dyn ITestService>(service.clone(), "test");

    let resolved = AppLocator::get_service_with_contract::<dyn ITestService>("test");

    assert!(resolved.is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn register_lazy_singleton_should_register_lazily() {
    let _scope = AppLocatorScope::new();
    let call_count = Rc::new(Cell::new(0));
    let count = call_count.clone();
    AppLocator::register_lazy_singleton::<dyn ITestService>(move || {
        count.set(count.get() + 1);
        Some(Rc::new(TestService) as Rc<dyn ITestService>)
    });

    assert_eq!(call_count.get(), 0);

    let service1 = AppLocator::get_service::<dyn ITestService>().expect("the singleton");
    assert_eq!(call_count.get(), 1);

    let service2 = AppLocator::get_service::<dyn ITestService>().expect("the singleton");
    assert_eq!(call_count.get(), 1); // Still 1, singleton
    assert!(same(&service2, &service1));
}

#[test]
fn register_lazy_singleton_with_contract_should_register_lazily() {
    let _scope = AppLocatorScope::new();
    let call_count = Rc::new(Cell::new(0));
    let count = call_count.clone();
    AppLocator::register_lazy_singleton_with_contract::<dyn ITestService>(
        move || {
            count.set(count.get() + 1);
            Some(Rc::new(TestService) as Rc<dyn ITestService>)
        },
        "test",
    );

    assert_eq!(call_count.get(), 0);

    let _ = AppLocator::get_service_with_contract::<dyn ITestService>("test");
    assert_eq!(call_count.get(), 1);
}

#[test]
fn get_service_should_return_registered_service() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);

    // Register via a factory (rather than a constant) to exercise get_service against a factory
    // registration.
    let registered = service.clone();
    AppLocator::current_mutable()
        .register::<dyn ITestService>(move || Some(registered.clone() as Rc<dyn ITestService>));

    let resolved = AppLocator::get_service::<dyn ITestService>();

    assert!(resolved.is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn get_service_with_contract_should_return_registered_service() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);

    // Register via a factory (rather than a constant) to exercise get_service against a factory
    // registration.
    let registered = service.clone();
    AppLocator::current_mutable().register_with_contract::<dyn ITestService>(
        move || Some(registered.clone() as Rc<dyn ITestService>),
        Some("test"),
    );

    let resolved = AppLocator::get_service_with_contract::<dyn ITestService>("test");

    assert!(resolved.is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn get_service_with_no_registration_should_return_null() {
    let _scope = AppLocatorScope::new();

    let resolved = AppLocator::get_service::<dyn ITestService>();

    assert!(resolved.is_none());
}

#[test]
fn get_services_should_return_all_registered_services() {
    let _scope = AppLocatorScope::new();
    let service1 = Rc::new(TestService);
    let service2 = Rc::new(TestService);
    AppLocator::register_constant::<dyn ITestService>(service1.clone());
    AppLocator::register_constant::<dyn ITestService>(service2.clone());

    let services = AppLocator::get_services::<dyn ITestService>();

    const EXPECTED_SERVICE_COUNT: usize = 2;
    assert_eq!(services.len(), EXPECTED_SERVICE_COUNT);
    assert!(services.iter().any(|service| same(service, &service1)));
    assert!(services.iter().any(|service| same(service, &service2)));
}

#[test]
fn get_services_with_contract_should_return_registered_services() {
    let _scope = AppLocatorScope::new();
    let service = Rc::new(TestService);
    AppLocator::register_constant_with_contract::<dyn ITestService>(service.clone(), "test");

    let services = AppLocator::get_services_with_contract::<dyn ITestService>("test");

    assert_eq!(services.len(), 1);
    assert!(same(&services[0], &service));
}

#[test]
fn has_registration_with_registered_service_should_return_true() {
    let _scope = AppLocatorScope::new();
    AppLocator::register_constant::<dyn ITestService>(Rc::new(TestService));

    let has_registration = AppLocator::has_registration::<dyn ITestService>();

    assert!(has_registration);
}

#[test]
fn has_registration_with_no_registration_should_return_false() {
    let _scope = AppLocatorScope::new();

    let has_registration = AppLocator::has_registration::<dyn ITestService>();

    assert!(!has_registration);
}

#[test]
fn has_registration_with_contract_should_return_true() {
    let _scope = AppLocatorScope::new();
    AppLocator::register_constant_with_contract::<dyn ITestService>(Rc::new(TestService), "test");

    let has_registration = AppLocator::has_registration_with_contract::<dyn ITestService>("test");

    assert!(has_registration);
}

#[test]
fn unregister_current_should_remove_last_registration() {
    let _scope = AppLocatorScope::new();
    let service1 = Rc::new(TestService);
    let service2 = Rc::new(TestService);
    AppLocator::register_constant::<dyn ITestService>(service1.clone());
    AppLocator::register_constant::<dyn ITestService>(service2.clone());

    AppLocator::unregister_current::<dyn ITestService>();

    let resolved = AppLocator::get_service::<dyn ITestService>();
    assert!(resolved.is_some_and(|resolved| same(&resolved, &service1)));
}

#[test]
fn unregister_current_with_contract_should_remove_last_registration() {
    let _scope = AppLocatorScope::new();
    AppLocator::register_constant_with_contract::<dyn ITestService>(Rc::new(TestService), "test");
    AppLocator::register_constant_with_contract::<dyn ITestService>(Rc::new(TestService), "test");

    AppLocator::unregister_current_with_contract::<dyn ITestService>("test");

    let has_registration = AppLocator::has_registration_with_contract::<dyn ITestService>("test");
    assert!(has_registration);
}

#[test]
fn unregister_all_should_remove_all_registrations() {
    let _scope = AppLocatorScope::new();
    AppLocator::register_constant::<dyn ITestService>(Rc::new(TestService));
    AppLocator::register_constant::<dyn ITestService>(Rc::new(TestService));

    AppLocator::unregister_all::<dyn ITestService>();

    let has_registration = AppLocator::has_registration::<dyn ITestService>();
    assert!(!has_registration);
}

#[test]
fn unregister_all_with_contract_should_remove_all_registrations() {
    let _scope = AppLocatorScope::new();
    AppLocator::register_constant_with_contract::<dyn ITestService>(Rc::new(TestService), "test");
    AppLocator::register_constant_with_contract::<dyn ITestService>(Rc::new(TestService), "test");

    AppLocator::unregister_all_with_contract::<dyn ITestService>("test");

    let has_registration = AppLocator::has_registration_with_contract::<dyn ITestService>("test");
    assert!(!has_registration);
}

#[test]
fn register_resolver_callback_changed_should_invoke_immediately() {
    let _scope = AppLocatorScope::new();
    let callback_invoked = Rc::new(Cell::new(false));
    let invoked = callback_invoked.clone();

    let _subscription = AppLocator::register_resolver_callback_changed(move || invoked.set(true));

    assert!(callback_invoked.get());
}

#[test]
fn suppress_resolver_callback_changed_notifications_should_suppress_callbacks() {
    let _scope = AppLocatorScope::new();
    assert!(AppLocator::are_resolver_callback_changed_notifications_enabled());

    let suppression = AppLocator::suppress_resolver_callback_changed_notifications();
    assert!(!AppLocator::are_resolver_callback_changed_notifications_enabled());
    suppression.dispose();

    assert!(AppLocator::are_resolver_callback_changed_notifications_enabled());
}

#[test]
fn are_resolver_callback_changed_notifications_enabled_by_default_should_be_true() {
    let _scope = AppLocatorScope::new();

    let enabled = AppLocator::are_resolver_callback_changed_notifications_enabled();

    assert!(enabled);
}

// Not ported: SetLocator_WithNullResolver_ShouldThrow - argument-null validation, which Rust's types make impossible
// Not ported: RegisterResolverCallbackChanged_WithNullCallback_ShouldThrow - argument-null validation, which Rust's types make impossible
