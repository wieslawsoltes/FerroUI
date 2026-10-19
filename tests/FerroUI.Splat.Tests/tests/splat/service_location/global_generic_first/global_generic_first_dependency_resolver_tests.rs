//! Ported from `Splat.Tests/ServiceLocation/GlobalGenericFirst/GlobalGenericFirstDependencyResolverTests.cs` of Splat.
//! Adapted: the inherited tests of `BaseDependencyResolverTests<GlobalGenericFirstDependencyResolver>`
//! are the instantiation of `base_dependency_resolver_tests!` below (none is overridden); the
//! expression that makes their resolver first clears the global containers, as upstream's
//! `[After(Test)]` `Clear` does between tests (the containers are per thread in the port, and each
//! test runs on a thread of its own, so a test starts empty anyway). The `Clear` calls of the tests
//! are kept. The callbacks of this resolver run for any registration of the resolver, and once per
//! existing registration on subscription (upstream twice for a generic registration, which it
//! mirrors in its non-generic registry). The factory that disposes the resolver holds it weakly
//! (`Rc::downgrade`). The disposable test services are registered with
//! `register_lazy_singleton_disposable[_with_contract]`, the forms whose values the resolver
//! disposes. `Register<IViewModelOne, ViewModelOne>()` is a factory of a `ViewModelOne` coerced to
//! `Rc<dyn IViewModelOne>`; the `typeof(ViewModelTwo)` registration that only marks the resolver
//! as used is the generic one. A `null` configuration action has no form (`new()` is the
//! resolver without one). A thrown exception is a panic (`panics`). The `Type`-based
//! (non-generic) tests and the `Type`-based assertions of the after-dispose tests are not ported.

use super::super::base_dependency_resolver_tests::{DisposableTestService, ThrowingDisposableService};
use crate::tests::common::{panics, IViewModelOne, ViewModelOne, ViewModelTwo};
use ferroui_splat::{GlobalGenericFirstDependencyResolver, MutableDependencyResolverExt, ReadonlyDependencyResolverExt};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The contract used when probing for a service that was never registered under that contract.
const MISSING_CONTRACT: &str = "missing";

/// The number of times the change callback is expected to fire for a single registration.
const EXPECTED_CALLBACK_INVOCATIONS: usize = 1;

/// The contract used by the registered-then-resolved tests.
const CONTRACT: &str = "contract";

super::super::base_dependency_resolver_tests::base_dependency_resolver_tests!({
    GlobalGenericFirstDependencyResolver::clear();
    Rc::new(GlobalGenericFirstDependencyResolver::new())
});

fn view_model_one() -> Option<Rc<ViewModelOne>> {
    Some(Rc::new(ViewModelOne))
}

fn view_model_one_as_interface() -> Option<Rc<dyn IViewModelOne>> {
    Some(Rc::new(ViewModelOne) as Rc<dyn IViewModelOne>)
}

#[test]
fn get_service_generic_with_contract_when_never_registered_returns_default() {
    GlobalGenericFirstDependencyResolver::clear();
    let resolver = GlobalGenericFirstDependencyResolver::new();

    assert!(resolver.get_service_with_contract::<dyn IViewModelOne>(Some(CONTRACT)).is_none());
    resolver.dispose();
}

#[test]
fn get_service_generic_with_contract_when_lazy_factory_disposes_resolver_disposes_value_and_throws() {
    GlobalGenericFirstDependencyResolver::clear();
    let resolver = Rc::new(GlobalGenericFirstDependencyResolver::new());
    let service = Rc::new(DisposableTestService::new());

    let weak = Rc::downgrade(&resolver);
    let registered = service.clone();
    resolver.register_lazy_singleton_disposable_with_contract::<DisposableTestService>(
        move || {
            if let Some(resolver) = weak.upgrade() {
                resolver.dispose();
            }
            Some(registered.clone())
        },
        Some(CONTRACT),
    );

    assert!(panics(|| {
        let _ = resolver.get_service_with_contract::<DisposableTestService>(Some(CONTRACT));
    }));
    assert!(service.is_disposed());

    GlobalGenericFirstDependencyResolver::clear();
}

#[test]
fn get_service_generic_when_only_unrelated_type_registered_returns_default() {
    let resolver = GlobalGenericFirstDependencyResolver::new();

    // A registration for a different type marks the process as registered, so the generic
    // GetService evaluates its lookup for the unregistered type rather than the never-registered
    // fast path.
    resolver.register::<ViewModelTwo>(|| Some(Rc::new(ViewModelTwo)));

    let result = resolver.get_service::<dyn IViewModelOne>();

    assert!(result.is_none());
    resolver.dispose();
    GlobalGenericFirstDependencyResolver::clear();
}

#[test]
fn get_service_generic_with_contract_when_no_contract_registration_returns_default() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.register::<ViewModelTwo>(|| Some(Rc::new(ViewModelTwo)));

    let result = resolver.get_service_with_contract::<dyn IViewModelOne>(Some(MISSING_CONTRACT));

    assert!(result.is_none());
    resolver.dispose();
    GlobalGenericFirstDependencyResolver::clear();
}

#[test]
fn register_when_registration_callback_throws_does_not_propagate() {
    let resolver = GlobalGenericFirstDependencyResolver::new();

    let invocations = Rc::new(Cell::new(0));

    // Subscribe before any registration exists so the callback is not invoked during subscription.
    let counter = invocations.clone();
    let subscription = resolver.service_registration_callback::<dyn IViewModelOne>(move |_| {
        counter.set(counter.get() + 1);
        panic!("callback failure");
    });

    // The throwing callback fires during the registration notification; the panic must be suppressed.
    assert!(!panics(|| resolver.register::<dyn IViewModelOne>(view_model_one_as_interface)));

    assert_eq!(invocations.get(), EXPECTED_CALLBACK_INVOCATIONS);
    assert!(resolver.has_registration::<dyn IViewModelOne>());

    subscription.dispose();
    resolver.dispose();
    GlobalGenericFirstDependencyResolver::clear();
}

#[test]
fn dispose_when_lazy_singleton_disposal_throws_does_not_propagate() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.register_lazy_singleton_disposable::<ThrowingDisposableService>(|| Some(Rc::new(ThrowingDisposableService)));

    // Force creation so the lazy value exists and is disposed during teardown.
    let service = resolver.get_service::<ThrowingDisposableService>();
    assert!(service.is_some());

    assert!(!panics(|| resolver.dispose()));

    GlobalGenericFirstDependencyResolver::clear();
}

#[test]
fn constructor_with_configure_registers_services() {
    let resolver = GlobalGenericFirstDependencyResolver::with_configuration(|r| {
        r.register::<dyn IViewModelOne>(view_model_one_as_interface);
        r.register_constant(Rc::new(ViewModelOne));
    });

    let result1 = resolver.get_service::<dyn IViewModelOne>();
    let result2 = resolver.get_service::<ViewModelOne>();

    assert!(result1.is_some());
    assert!(result2.is_some());

    resolver.dispose();
    GlobalGenericFirstDependencyResolver::clear();
}

#[test]
fn clear_removes_all_registrations() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.register::<dyn IViewModelOne>(view_model_one_as_interface);
    resolver.register::<ViewModelOne>(view_model_one);

    assert!(resolver.has_registration::<dyn IViewModelOne>());
    assert!(resolver.has_registration::<ViewModelOne>());

    GlobalGenericFirstDependencyResolver::clear();

    assert!(!resolver.has_registration::<dyn IViewModelOne>());
    assert!(!resolver.has_registration::<ViewModelOne>());
    resolver.dispose();
}

#[test]
fn dispose_disposes_resolver() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);

    resolver.dispose();

    assert!(panics(|| resolver.register::<ViewModelOne>(view_model_one)));
}

#[test]
fn after_dispose_operations_throw_object_disposed_exception() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.dispose();

    assert!(panics(|| resolver.register::<ViewModelOne>(view_model_one)));

    assert!(panics(|| resolver.register_with_contract::<ViewModelOne>(view_model_one, Some("contract"))));

    // `Register<IViewModelOne, ViewModelOne>()`.
    assert!(panics(|| resolver.register::<dyn IViewModelOne>(view_model_one_as_interface)));
}

#[test]
fn after_dispose_unregister_current_throws_object_disposed_exception() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.dispose();

    assert!(panics(|| resolver.unregister_current::<ViewModelOne>()));
}

#[test]
fn after_dispose_unregister_all_throws_object_disposed_exception() {
    let resolver = GlobalGenericFirstDependencyResolver::new();
    resolver.dispose();

    assert!(panics(|| resolver.unregister_all::<ViewModelOne>()));
}

// Not ported: GetService_NonGeneric_WhenNeverRegistered_ReturnsNull - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_WhenNeverRegistered_ReturnsEmpty - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_WithContract_ReturnsRegisteredServices - the `Type`-based (non-generic) API, which is not ported
// Not ported: Register_ServiceAndImplementation_ResolvableViaGenericAndNonGeneric - `Register<TService, TImplementation>` and the `Type`-based (non-generic) API, which are not ported
// Not ported: Register_ServiceAndImplementationWithContract_ResolvableViaNonGeneric - the `Type`-based (non-generic) API, which is not ported
// Not ported: RegisterConstant_ResolvableViaNonGeneric - the `Type`-based (non-generic) API, which is not ported
// Not ported: RegisterConstantWithContract_ResolvableViaNonGeneric - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGeneric_WhenLazyFactoryDisposesResolver_DisposesValueAndThrows - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGenericWithContract_WhenLazyFactoryDisposesResolver_DisposesValueAndThrows - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_WhenAllFactoriesReturnNull_ReturnsEmpty - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_Generic_FallsBackToNonGenericRegistration - the fallback to the non-generic registry, which is not ported
// Not ported: GetServices_Generic_WhenFallbackYieldsNull_ReturnsEmpty - the fallback to the non-generic registry, which is not ported
// Not ported: GetServices_Generic_IgnoresNonCastableFallbackRegistrations - the fallback to the non-generic registry, which is not ported
// Not ported: ServiceRegistrationCallback_NonGeneric_WithExistingRegistrations_InvokesImmediately - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_UnwrapsNullServiceTypeMarkers - the null service type (NullServiceType), which is not ported
// Not ported: Constructor_WithNullConfigure_DoesNotThrow - argument-null validation (a null configuration action), which Rust's types make impossible
