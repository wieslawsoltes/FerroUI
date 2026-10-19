//! Ported from `Splat.Tests/ServiceLocation/ModernDependencyResolverTests.cs` of Splat.
//! Adapted: the inherited tests of `BaseDependencyResolverTests<ModernDependencyResolver>` are the
//! instantiation of `base_dependency_resolver_tests!` below (none is overridden). The factories
//! and callbacks that use the resolver hold it weakly (`Rc::downgrade`), so that the resolver does
//! not keep itself alive. `DisposableTestService` is registered with
//! `register_lazy_singleton_disposable`, the form whose value the resolver disposes. A thrown
//! exception is a panic (`panics`). The `Type`-based (non-generic) tests are not ported.

use super::base_dependency_resolver_tests::DisposableTestService;
use super::nested_resolution_probe::NestedResolutionProbe;
use crate::tests::common::{panics, same, IViewModelOne, ViewModelOne, ViewModelTwo};
use ferroui_splat::{ModernDependencyResolver, MutableDependencyResolverExt, ReadonlyDependencyResolverExt};
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::Rc;

/// The number of existing registrations expected before the callback runs.
const EXISTING_REGISTRATION_COUNT: usize = 3;

/// The number of registrations expected when a duplicate is registered.
const DUPLICATED_REGISTRATION_COUNT: usize = 2;

/// The number of registrations made before subscribing a callback.
const NON_GENERIC_EXISTING_COUNT: usize = 2;

/// The number of times a one-shot callback is expected to fire.
const SINGLE_INVOCATION: usize = 1;

super::base_dependency_resolver_tests::base_dependency_resolver_tests!(Rc::new(ModernDependencyResolver::new()));

fn view_model_one() -> Option<Rc<ViewModelOne>> {
    Some(Rc::new(ViewModelOne))
}

#[test]
fn service_registration_callback_generic_only_fires_for_matching_service_type() {
    let resolver = Rc::new(ModernDependencyResolver::new());
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

    resolver.register_constant(Rc::new(ViewModelTwo));
    assert_eq!(total_invocations.get(), 0);

    resolver.register_constant(Rc::new(ViewModelOne));
    assert_eq!(total_invocations.get(), 1);
    assert_eq!(matching_invocations.get(), 1);

    subscription.dispose();
}

#[test]
fn service_registration_callback_generic_does_not_fire_on_unregister() {
    let resolver = ModernDependencyResolver::new();
    resolver.register_constant(Rc::new(ViewModelOne));

    let invocations = Rc::new(Cell::new(0));
    let counter = invocations.clone();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

    let baseline = invocations.get();

    resolver.unregister_current::<ViewModelOne>();
    resolver.unregister_all::<ViewModelOne>();

    assert_eq!(invocations.get(), baseline);
    subscription.dispose();
}

#[test]
fn get_service_generic_when_missing_during_nested_resolution_resolves_after_later_registration() {
    let resolver = Rc::new(ModernDependencyResolver::new());
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
fn service_registration_callback_invoked_once_per_existing_registration() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.register::<ViewModelOne>(view_model_one);

    let callback_count = Rc::new(Cell::new(0));

    let counter = callback_count.clone();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

    assert_eq!(callback_count.get(), EXISTING_REGISTRATION_COUNT); // Called once for each existing registration
    subscription.dispose();
}

#[test]
fn service_registration_callback_after_disposal_returns_empty_disposable() {
    let resolver = ModernDependencyResolver::new();
    resolver.dispose();

    let callback_invoked = Rc::new(Cell::new(false));

    let invoked = callback_invoked.clone();
    assert!(!panics(|| {
        let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| invoked.set(true));
        subscription.dispose();
    }));

    assert!(!callback_invoked.get());
}

#[test]
fn dispose_disposes_resolver() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);

    assert!(!panics(|| resolver.dispose()));
}

#[test]
fn duplicate_creates_resolver_with_same_registrations() {
    let resolver = ModernDependencyResolver::new();
    let instance = Rc::new(ViewModelOne);
    resolver.register_constant(instance.clone());
    resolver.register::<dyn IViewModelOne>(|| Some(Rc::new(ViewModelOne) as Rc<dyn IViewModelOne>));

    let duplicate = resolver.duplicate();

    let result1 = duplicate.get_service::<ViewModelOne>();
    let result2 = duplicate.get_service::<dyn IViewModelOne>();

    assert!(result1.is_some_and(|result| same(&result, &instance)));
    assert!(result2.is_some());
}

#[test]
fn duplicate_of_disposed_resolver_returns_empty_resolver() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.dispose();

    let duplicate = resolver.duplicate();

    assert!(!duplicate.has_registration::<ViewModelOne>());
}

#[test]
fn duplicate_creates_independent_resolver() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);

    let duplicate = resolver.duplicate();

    duplicate.register::<ViewModelOne>(view_model_one);

    let original_services = resolver.get_services::<ViewModelOne>();
    let duplicate_services = duplicate.get_services::<ViewModelOne>();

    assert_eq!(original_services.len(), 1);
    assert_eq!(duplicate_services.len(), DUPLICATED_REGISTRATION_COUNT);
}

#[test]
fn after_dispose_get_service_throws_object_disposed_exception() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.dispose();

    assert!(panics(|| {
        let _ = resolver.get_service::<ViewModelOne>();
    }));
}

#[test]
fn after_dispose_has_registration_returns_false() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.dispose();

    let result = resolver.has_registration::<ViewModelOne>();

    assert!(!result);
}

#[test]
fn after_dispose_get_services_generic_throws_object_disposed_exception() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.dispose();

    assert!(panics(|| {
        let _ = resolver.get_services::<ViewModelOne>();
    }));
}

#[test]
fn get_service_generic_after_unregister_all_returns_default() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.unregister_all::<ViewModelOne>();

    assert!(resolver.get_service::<ViewModelOne>().is_none());
    resolver.dispose();
}

#[test]
fn service_registration_callback_generic_with_existing_registrations_invokes_immediately() {
    let resolver = ModernDependencyResolver::new();
    resolver.register::<ViewModelOne>(view_model_one);
    resolver.register::<ViewModelOne>(view_model_one);

    let callback_count = Rc::new(Cell::new(0));
    let counter = callback_count.clone();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |_| counter.set(counter.get() + 1));

    assert_eq!(callback_count.get(), NON_GENERIC_EXISTING_COUNT);
    subscription.dispose();
    resolver.dispose();
}

#[test]
fn get_service_when_lazy_factory_disposes_resolver_disposes_value_and_throws() {
    let resolver = Rc::new(ModernDependencyResolver::new());
    let service = Rc::new(DisposableTestService::new());

    let weak = Rc::downgrade(&resolver);
    let registered = service.clone();
    resolver.register_lazy_singleton_disposable::<DisposableTestService>(move || {
        if let Some(resolver) = weak.upgrade() {
            resolver.dispose();
        }
        Some(registered.clone())
    });

    assert!(panics(|| {
        let _ = resolver.get_service::<DisposableTestService>();
    }));

    assert!(service.is_disposed());
}

#[test]
fn register_when_callback_disposes_token_removes_callback_after_firing() {
    let resolver = ModernDependencyResolver::new();
    let invocations = Rc::new(Cell::new(0));

    let counter = invocations.clone();
    let subscription = resolver.service_registration_callback::<ViewModelOne>(move |disposable| {
        counter.set(counter.get() + 1);
        disposable.dispose();
    });

    resolver.register::<ViewModelOne>(view_model_one);
    resolver.register::<ViewModelOne>(view_model_one);

    // The callback disposed its token on the first registration, so it is removed and does not fire again.
    assert_eq!(invocations.get(), SINGLE_INVOCATION);
    subscription.dispose();
    resolver.dispose();
}

// Not ported: ServiceRegistrationCallback_NullServiceType_Throws - argument-null validation of a null service type (the `Type`-based API), which Rust's types make impossible
// Not ported: AfterDispose_HasRegistration_NonGeneric_ReturnsFalse - the `Type`-based (non-generic) API, which is not ported
// Not ported: AfterDispose_GetService_NonGeneric_ThrowsObjectDisposedException - the `Type`-based (non-generic) API, which is not ported
// Not ported: AfterDispose_GetServices_NonGeneric_ThrowsObjectDisposedException - the `Type`-based (non-generic) API, which is not ported
// Not ported: AfterDispose_ServiceRegistrationCallback_NonGeneric_ReturnsEmptyDisposable - the `Type`-based (non-generic) API, which is not ported
// Not ported: GetService_NonGeneric_AfterUnregisterAll_ReturnsNull - the `Type`-based (non-generic) API, which is not ported
// Not ported: ServiceRegistrationCallback_NonGeneric_WithExistingRegistrations_InvokesImmediately - the `Type`-based (non-generic) API, which is not ported
// Not ported: Dispose_WhenNotDisposingManagedResources_LeavesResolverUsable - subclassing (`protected virtual Dispose(bool)`)
