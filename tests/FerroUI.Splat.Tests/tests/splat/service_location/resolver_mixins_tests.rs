//! Ported from `Splat.Tests/ServiceLocation/ResolverMixinsTests.cs` of Splat.
//! Adapted: the `[Before(Test)]`/`[After(Test)]` scope is an `AppLocatorScope` at the start of each
//! test. The overloads of upstream have names of their own: `RegisterAnd<T>()` is `register_and`,
//! `RegisterAnd<T>(Func<T>)` is `register_and_factory`, `RegisterConstantAnd<T>(T)` is
//! `register_constant_and_value`, `RegisterLazySingletonAnd<T>(Func<T>)` is
//! `register_lazy_singleton_and_factory`, each with a `_with_contract` form. A factory of the
//! service `dyn ITestService` returns the coerced `Rc`. `IsEqualTo(resolver)` is the identity of
//! the returned reference; `IsTypeOf<TestService>` of a service resolved as `TestService` holds by
//! its type, and of one resolved as `dyn ITestService` is its identity with the instance the
//! factory created.

use crate::tests::common::{same, AppLocatorScope};
use ferroui_splat::{AppLocator, IMutableDependencyResolver, ResolverMixins};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The marker service interface of the tests.
trait ITestService {}

/// The implementation of [`ITestService`].
#[derive(Default)]
struct TestService;

impl ITestService for TestService {}

/// `IsEqualTo(resolver)` of the resolver a mixin returns.
fn same_resolver(result: &dyn IMutableDependencyResolver, resolver: &Rc<dyn IMutableDependencyResolver>) -> bool {
    std::ptr::addr_eq(result, Rc::as_ptr(resolver))
}

#[test]
fn register_and_t_service_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let result = resolver.register_and::<TestService>();

    assert!(same_resolver(result, &resolver));
    let service = AppLocator::get_service::<TestService>();
    assert!(service.is_some());
}

#[test]
fn register_and_t_service_with_contract_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let result = resolver.register_and_with_contract::<TestService>("test");

    assert!(same_resolver(result, &resolver));
    let service = AppLocator::get_service_with_contract::<TestService>("test");
    assert!(service.is_some());
}

#[test]
fn register_and_t_service_with_factory_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();
    let instance = Rc::new(TestService);
    let registered = instance.clone();

    let result = resolver.register_and_factory::<dyn ITestService>(move || registered.clone() as Rc<dyn ITestService>);

    assert!(same_resolver(result, &resolver));
    let service = AppLocator::get_service::<dyn ITestService>();
    assert!(service.is_some_and(|service| same(&service, &instance)));
}

#[test]
fn register_and_t_service_with_factory_and_contract_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();
    let instance = Rc::new(TestService);
    let registered = instance.clone();

    let result = resolver.register_and_factory_with_contract::<dyn ITestService>(
        move || registered.clone() as Rc<dyn ITestService>,
        "test",
    );

    assert!(same_resolver(result, &resolver));
    let service = AppLocator::get_service_with_contract::<dyn ITestService>("test");
    assert!(service.is_some_and(|service| same(&service, &instance)));
}

#[test]
fn register_constant_and_t_service_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let result = resolver.register_constant_and::<TestService>();

    assert!(same_resolver(result, &resolver));
    let service1 = AppLocator::get_service::<TestService>().expect("the constant");
    let service2 = AppLocator::get_service::<TestService>().expect("the constant");
    assert!(same(&service2, &service1));
}

#[test]
fn register_constant_and_t_service_with_contract_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let result = resolver.register_constant_and_with_contract::<TestService>("test");

    assert!(same_resolver(result, &resolver));
    let service1 = AppLocator::get_service_with_contract::<TestService>("test").expect("the constant");
    let service2 = AppLocator::get_service_with_contract::<TestService>("test").expect("the constant");
    assert!(same(&service2, &service1));
}

#[test]
fn register_constant_and_t_service_with_value_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();
    let instance = Rc::new(TestService);

    let result = resolver.register_constant_and_value::<dyn ITestService>(instance.clone());

    assert!(same_resolver(result, &resolver));
    let service = AppLocator::get_service::<dyn ITestService>();
    assert!(service.is_some_and(|service| same(&service, &instance)));
}

#[test]
fn register_constant_and_t_service_with_value_and_contract_should_register_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();
    let instance = Rc::new(TestService);

    let result = resolver.register_constant_and_value_with_contract::<dyn ITestService>(instance.clone(), "test");

    assert!(same_resolver(result, &resolver));
    let service = AppLocator::get_service_with_contract::<dyn ITestService>("test");
    assert!(service.is_some_and(|service| same(&service, &instance)));
}

#[test]
fn register_lazy_singleton_and_t_service_should_register_lazily_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let result = resolver.register_lazy_singleton_and::<TestService>();

    assert!(same_resolver(result, &resolver));

    let service1 = AppLocator::get_service::<TestService>().expect("the singleton");
    let service2 = AppLocator::get_service::<TestService>().expect("the singleton");
    assert!(same(&service2, &service1));
}

#[test]
fn register_lazy_singleton_and_t_service_with_contract_should_register_lazily_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let result = resolver.register_lazy_singleton_and_with_contract::<TestService>("test");

    assert!(same_resolver(result, &resolver));

    let service1 = AppLocator::get_service_with_contract::<TestService>("test").expect("the singleton");
    let service2 = AppLocator::get_service_with_contract::<TestService>("test").expect("the singleton");
    assert!(same(&service2, &service1));
}

#[test]
fn register_lazy_singleton_and_t_service_with_factory_should_register_lazily_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();
    let call_count = Rc::new(Cell::new(0));
    let created: Rc<RefCell<Option<Rc<TestService>>>> = Rc::new(RefCell::new(None));
    let count = call_count.clone();
    let created_by_factory = created.clone();

    let result = resolver.register_lazy_singleton_and_factory::<dyn ITestService>(move || {
        count.set(count.get() + 1);
        let instance = Rc::new(TestService);
        *created_by_factory.borrow_mut() = Some(instance.clone());
        instance as Rc<dyn ITestService>
    });

    assert!(same_resolver(result, &resolver));
    assert_eq!(call_count.get(), 0);

    let service1 = AppLocator::get_service::<dyn ITestService>().expect("the singleton");
    assert_eq!(call_count.get(), 1);
    assert!(created.borrow().as_ref().is_some_and(|instance| same(&service1, instance)));

    let service2 = AppLocator::get_service::<dyn ITestService>().expect("the singleton");
    assert_eq!(call_count.get(), 1);
    assert!(same(&service2, &service1));
}

#[test]
fn register_lazy_singleton_and_t_service_with_factory_and_contract_should_register_lazily_and_return_resolver() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();
    let call_count = Rc::new(Cell::new(0));
    let created: Rc<RefCell<Option<Rc<TestService>>>> = Rc::new(RefCell::new(None));
    let count = call_count.clone();
    let created_by_factory = created.clone();

    let result = resolver.register_lazy_singleton_and_factory_with_contract::<dyn ITestService>(
        move || {
            count.set(count.get() + 1);
            let instance = Rc::new(TestService);
            *created_by_factory.borrow_mut() = Some(instance.clone());
            instance as Rc<dyn ITestService>
        },
        "test",
    );

    assert!(same_resolver(result, &resolver));
    assert_eq!(call_count.get(), 0);

    let service = AppLocator::get_service_with_contract::<dyn ITestService>("test").expect("the singleton");
    assert_eq!(call_count.get(), 1);
    assert!(created.borrow().as_ref().is_some_and(|instance| same(&service, instance)));
}

#[test]
fn register_and_supports_fluent_chaining() {
    let _scope = AppLocatorScope::new();
    let resolver = AppLocator::current_mutable();

    let _ = resolver
        .register_and::<TestService>()
        .register_constant_and_value::<dyn ITestService>(Rc::new(TestService))
        .register_lazy_singleton_and_factory_with_contract::<dyn ITestService>(
            || Rc::new(TestService) as Rc<dyn ITestService>,
            "lazy",
        );

    let service1 = AppLocator::get_service::<TestService>();
    let service2 = AppLocator::get_service::<dyn ITestService>();
    let service3 = AppLocator::get_service_with_contract::<dyn ITestService>("lazy");

    assert!(service1.is_some());
    assert!(service2.is_some());
    assert!(service3.is_some());
}

// Not ported: RegisterAnd_TServiceTImplementation_ShouldRegisterAndReturnResolver - `RegisterAnd<TService, TImplementation>` is not ported (generic unsizing); the factory form is RegisterAnd_TService_WithFactory
// Not ported: RegisterAnd_TServiceTImplementation_WithContract_ShouldRegisterAndReturnResolver - `RegisterAnd<TService, TImplementation>` is not ported (generic unsizing)
// Not ported: RegisterAnd_TServiceTImplementation_WithFactory_ShouldRegisterAndReturnResolver - `RegisterAnd<TService, TImplementation>(Func<TImplementation>)` is not ported; it is RegisterAnd_TService_WithFactory here
// Not ported: RegisterAnd_TServiceTImplementation_WithFactoryAndContract_ShouldRegisterAndReturnResolver - `RegisterAnd<TService, TImplementation>(Func<TImplementation>, string)` is not ported; it is RegisterAnd_TService_WithFactoryAndContract here
// Not ported: RegisterConstantAnd_TypeBased_ShouldRegisterAndReturnResolver - the `Type`-based `RegisterConstantAnd(object, Type)`, which is not ported
// Not ported: RegisterConstantAnd_TypeBased_WithContract_ShouldRegisterAndReturnResolver - the `Type`-based `RegisterConstantAnd(object, Type, string)`, which is not ported
// Not ported: RegisterLazySingletonAnd_TypeBased_ShouldRegisterLazilyAndReturnResolver - the `Type`-based `RegisterLazySingletonAnd(Func<object>, Type)`, which is not ported
// Not ported: RegisterLazySingletonAnd_TypeBased_WithContract_ShouldRegisterLazilyAndReturnResolver - the `Type`-based `RegisterLazySingletonAnd(Func<object>, Type, string)`, which is not ported
