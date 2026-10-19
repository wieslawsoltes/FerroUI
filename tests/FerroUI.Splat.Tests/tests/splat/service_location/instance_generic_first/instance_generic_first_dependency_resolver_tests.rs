//! Ported from `Splat.Tests/ServiceLocation/InstanceGenericFirst/InstanceGenericFirstDependencyResolverTests.cs` of Splat.
//! Adapted: the inherited tests of `BaseDependencyResolverTests<InstanceGenericFirstDependencyResolver>`
//! are the instantiation of `base_dependency_resolver_tests!` below (none is overridden). The
//! callbacks of this resolver run for the registrations of their service type and contract, and
//! once per existing registration on subscription (upstream twice for a generic registration,
//! which it mirrors in its non-generic registry). The factories and callbacks that use the
//! resolver hold it weakly (`Rc::downgrade`). The disposable test services are registered with
//! `register_lazy_singleton_disposable[_with_contract]`, the forms whose values the resolver
//! disposes. `Register<IViewModelOne, ViewModelOne>()` is a factory of a `ViewModelOne` coerced to
//! `Rc<dyn IViewModelOne>`. A `null` configuration action has no form (`new()` is the resolver
//! without one). A thrown exception is a panic (`panics`). The `Type`-based (non-generic) tests
//! and the `Type`-based assertion of the after-dispose test are not ported.

use super::super::base_dependency_resolver_tests::{DisposableTestService, ThrowingDisposableService};
use super::super::nested_resolution_probe::NestedResolutionProbe;
use crate::tests::common::{panics, same, IViewModelOne, ViewModelOne, ViewModelTwo};
use ferroui_splat::{
    InstanceGenericFirstDependencyResolver, MutableDependencyResolverExt, ReadonlyDependencyResolverExt,
};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The number of times the change callback is expected to fire for a single registration.
const EXPECTED_CALLBACK_INVOCATIONS: usize = 1;

/// The contract used by the contract-scoped tests.
const CONTRACT: &str = "contract";

super::super::base_dependency_resolver_tests::base_dependency_resolver_tests!(Rc::new(
    InstanceGenericFirstDependencyResolver::new()
));

fn view_model_one() -> Option<Rc<ViewModelOne>> {
    Some(Rc::new(ViewModelOne))
}

fn view_model_one_as_interface() -> Option<Rc<dyn IViewModelOne>> {
    Some(Rc::new(ViewModelOne) as Rc<dyn IViewModelOne>)
}

#[test]
fn get_service_generic_with_contract_when_no_registrations_returns_default() {
    let resolver = InstanceGenericFirstDependencyResolver::new();

    assert!(resolver.get_service_with_contract::<dyn IViewModelOne>(Some(CONTRACT)).is_none());
    resolver.dispose();
}

#[test]
fn get_service_generic_with_contract_when_lazy_factory_disposes_resolver_disposes_value_and_throws() {
    let resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());
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
}

#[test]
fn callback_subscription_disposed_after_resolver_disposed_does_not_throw() {
    let resolver = InstanceGenericFirstDependencyResolver::new();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(|_| {});

    resolver.dispose();

    assert!(!panics(|| subscription.dispose()));
}

#[test]
fn register_when_registration_callback_throws_does_not_propagate() {
    let resolver = InstanceGenericFirstDependencyResolver::new();

    let invocations = Rc::new(Cell::new(0));

    // Subscribe before any matching registration so the callback is not invoked during subscription.
    let counter = invocations.clone();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| {
        counter.set(counter.get() + 1);
        panic!("callback failure");
    });

    // The throwing callback fires during the registration notification; the panic must be suppressed.
    assert!(!panics(|| resolver.register_constant(Rc::new(ViewModelOne))));

    assert_eq!(invocations.get(), EXPECTED_CALLBACK_INVOCATIONS);
    assert!(resolver.has_registration::<ViewModelOne>());

    subscription.dispose();
    resolver.dispose();
}

#[test]
fn dispose_when_lazy_singleton_disposal_throws_does_not_propagate() {
    let resolver = InstanceGenericFirstDependencyResolver::new();
    resolver.register_lazy_singleton_disposable::<ThrowingDisposableService>(|| Some(Rc::new(ThrowingDisposableService)));

    // Force creation so the lazy value exists and is disposed during teardown.
    let service = resolver.get_service::<ThrowingDisposableService>();
    assert!(service.is_some());

    assert!(!panics(|| resolver.dispose()));
}

#[test]
fn service_registration_callback_generic_only_fires_for_matching_service_type() {
    let resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());
    let total_invocations = Rc::new(Cell::new(0));
    let matching_invocations = Rc::new(Cell::new(0));

    let weak = Rc::downgrade(&resolver);
    let (total, matching) = (total_invocations.clone(), matching_invocations.clone());
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| {
        total.set(total.get() + 1);
        if weak.upgrade().is_some_and(|resolver| resolver.has_registration::<ViewModelOne>()) {
            matching.set(matching.get() + 1);
        }
    });

    // An unrelated registration must not trigger the ViewModelOne callback.
    resolver.register_constant(Rc::new(ViewModelTwo));
    assert_eq!(total_invocations.get(), 0);

    // The matching registration must trigger it exactly once, while the service is resolvable.
    resolver.register_constant(Rc::new(ViewModelOne));
    assert_eq!(total_invocations.get(), 1);
    assert_eq!(matching_invocations.get(), 1);

    subscription.dispose();
}

#[test]
fn service_registration_callback_generic_does_not_fire_on_unregister() {
    let resolver = InstanceGenericFirstDependencyResolver::new();
    resolver.register_constant(Rc::new(ViewModelOne));

    let invocations = Rc::new(Cell::new(0));
    let counter = invocations.clone();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

    // The single existing registration is replayed once on subscribe; nothing else should fire.
    let baseline = invocations.get();

    resolver.unregister_current::<ViewModelOne>();
    resolver.unregister_all::<ViewModelOne>();

    assert_eq!(invocations.get(), baseline);
    subscription.dispose();
}

#[test]
fn get_service_generic_when_missing_during_nested_resolution_resolves_after_later_registration() {
    let resolver = Rc::new(InstanceGenericFirstDependencyResolver::new());
    let weak = Rc::downgrade(&resolver);
    resolver.register_lazy_singleton(move || {
        weak.upgrade().map(|resolver| Rc::new(NestedResolutionProbe::<ViewModelOne>::new(&*resolver)))
    });

    let probe = resolver.get_service::<NestedResolutionProbe<ViewModelOne>>();

    assert!(probe.is_some());
    assert!(probe.is_some_and(|probe| probe.service().is_none()));

    let service = Rc::new(ViewModelOne);
    resolver.register_constant(service.clone());

    assert!(resolver.has_registration::<ViewModelOne>());
    assert!(resolver.get_service::<ViewModelOne>().is_some_and(|resolved| same(&resolved, &service)));
}

#[test]
fn constructor_with_configure_registers_services() {
    let resolver = InstanceGenericFirstDependencyResolver::with_configuration(|r| {
        r.register::<dyn IViewModelOne>(view_model_one_as_interface);
        r.register_constant(Rc::new(ViewModelOne));
    });

    let result1 = resolver.get_service::<dyn IViewModelOne>();
    let result2 = resolver.get_service::<ViewModelOne>();

    assert!(result1.is_some());
    assert!(result2.is_some());
    resolver.dispose();
}

#[test]
fn dispose_disposes_resolver() {
    let resolver = InstanceGenericFirstDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);

    resolver.dispose();

    assert!(panics(|| resolver.register::<ViewModelOne>(view_model_one)));
}

#[test]
fn after_dispose_operations_throw_object_disposed_exception() {
    let resolver = InstanceGenericFirstDependencyResolver::new();
    resolver.dispose();

    assert!(panics(|| resolver.register::<ViewModelOne>(view_model_one)));

    assert!(panics(|| resolver.register_with_contract::<ViewModelOne>(view_model_one, Some("contract"))));

    // `Register<IViewModelOne, ViewModelOne>()`.
    assert!(panics(|| resolver.register::<dyn IViewModelOne>(view_model_one_as_interface)));
}

// Not ported: GetServices_NonGeneric_WhenNoRegistrations_ReturnsEmpty - the `Type`-based (non-generic) API, which is not ported
// Not ported: Register_ServiceAndImplementation_ResolvableViaNonGeneric - the `Type`-based (non-generic) API, which is not ported
// Not ported: Register_ServiceAndImplementationWithContract_ResolvableViaNonGeneric - the `Type`-based (non-generic) API, which is not ported
// Not ported: RegisterConstantWithContract_ResolvableViaNonGeneric - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGeneric_WhenLazyFactoryDisposesResolver_DisposesValueAndThrows - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGenericWithContract_WhenLazyFactoryDisposesResolver_DisposesValueAndThrows - the `Type`-based (non-generic) API, which is not ported
// Not ported: ServiceRegistrationCallback_NonGeneric_WithExistingRegistrations_InvokesImmediately - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetServices_NonGeneric_WhenMarkerUnwrapsToNull_ReturnsEmpty - the null service type (NullServiceType), which is not ported
// Not ported: GetServices_Generic_WhenFallbackYieldsNull_ReturnsEmpty - the fallback to the non-generic registry, which is not ported
// Not ported: GetServices_Generic_IgnoresNonCastableFallbackRegistrations - the fallback to the non-generic registry, which is not ported
// Not ported: GetServices_NonGeneric_UnwrapsNullServiceTypeMarkers - the null service type (NullServiceType), which is not ported
// Not ported: Constructor_WithNullConfigure_DoesNotThrow - argument-null validation (a null configuration action), which Rust's types make impossible
