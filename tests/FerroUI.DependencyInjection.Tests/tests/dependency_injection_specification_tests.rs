//! Ported behavior from DependencyInjectionSpecificationTests.cs; closed Rust types and trait
//! objects stand for reflected implementation types (see dependency-injection.md).
use super::*;

#[test]
fn services_registered_with_implementation_type_return_different_instances_per_resolution_for_transient_services(
) {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Transient, 1));
    let p = s.build_service_provider(options()).unwrap();
    assert!(!Rc::ptr_eq(
        &p.get_required_service::<dyn Service>().unwrap(),
        &p.get_required_service::<dyn Service>().unwrap()
    ));
}
#[test]
fn services_registered_with_implementation_type_return_same_instances_per_resolution_for_singletons(
) {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Singleton, 1));
    let p = s.build_service_provider(options()).unwrap();
    assert!(Rc::ptr_eq(
        &p.get_required_service::<dyn Service>().unwrap(),
        &p.get_required_service::<dyn Service>().unwrap()
    ));
}
#[test]
fn service_instance_can_be_resolved() {
    let value: Rc<dyn Service> = Rc::new(Value(3));
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(value.clone());
    let p = s.build_service_provider(options()).unwrap();
    assert!(Rc::ptr_eq(
        &value,
        &p.get_required_service::<dyn Service>().unwrap()
    ));
}
#[test]
fn last_service_replaces_previous_services() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Singleton, 1))
        .add(descriptor(ServiceLifetime::Singleton, 2));
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(p.get_required_service::<dyn Service>().unwrap().value(), 2);
}
#[test]
fn registration_order_is_preserved_when_services_are_ienumerable_resolved() {
    let mut s = ServiceCollection::new();
    for n in 0..5 {
        s.add(descriptor(ServiceLifetime::Singleton, n));
    }
    let p = s.build_service_provider(options()).unwrap();
    let all = p.get_services::<dyn Service>().unwrap();
    assert_eq!(
        all.iter().map(|x| x.value()).collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert!(Rc::ptr_eq(
        &all[4],
        &p.get_required_service::<dyn Service>().unwrap()
    ));
    for (a, b) in all.iter().zip(p.get_services::<dyn Service>().unwrap()) {
        assert!(Rc::ptr_eq(a, &b));
    }
}
#[test]
fn scoped_service_can_be_resolved() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1));
    let p = s.build_service_provider(options()).unwrap();
    let a = p.create_scope().unwrap();
    let b = p.create_scope().unwrap();
    let value = a
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    assert!(Rc::ptr_eq(
        &value,
        &a.service_provider()
            .get_required_service::<dyn Service>()
            .unwrap()
    ));
    assert!(!Rc::ptr_eq(
        &value,
        &b.service_provider()
            .get_required_service::<dyn Service>()
            .unwrap()
    ));
}
#[test]
fn scopes_are_flat_not_hierarchical() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1));
    let p = s.build_service_provider(options()).unwrap();
    let a = p.create_scope().unwrap();
    let b = a.service_provider().create_scope().unwrap();
    let av = a
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    let bv = b
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    assert!(!Rc::ptr_eq(&av, &bv));
    a.dispose().unwrap();
    assert!(b
        .service_provider()
        .get_required_service::<dyn Service>()
        .is_ok());
}
#[test]
fn singleton_services_come_from_root_provider() {
    let mut s = ServiceCollection::new();
    s.add_scoped::<usize, _>(|_| Ok(Rc::new(7)));
    s.add_singleton::<dyn Service, _>(|p| Ok(Rc::new(Value(*p.get_required_service::<usize>()?))));
    let p = s.build_service_provider(Default::default()).unwrap();
    let a = p.create_scope().unwrap();
    let value = a
        .service_provider()
        .get_required_service::<dyn Service>()
        .unwrap();
    assert!(Rc::ptr_eq(
        &value,
        &p.get_required_service::<dyn Service>().unwrap()
    ));
    a.dispose().unwrap();
    assert_eq!(value.value(), 7);
}
#[test]
fn service_scope_factory_is_singleton() {
    let p = ServiceCollection::new()
        .build_service_provider(options())
        .unwrap();
    let a = p.create_scope().unwrap();
    let root = p
        .get_required_service::<dyn IServiceScopeFactory>()
        .unwrap();
    let scoped = a
        .service_provider()
        .get_required_service::<dyn IServiceScopeFactory>()
        .unwrap();
    assert!(Rc::ptr_eq(&root, &scoped));
    a.dispose().unwrap();
    assert!(root.create_scope().is_ok());
}
#[test]
fn non_singleton_service_with_injected_provider_resolves_scope_provider() {
    let p = ServiceCollection::new()
        .build_service_provider(options())
        .unwrap();
    let a = p.create_scope().unwrap();
    let provider = a
        .service_provider()
        .get_required_service::<dyn IServiceProvider>()
        .unwrap();
    let again = provider
        .get_required_service::<dyn IServiceProvider>()
        .unwrap();
    assert!(Rc::ptr_eq(&provider, &again));
    assert!(!Rc::ptr_eq(
        &provider,
        &p.get_required_service::<dyn IServiceProvider>().unwrap()
    ));
    a.dispose().unwrap();
    assert!(matches!(
        provider.get_service::<usize>(),
        Err(ServiceError::Disposed)
    ));
}
#[test]
fn attempting_to_resolve_nonexistent_service_returns_null() {
    let p = ServiceCollection::new()
        .build_service_provider(options())
        .unwrap();
    assert!(p.get_service::<dyn Service>().unwrap().is_none());
    assert!(p.get_services::<dyn Service>().unwrap().is_empty());
    assert!(matches!(
        p.get_required_service::<dyn Service>(),
        Err(ServiceError::MissingService(_))
    ));
}
#[test]
fn is_service_does_not_invoke_factory() {
    let mut s = ServiceCollection::new();
    s.add_transient::<usize, _>(|_| panic!("must not invoke"));
    let p = s.build_service_provider(options()).unwrap();
    let probe = p
        .get_required_service::<dyn IServiceProviderIsService>()
        .unwrap();
    assert!(probe.is_service(ServiceType::of::<usize>()));
    assert!(!probe.is_service(ServiceType::of::<String>()));
    assert!(probe.is_service(ServiceType::of::<dyn IServiceScopeFactory>()));
}
#[test]
fn building_takes_a_snapshot_and_providers_have_independent_singletons() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Singleton, 1));
    let a = s.build_service_provider(options()).unwrap();
    s.add(descriptor(ServiceLifetime::Singleton, 2));
    let b = s.build_service_provider(options()).unwrap();
    assert_eq!(a.get_required_service::<dyn Service>().unwrap().value(), 1);
    assert_eq!(b.get_required_service::<dyn Service>().unwrap().value(), 2);
    assert!(!Rc::ptr_eq(
        &a.get_required_service::<dyn Service>().unwrap(),
        &b.get_services::<dyn Service>().unwrap()[0]
    ));
}
#[test]
fn factory_failure_and_panic_do_not_poison_resolution() {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let mut s = ServiceCollection::new();
    s.add_singleton::<usize, _>(move |_| {
        let n = c.get();
        c.set(n + 1);
        match n {
            0 => Err(ServiceError::Factory("fail".into())),
            1 => panic!("factory"),
            _ => Ok(Rc::new(7)),
        }
    });
    let p = s.build_service_provider(options()).unwrap();
    assert!(p.get_service::<usize>().is_err());
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| p.get_service::<usize>()))
            .is_err()
    );
    assert_eq!(*p.get_required_service::<usize>().unwrap(), 7);
    assert_eq!(count.get(), 3);
}
#[test]
fn nested_service_graphs_and_enumerations_obey_all_lifetimes() {
    struct Consumer(Rc<dyn Service>);
    for lifetime in [
        ServiceLifetime::Singleton,
        ServiceLifetime::Scoped,
        ServiceLifetime::Transient,
    ] {
        let mut s = ServiceCollection::new();
        s.add(descriptor(lifetime, 1));
        s.add(ServiceDescriptor::from_factory(
            lifetime,
            service_factory!(Consumer, |value: required(dyn Service)| Rc::new(Consumer(
                value
            ))),
        ));
        let p = s.build_service_provider(options()).unwrap();
        let factory = p
            .get_required_service::<dyn IServiceScopeFactory>()
            .unwrap();
        for _ in 0..2 {
            let scope = factory.create_scope().unwrap();
            let sp = scope.service_provider();
            let outer = sp.get_required_service::<Consumer>().unwrap();
            assert_eq!(outer.0.value(), 1);
            let first = sp.get_services::<dyn Service>().unwrap();
            assert_eq!(first.len(), 1);
            let second = sp.get_services::<dyn Service>().unwrap();
            assert_eq!(
                Rc::ptr_eq(&first[0], &second[0]),
                lifetime != ServiceLifetime::Transient
            );
            assert_eq!(
                Rc::ptr_eq(&outer.0, &first[0]),
                lifetime != ServiceLifetime::Transient
            );
            scope.dispose().unwrap();
        }
        p.dispose().unwrap();
    }
}
