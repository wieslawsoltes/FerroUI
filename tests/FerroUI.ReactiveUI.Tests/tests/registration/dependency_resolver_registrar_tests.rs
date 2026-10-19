//! `DependencyResolverRegistrarTests` (`ReactiveUI.Tests/Registration/DependencyResolverRegistrarTests.cs`).
//!
//! The mock resolver records the erased registrations of Splat's resolvers. Upstream compares the
//! recorded factory with the one it registered (a delegate); a factory here is wrapped on its way,
//! so the tests call the recorded factory and check that it creates the service.

use ferroui_reactiveui::*;
use ferroui_splat::service_location::{unerase_service, ErasedService, LazyDisposal, RegistrationCallback, ServiceFactory, ServiceType};
use ferroui_splat::IMutableDependencyResolver;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::Rc;

/// `TestService`.
struct TestService;

/// A resolver that records the registrations (`MockDependencyResolver`).
#[derive(Default)]
struct MockDependencyResolver {
    /// `RegisterConstantCalls`.
    register_constant_calls: RefCell<Vec<(ErasedService, Option<String>)>>,
    /// `RegisterLazySingletonCalls`.
    register_lazy_singleton_calls: RefCell<Vec<(ServiceFactory, Option<String>)>>,
    /// `RegisterCalls`.
    register_calls: RefCell<Vec<(ServiceFactory, Option<String>)>>,
}

struct NoOpDisposable;

impl IDisposable for NoOpDisposable {
    fn dispose(&self) {}
}

impl IMutableDependencyResolver for MockDependencyResolver {
    fn has_registration_erased(&self, _service_type: ServiceType, _contract: Option<&str>) -> bool {
        false
    }

    fn register_erased(&self, factory: ServiceFactory, _service_type: ServiceType, contract: Option<&str>) {
        self.register_calls.borrow_mut().push((factory, contract.map(str::to_owned)));
    }

    fn register_constant_erased(
        &self,
        value: Option<ErasedService>,
        _disposal: Option<Rc<dyn Fn()>>,
        _service_type: ServiceType,
        contract: Option<&str>,
    ) {
        let Some(value) = value else { return };
        self.register_constant_calls.borrow_mut().push((value, contract.map(str::to_owned)));
    }

    fn register_lazy_singleton_erased(
        &self,
        factory: ServiceFactory,
        _disposal: Option<LazyDisposal>,
        _service_type: ServiceType,
        contract: Option<&str>,
    ) {
        self.register_lazy_singleton_calls.borrow_mut().push((factory, contract.map(str::to_owned)));
    }

    fn unregister_current_erased(&self, _service_type: ServiceType, _contract: Option<&str>) {
        // The mock does not track the removals; the tests check the registrations.
    }

    fn unregister_all_erased(&self, _service_type: ServiceType, _contract: Option<&str>) {
        // The mock does not track the removals; the tests check the registrations.
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

fn registrar() -> (Rc<MockDependencyResolver>, DependencyResolverRegistrar) {
    let resolver = Rc::new(MockDependencyResolver::default());
    let registrar = DependencyResolverRegistrar::new(resolver.clone());
    (resolver, registrar)
}

/// Whether `factory` creates a [`TestService`].
fn creates_test_service(factory: &ServiceFactory) -> bool {
    factory().is_some_and(|service| unerase_service::<TestService>(&service).is_some())
}

// Not ported: Constructor_NullResolver_ThrowsArgumentNullException - argument validation

#[test]
fn constructor_valid_resolver_succeeds() {
    let (_resolver, registrar) = registrar();

    assert_eq!(format!("{registrar:?}"), "DependencyResolverRegistrar");
}

#[test]
fn register_constant_without_contract_calls_resolver_register_constant() {
    let (resolver, registrar) = registrar();
    let service = Rc::new(TestService);

    let registered = service.clone();
    registrar.register_constant(move || registered);

    let calls = resolver.register_constant_calls.borrow();
    assert_eq!(calls.len(), 1);
    let recorded = unerase_service::<TestService>(&calls[0].0).unwrap();
    assert!(Rc::ptr_eq(&recorded, &service));
    assert_eq!(calls[0].1, None);
}

#[test]
fn register_constant_with_contract_calls_resolver_register_constant_with_contract() {
    let (resolver, registrar) = registrar();
    let service = Rc::new(TestService);
    const CONTRACT: &str = "test-contract";

    let registered = service.clone();
    registrar.register_constant_with_contract(move || registered, Some(CONTRACT));

    let calls = resolver.register_constant_calls.borrow();
    assert_eq!(calls.len(), 1);
    let recorded = unerase_service::<TestService>(&calls[0].0).unwrap();
    assert!(Rc::ptr_eq(&recorded, &service));
    assert_eq!(calls[0].1.as_deref(), Some(CONTRACT));
}

// Not ported: RegisterConstant_NullFactory_ThrowsArgumentNullException - argument validation

#[test]
fn register_lazy_singleton_without_contract_calls_resolver_register_lazy_singleton() {
    let (resolver, registrar) = registrar();
    let factory = || Rc::new(TestService);

    registrar.register_lazy_singleton(factory);

    let calls = resolver.register_lazy_singleton_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(creates_test_service(&calls[0].0));
    assert_eq!(calls[0].1, None);
}

#[test]
fn register_lazy_singleton_with_contract_calls_resolver_register_lazy_singleton_with_contract() {
    let (resolver, registrar) = registrar();
    let factory = || Rc::new(TestService);
    const CONTRACT: &str = "test-contract";

    registrar.register_lazy_singleton_with_contract(factory, Some(CONTRACT));

    let calls = resolver.register_lazy_singleton_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(creates_test_service(&calls[0].0));
    assert_eq!(calls[0].1.as_deref(), Some(CONTRACT));
}

// Not ported: RegisterLazySingleton_NullFactory_ThrowsArgumentNullException - argument validation

#[test]
fn register_without_contract_calls_resolver_register() {
    let (resolver, registrar) = registrar();
    let factory = || Rc::new(TestService);

    registrar.register(factory);

    let calls = resolver.register_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(creates_test_service(&calls[0].0));
    assert_eq!(calls[0].1, None);
}

#[test]
fn register_with_contract_calls_resolver_register_with_contract() {
    let (resolver, registrar) = registrar();
    let factory = || Rc::new(TestService);
    const CONTRACT: &str = "test-contract";

    registrar.register_with_contract(factory, Some(CONTRACT));

    let calls = resolver.register_calls.borrow();
    assert_eq!(calls.len(), 1);
    assert!(creates_test_service(&calls[0].0));
    assert_eq!(calls[0].1.as_deref(), Some(CONTRACT));
}

// Not ported: Register_NullFactory_ThrowsArgumentNullException - argument validation
