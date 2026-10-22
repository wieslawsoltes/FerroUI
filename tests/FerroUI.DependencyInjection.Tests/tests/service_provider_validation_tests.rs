//! ServiceProviderValidationTests.cs and CircularDependencyTests.cs.
use super::*;
struct Outer;
fn needs_service(lifetime: ServiceLifetime) -> ServiceDescriptor {
    ServiceDescriptor::from_factory(
        lifetime,
        service_factory!(Outer, |s: required(dyn Service)| {
            let _ = s;
            Rc::new(Outer)
        }),
    )
}
#[test]
fn build_service_provider_validate_on_build_throws_for_unresolvable_services() {
    let mut s = ServiceCollection::new();
    s.add(needs_service(ServiceLifetime::Transient));
    assert!(
        matches!(s.build_service_provider(options()), Err(ServiceError::Validation(v)) if matches!(v.as_slice(), [ServiceError::MissingService(_)]))
    );
}
#[test]
fn build_service_provider_validate_on_build_validates_all_descriptors() {
    let mut s = ServiceCollection::new();
    s.add(needs_service(ServiceLifetime::Transient));
    s.add(needs_service(ServiceLifetime::Singleton));
    s.add_singleton::<Outer, _>(|_| Ok(Rc::new(Outer)));
    assert!(
        matches!(s.build_service_provider(options()), Err(ServiceError::Validation(v)) if v.len()==2)
    );
}
#[test]
fn get_service_throws_when_scoped_is_injected_into_singleton() {
    for reverse in [false, true] {
        for build in [false, true] {
            let mut s = ServiceCollection::new();
            let a = descriptor(ServiceLifetime::Scoped, 1);
            let b = needs_service(ServiceLifetime::Singleton);
            if reverse {
                s.add(b).add(a);
            } else {
                s.add(a).add(b);
            }
            let result = s.build_service_provider(ServiceProviderOptions {
                validate_on_build: build,
                validate_scopes: true,
            });
            if build {
                assert!(
                    matches!(result, Err(ServiceError::Validation(v)) if matches!(v.as_slice(), [ServiceError::ScopedInSingleton(_)]))
                );
            } else {
                let p = result.unwrap();
                let scope = p.create_scope().unwrap();
                assert!(matches!(
                    scope.service_provider().get_required_service::<Outer>(),
                    Err(ServiceError::ScopedInSingleton(_))
                ));
            }
        }
    }
}
#[test]
fn get_service_throws_when_scoped_is_injected_into_singleton_through_transient() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1))
        .add(needs_service(ServiceLifetime::Transient));
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Singleton,
        service_factory!(usize, |o: required(Outer)| {
            let _ = o;
            Rc::new(1)
        }),
    ));
    assert!(matches!(
        s.build_service_provider(options()),
        Err(ServiceError::Validation(_))
    ));
}
#[test]
fn get_service_throws_when_get_service_for_scoped_service_is_called_on_root_via_transient() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1))
        .add(needs_service(ServiceLifetime::Transient));
    let p = s.build_service_provider(options()).unwrap();
    assert!(matches!(
        p.get_required_service::<Outer>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
    assert!(p
        .create_scope()
        .unwrap()
        .service_provider()
        .get_required_service::<Outer>()
        .is_ok());
}
#[test]
fn validation_checks_all_enumerable_dependencies_but_only_last_single_dependency() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1))
        .add(descriptor(ServiceLifetime::Singleton, 2));
    s.add(needs_service(ServiceLifetime::Singleton));
    assert!(s.build_service_provider(options()).is_ok());
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Singleton,
        service_factory!(usize, |all: all(dyn Service)| Rc::new(all.len())),
    ));
    assert!(s.build_service_provider(options()).is_err());
}
#[test]
fn get_service_does_not_throw_when_scope_factory_is_injected_into_singleton() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1));
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Singleton,
        service_factory!(usize, |f: required(dyn IServiceScopeFactory)| {
            let scope = f.create_scope().unwrap();
            let value = scope
                .service_provider()
                .get_required_service::<dyn Service>()
                .unwrap()
                .value();
            scope.dispose().unwrap();
            Rc::new(value)
        }),
    ));
    assert_eq!(
        *s.build_service_provider(options())
            .unwrap()
            .get_required_service::<usize>()
            .unwrap(),
        1
    );
}
#[test]
fn factories_are_opaque_at_build_but_validated_at_runtime() {
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1));
    s.add_singleton::<Outer, _>(move |p| {
        c.set(c.get() + 1);
        p.get_required_service::<dyn Service>()?;
        Ok(Rc::new(Outer))
    });
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(count.get(), 0);
    assert!(matches!(
        p.create_scope()
            .unwrap()
            .service_provider()
            .get_required_service::<Outer>(),
        Err(ServiceError::ScopedFromRoot(_))
    ));
    assert_eq!(count.get(), 1);
}
#[test]
fn circular_dependencies_report_a_path_without_recursing_forever() {
    let mut s = ServiceCollection::new();
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Transient,
        service_factory!(Outer, |v: required(usize)| {
            let _ = v;
            Rc::new(Outer)
        }),
    ));
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Transient,
        service_factory!(usize, |v: required(Outer)| {
            let _ = v;
            Rc::new(1)
        }),
    ));
    assert!(
        matches!(s.build_service_provider(options()),Err(ServiceError::Validation(v)) if matches!(&v[0],ServiceError::CircularDependency(path) if path.len()==3))
    );
}
#[test]
fn runtime_factory_cycles_fail_and_unwind() {
    let mut s = ServiceCollection::new();
    s.add_transient::<usize, _>(|p| p.get_required_service::<usize>());
    let p = s.build_service_provider(options()).unwrap();
    for _ in 0..2 {
        assert!(matches!(
            p.get_service::<usize>(),
            Err(ServiceError::CircularDependency(_))
        ));
    }
}
#[test]
fn validation_can_be_disabled_and_root_scoped_services_are_cached() {
    let mut s = ServiceCollection::new();
    s.add(descriptor(ServiceLifetime::Scoped, 1))
        .add(needs_service(ServiceLifetime::Singleton));
    let p = s
        .build_service_provider(ServiceProviderOptions {
            validate_scopes: false,
            validate_on_build: true,
        })
        .unwrap();
    assert!(p.get_service::<Outer>().is_ok());
    assert!(Rc::ptr_eq(
        &p.get_required_service::<dyn Service>().unwrap(),
        &p.get_required_service::<dyn Service>().unwrap()
    ));
}
#[test]
fn same_dependency_in_two_constructor_parameters_is_not_a_cycle() {
    let mut s = ServiceCollection::new();
    s.add_singleton_instance(Rc::new(3usize));
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Transient,
        service_factory!(String, |a: required(usize), b: required(usize)| Rc::new(
            format!("{}", *a + *b)
        )),
    ));
    let p = s.build_service_provider(options()).unwrap();
    assert_eq!(&*p.get_required_service::<String>().unwrap(), "6");
}
#[test]
fn self_dependency_in_enumerable_is_a_cycle() {
    let mut s = ServiceCollection::new();
    s.add(ServiceDescriptor::from_factory(
        ServiceLifetime::Transient,
        service_factory!(usize, |values: all(usize)| Rc::new(values.len())),
    ));
    assert!(
        matches!(s.build_service_provider(options()),Err(ServiceError::Validation(v)) if matches!(&v[0],ServiceError::CircularDependency(_)))
    );
}
